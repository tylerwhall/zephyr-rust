extern crate alloc;
extern crate zephyr_core;

use alloc::sync::{Arc, Weak};
use core::cell::{RefCell, UnsafeCell};
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use std::time::Instant;

use futures::future::{Future, LocalFutureObj};
use futures::stream::Stream;
use futures::task::{ArcWake, LocalSpawn, SpawnError};
use futures_util::future::FutureExt;
use log::trace;

use zephyr_core::mutex::*;
use zephyr_core::poll::*;
use zephyr_core::semaphore::*;
use zephyr_core::DurationMs;

pub mod delay;

use delay::{TimerPoll, TimerReactor};

struct Reactor {
    events: Vec<KPollEvent>,
    // Wakers corresponding to each event
    wakers: Vec<Waker>,
    timers: TimerReactor,
}

impl Reactor {
    fn new() -> Self {
        Reactor {
            events: Vec::new(),
            wakers: Vec::new(),
            timers: TimerReactor::new(),
        }
    }

    fn register(&mut self, signal: &'static impl PollableKobj, context: &mut Context) {
        let len = self.events.len();
        unsafe {
            self.events.reserve(1);
            self.events.set_len(len + 1);
            self.events[len].init(signal, PollMode::NotifyOnly);
        }
        self.wakers.push(context.waker().clone());
    }

    fn register_timer(&mut self, deadline: Instant, context: &mut Context) {
        self.timers.register(deadline, context)
    }

    /// Returns true if events was non empty and something is ready. False if
    /// there is nothing to wait on. Fired events are removed.
    fn poll<C: PollSyscalls>(&mut self, timeout: Option<DurationMs>) -> bool {
        if self.events.is_empty() && timeout.is_none() {
            return false;
        }
        self.events[..].poll_timeout::<C>(timeout).unwrap();

        assert_eq!(self.events.len(), self.wakers.len());
        let mut i = 0;
        while i < self.events.len() {
            if self.events[i].ready() {
                self.wakers[i].wake_by_ref();
                trace!("Ready {}", i);
                // Remove current element and replace with last. Continue search
                // at current position.
                self.events.swap_remove(i);
                self.wakers.swap_remove(i);
            } else {
                i += 1;
            }
        }
        true
    }
}

thread_local! {
    static REACTOR: RefCell<Option<Reactor>> = RefCell::new(None);
}

#[inline(never)]
pub fn current_reactor_register(signal: &'static impl PollableKobj, context: &mut Context) {
    match REACTOR.try_with(|r| r.borrow_mut().as_mut().map(|r| r.register(signal, context))) {
        Ok(None) | Err(_) => panic!("register with no reactor"),
        Ok(Some(_)) => (),
    }
}

#[inline(never)]
pub fn current_reactor_register_timer(deadline: Instant, context: &mut Context) {
    match REACTOR.try_with(|r| {
        r.borrow_mut()
            .as_mut()
            .map(|r| r.register_timer(deadline, context))
    }) {
        Ok(None) | Err(_) => panic!("register with no reactor"),
        Ok(Some(_)) => (),
    }
}

struct Task {
    future: UnsafeCell<LocalFutureObj<'static, ()>>,
    executor: ExecutorHandle,
}

// The future is not required to be thread safe, but it is only used from the unsafe poll function.
// Holding an Arc reference and only using the safe interface to wake the task is thread safe
// because it doesn't access the future. We guarantee single thread access to the future because a
// task is only created and owned by one executor and the executor is not send or sync.
unsafe impl Send for Task {}
unsafe impl Sync for Task {}

impl Task {
    fn new(future: LocalFutureObj<'static, ()>, executor: ExecutorHandle) -> Self {
        Task {
            future: UnsafeCell::new(future),
            executor,
        }
    }

    /// Unsafe because this mutates the future with no locking.
    /// We use multiple Arc references to tasks for the run queue and wakers, but
    /// only the single executor should access the future contained within, so it
    /// is safe for it to be the sole writer.
    unsafe fn poll(&self, context: &mut Context) -> Poll<()> {
        let pin_mut = &mut *self.future.get();
        pin_mut.poll_unpin(context)
    }
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.executor.push_runnable(arc_self.clone());
    }
}

#[derive(Default)]
struct ExecutorInner {
    runnable: Vec<Arc<Task>>,
}

impl ExecutorInner {
    fn push_runnable(&mut self, task: Arc<Task>) {
        if !self.runnable.iter().any(|other| Arc::ptr_eq(other, &task)) {
            self.runnable.push(task);
        }
    }

    fn pop_runnable(&mut self) -> Option<Arc<Task>> {
        self.runnable.pop()
    }
}

// Because we've marked Tasks as Send + Sync so we can use Arc references to wake them, we could
// get an auto impl of Send. But the thread safety of Task depends on the true owner of the task
// that calls poll being not Send or Sync. Since we're not requiring spawned futures to be Send or
// Sync and Executor is the effective owner, add a PhantomData here as if we directly own a Future
// that is not explicitly Send or Sync.
pub struct Executor(
    Arc<Mutex<'static, ExecutorInner>>,
    PhantomData<dyn Future<Output = ()>>,
);
#[derive(Clone)]
pub struct ExecutorHandle(Weak<Mutex<'static, ExecutorInner>>);

impl Executor {
    /// Unsafe because the client guarantees the static mutex is intended for
    /// this purpose.
    pub unsafe fn new(mutex: &'static KMutex) -> Self {
        Executor(Arc::new(Mutex::new(mutex, Default::default())), PhantomData)
    }

    fn pop_runnable<C: MutexSyscalls>(&self) -> Option<Arc<Task>> {
        self.0.lock::<C>().pop_runnable()
    }

    fn push_runnable<C: MutexSyscalls>(&self, task: Arc<Task>) {
        self.0.lock::<C>().push_runnable(task);
    }

    pub fn spawner(&self) -> ExecutorHandle {
        ExecutorHandle(Arc::downgrade(&self.0))
    }

    pub fn run<C: MutexSyscalls + PollSyscalls>(&mut self) {
        let reactor = Reactor::new();
        //let waker = task.clone().into_waker();
        //let mut context = Context::from_waker(&waker);

        REACTOR.with(move |r| {
            r.replace(Some(reactor));

            loop {
                while let Some(task) = self.pop_runnable::<C>() {
                    let waker = futures::task::waker_ref(&task);
                    let mut context = Context::from_waker(&*waker);
                    // Don't care about the result of poll. If the future is
                    // not complete, it will likely either have been
                    // registered with the reactor for I/O, or somewhere
                    // there's a live reference to the waker. If not,
                    // there's no way this could ever be marked runnable in
                    // the future, so we always drop our reference we took
                    // from the ready queue.
                    // The future is fused, so if the task is woken after it
                    // completes here, it will get added to the ready queue
                    // and harmlessly polled once more.
                    let _ = unsafe { task.poll(&mut context) };
                }

                let mut reactor_borrow = r.borrow_mut();
                let reactor = reactor_borrow.as_mut().unwrap();
                let timeout = match reactor.timers.poll() {
                    TimerPoll::Idle => None,
                    TimerPoll::Delay(timeout) => Some(timeout),
                    TimerPoll::Woken => continue,
                };
                trace!("Reactor poll wait. Timeout {:?}", timeout);
                if !reactor.poll::<C>(timeout) {
                    break;
                }
                trace!("Reactor poll finished");
            }
            r.replace(None);
        })
    }
}

impl LocalSpawn for Executor {
    fn spawn_local_obj(&mut self, future: LocalFutureObj<'static, ()>) -> Result<(), SpawnError> {
        let task = Arc::new(Task::new(future, self.spawner()));
        self.push_runnable::<zephyr::context::Any>(task);
        Ok(())
    }
}

impl ExecutorHandle {
    fn push_runnable(&self, task: Arc<Task>) {
        // Do nothing if our weak reference is invalid
        if let Some(executor) = self.0.upgrade() {
            executor.lock::<zephyr::context::Any>().push_runnable(task);
        }
    }
}

impl LocalSpawn for ExecutorHandle {
    fn spawn_local_obj(&mut self, future: LocalFutureObj<'static, ()>) -> Result<(), SpawnError> {
        if let Some(executor) = self.0.upgrade() {
            let task = Arc::new(Task::new(future, self.clone()));
            executor.lock::<zephyr::context::Any>().push_runnable(task);
            Ok(())
        } else {
            Err(SpawnError::shutdown())
        }
    }
}

pub struct SemaphoreStream(&'static KSem);

impl SemaphoreStream {
    pub fn new(sem: &'static KSem) -> Self {
        SemaphoreStream(sem)
    }
}

impl Stream for SemaphoreStream {
    type Item = ();

    fn poll_next(self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
        if self.0.try_take::<zephyr::context::Any>() {
            Poll::Ready(Some(()))
        } else {
            REACTOR.with(|r| {
                r.borrow_mut()
                    .as_mut()
                    .expect("polled semaphore outside of reactor context")
                    .register(self.0, context);
            });
            Poll::Pending
        }
    }
}
