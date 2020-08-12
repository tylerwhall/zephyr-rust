extern crate alloc;
extern crate zephyr_core;

use alloc::sync::{Arc, Weak};
use core::cell::{RefCell, UnsafeCell};
use core::marker::PhantomData;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll, Waker};
use std::time::Instant;

use futures::future::{Future, FutureExt, LocalFutureObj};
use futures::stream::Stream;
use futures::task::{ArcWake, LocalSpawn, SpawnError};
use log::trace;

use zephyr_core::mutex::*;
use zephyr_core::poll::*;
use zephyr_core::semaphore::*;
use zephyr_core::thread::{ThreadId, ThreadSyscalls};
use zephyr_core::Timeout;

pub mod delay;

use delay::{TimerPoll, TimerReactor};

struct Reactor {
    events: Vec<KPollEvent>,
    // Wakers corresponding to each event after the initial KPollSignal
    wakers: Vec<Waker>,
    timers: TimerReactor,
}

impl Reactor {
    fn new(signal: &'static KPollSignal) -> Self {
        // First event slot is used for the KPollSignal for cross-thread wake
        let mut events = vec![KPollEvent::new()];
        events[0].init(signal, PollMode::NotifyOnly);
        Reactor {
            events,
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

    fn poll<C: PollSyscalls>(&mut self, timeout: Option<Timeout>) {
        self.events[..].poll_timeout::<C>(timeout).unwrap();

        assert_eq!(self.events.len(), self.wakers.len() + 1);
        let mut i = 1;
        while i < self.events.len() {
            if self.events[i].ready() {
                self.wakers[i - 1].wake_by_ref();
                trace!("Rdy {} {}", i, self.events[i].type_());
                // Remove current element and replace with last. Continue search
                // at current position.
                self.events.swap_remove(i);
                self.wakers.swap_remove(i - 1);
            } else {
                i += 1;
            }
        }
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
    runnable: AtomicBool,
    /// Signal for the executor of this task
    thread_signal: &'static KPollSignal,
    /// ThreadId of the executor of this task
    thread: ThreadId,
}

// The future is not required to be thread safe, but it is only used from the unsafe poll function.
// Holding an Arc reference and only using the safe interface to wake the task is thread safe
// because it doesn't access the future. We guarantee single thread access to the future because a
// task is only created and owned by one executor and the executor is not send or sync.
unsafe impl Send for Task {}
unsafe impl Sync for Task {}

impl Task {
    fn new(
        future: LocalFutureObj<'static, ()>,
        thread_signal: &'static KPollSignal,
        thread: ThreadId,
    ) -> Self {
        Task {
            future: UnsafeCell::new(future),
            runnable: AtomicBool::new(true),
            thread_signal,
            thread,
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
        use zephyr::context::Any as C;
        if !arc_self.runnable.swap(true, Ordering::SeqCst) {
            // Wake executor if transitioning to true
            if arc_self.thread != C::k_current_get() {
                arc_self.thread_signal.raise::<C>(0);
            }
        }
    }
}

struct ExecutorInner {
    tasks: Vec<Arc<Task>>,
}

impl ExecutorInner {
    fn new() -> Self {
        ExecutorInner { tasks: Vec::new() }
    }

    /// Poll<Option> has the same semantics as polling a `Stream`
    fn get_runnable(&mut self) -> Poll<Option<Arc<Task>>> {
        if self.tasks.is_empty() {
            return Poll::Ready(None);
        }
        for task in self.tasks.iter_mut() {
            if task.runnable.swap(false, Ordering::SeqCst) {
                return Poll::Ready(Some(task.clone()));
            }
        }
        Poll::Pending
    }

    fn add_task(&mut self, task: Arc<Task>) {
        self.tasks.push(task);
    }

    fn remove_task(&mut self, task: Arc<Task>) {
        self.tasks.retain(|other| !Arc::ptr_eq(other, &task));
    }
}

struct ExecutorState {
    inner: Mutex<'static, ExecutorInner>,
    /// Allows explicit wake from another thread
    thread_signal: &'static KPollSignal,
}

// Because we've marked Tasks as Send + Sync so we can use Arc references to wake them, we could
// get an auto impl of Send. But the thread safety of Task depends on the true owner of the task
// that calls poll being not Send or Sync. Since we're not requiring spawned futures to be Send or
// Sync and Executor is the effective owner, add a PhantomData here as if we directly own a Future
// that is not explicitly Send or Sync.
pub struct Executor {
    state: Arc<ExecutorState>,
    _tasks: PhantomData<dyn Future<Output = ()>>,
}

#[derive(Clone)]
pub struct ExecutorHandle(Weak<ExecutorState>);

impl Executor {
    /// Unsafe because the client guarantees the static mutex is intended for
    /// this purpose.
    pub unsafe fn new(mutex: &'static KMutex, thread_signal: &'static KPollSignal) -> Self {
        Executor {
            state: Arc::new(ExecutorState {
                inner: Mutex::new(mutex, ExecutorInner::new()),
                thread_signal,
            }),
            _tasks: PhantomData,
        }
    }

    pub fn spawner(&self) -> ExecutorHandle {
        ExecutorHandle(Arc::downgrade(&self.state))
    }

    pub fn run<C: MutexSyscalls + KPollSignalSyscalls + PollSyscalls + ThreadSyscalls>(&mut self) {
        let reactor = Reactor::new(self.state.thread_signal);
        let current = C::k_current_get();

        REACTOR.with(move |r| {
            r.replace(Some(reactor));

            'main: loop {
                trace!("Reactor {:?} run", current);
                // Signal indicates need to poll run queue. Reset before poll.
                self.state.thread_signal.reset::<C>();
                loop {
                    match self.state.inner.lock::<C>().get_runnable() {
                        Poll::Ready(Some(task)) => {
                            let waker = futures::task::waker_ref(&task);
                            let mut context = Context::from_waker(&*waker);
                            if let Poll::Ready(()) = unsafe { task.poll(&mut context) } {
                                self.state.inner.lock::<C>().remove_task(task);
                            }
                        }
                        Poll::Pending => break,
                        Poll::Ready(None) => break 'main,
                    }
                }

                let mut reactor_borrow = r.borrow_mut();
                let reactor = reactor_borrow.as_mut().unwrap();
                let timeout = match reactor.timers.poll() {
                    TimerPoll::Idle => None,
                    TimerPoll::Delay(timeout) => Some(timeout),
                    TimerPoll::Woken => continue,
                };
                trace!("Reactor {:?} wait. Timeout {:?}", current, timeout);
                reactor.poll::<C>(timeout);
            }

            r.replace(None);
        })
    }
}

impl LocalSpawn for Executor {
    fn spawn_local_obj(&self, future: LocalFutureObj<'static, ()>) -> Result<(), SpawnError> {
        use zephyr::context::Any as C;
        let task = Arc::new(Task::new(
            future,
            self.state.thread_signal,
            C::k_current_get(),
        ));
        self.state.inner.lock::<C>().add_task(task);
        Ok(())
    }
}

impl LocalSpawn for ExecutorHandle {
    fn spawn_local_obj(&self, future: LocalFutureObj<'static, ()>) -> Result<(), SpawnError> {
        use zephyr::context::Any as C;
        if let Some(state) = self.0.upgrade() {
            let task = Arc::new(Task::new(future, state.thread_signal, C::k_current_get()));
            state.inner.lock::<C>().add_task(task);
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
