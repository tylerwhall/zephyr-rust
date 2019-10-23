use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

use zephyr_core::{InstantMs, DurationMs};

#[derive(Debug)]
pub struct Delay(Instant);

impl Delay {
    pub fn new(dur: Duration) -> Self {
        Self(Instant::now() + dur)
    }

    pub fn new_at(instant: Instant) -> Self {
        Self(instant)
    }
}

impl Future for Delay {
    type Output = ();

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        // Can we avoid a system call on every poll?
        if Instant::now() >= self.0 {
            Poll::Ready(())
        } else {
            super::current_reactor_register_timer(self.0, context);
            Poll::Pending
        }
    }
}

/// Simple registry of tasks and their soonest expiring timer
///
/// This is kept simple by storing the state of outstanding timers only in the `Delay`. Rather than
/// keeping each delay registered with the reactor until it expires, only keep track of the next
/// soonest deadline per task. When any deadline for a task expires, the task is polled, causing
/// any relevant deadlines to be re-registered. It is not necessary for a Delay to unregister
/// itself when it is dropped. A back reference to mark the delay expired is also not used.
///
/// The tradeoff is slower polling. Each delay must compare itself against the system time and
/// re-register on every poll if not expired. The `register` operation searches the list once per
/// timer and the list size is up to the number of tasks. This scales poorly, but the assumption
/// for Zephyr is that there will likely be no more than 5-10 each of tasks and timers.
pub(super) struct TimerReactor {
    tasks: Vec<(Waker, Instant)>,
}

impl TimerReactor {
    pub fn new() -> Self {
        TimerReactor { tasks: Vec::new() }
    }

    pub fn register(&mut self, new_deadline: Instant, context: &mut Context) {
        let new_waker = context.waker();
        if let Some((ref _waker, ref mut cur_deadline)) = self
            .tasks
            .iter_mut()
            .find(|(waker, _instant)| waker.will_wake(new_waker))
        {
            // We already have this task registered. Update the expiration time if it's sooner.
            // Else we can ignore this, because the task will get woken earlier, the delay will get
            // polled again and this deadline will be re-registered.
            if *cur_deadline > new_deadline {
                *cur_deadline = new_deadline;
            }
        } else {
            // Just add the deadline for this task.
            self.tasks.push((new_waker.clone(), new_deadline));
        }
    }

    /// Wake and remove expired timers. Return whether tasks or woken, or else how long to wait.
    pub fn poll(&mut self) -> TimerPoll<DurationMs> {
        if self.tasks.is_empty() {
            return TimerPoll::Idle;
        }
        let mut ret = TimerPoll::<Instant>::Idle;
        let now = Instant::now();

        // Could use drain_filter when stable. https://github.com/rust-lang/rust/issues/43244
        self.tasks.retain(|(waker, deadline)| {
            if now >= *deadline {
                waker.wake_by_ref();
                ret = TimerPoll::Woken;
                false
            } else {
                ret = match ret {
                    TimerPoll::Delay(ref cur) if deadline < cur => TimerPoll::Delay(*deadline),
                    TimerPoll::Delay(cur) => TimerPoll::Delay(cur),
                    TimerPoll::Idle => TimerPoll::Delay(*deadline),
                    TimerPoll::Woken => TimerPoll::Woken,
                };
                true
            }
        });
        ret.map(|deadline| InstantMs::from(deadline).sub_timeout(InstantMs::from(now)))
    }
}

pub(super) enum TimerPoll<T> {
    /// No pending timers
    Idle,
    /// A task was woken due to timer expiration
    Woken,
    /// No work to do now. Contains the soonest expiring timer.
    Delay(T),
}

impl<T> TimerPoll<T> {
    pub fn map<U, F>(self, f: F) -> TimerPoll<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            TimerPoll::Idle => TimerPoll::Idle,
            TimerPoll::Woken => TimerPoll::Woken,
            TimerPoll::Delay(t) => TimerPoll::Delay(f(t)),
        }
    }
}
