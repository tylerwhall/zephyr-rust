use libc::{c_int, c_void};
use zephyr_sys::raw::{
    _poll_types_bits__POLL_TYPE_SEM_AVAILABLE, _poll_types_bits__POLL_TYPE_SIGNAL, k_poll_event,
    k_poll_modes_K_POLL_MODE_NOTIFY_ONLY, k_timeout_t, K_POLL_STATE_NOT_READY, K_POLL_TYPE_IGNORE,
};

use crate::kobj::*;
use crate::semaphore::KSem;
use crate::time::Timeout;
use crate::NegErr;

pub use crate::poll_signal::*;

pub type KPollEvent = k_poll_event;

pub unsafe trait PollableKobj: KObj {
    const POLL_TYPE: u32;
}

unsafe impl PollableKobj for KSem {
    const POLL_TYPE: u32 = 1 << (_poll_types_bits__POLL_TYPE_SEM_AVAILABLE - 1);
}

unsafe impl PollableKobj for KPollSignal {
    const POLL_TYPE: u32 = 1 << (_poll_types_bits__POLL_TYPE_SIGNAL - 1);
}

#[repr(u32)]
pub enum PollMode {
    NotifyOnly = k_poll_modes_K_POLL_MODE_NOTIFY_ONLY,
}

pub trait PollEventFuncs {
    fn new() -> Self;

    fn init<'e, 'o: 'e, O: PollableKobj>(&'e mut self, kobj: &'o O, mode: PollMode);

    fn ready(&self) -> bool;
}

impl PollEventFuncs for KPollEvent {
    fn new() -> Self {
        unsafe {
            let mut event = core::mem::uninitialized();
            zephyr_sys::raw::k_poll_event_init(
                &mut event,
                K_POLL_TYPE_IGNORE,
                0,
                1 as *const c_void as *mut c_void, // Must not be null, but won't be touched by the kernel because type is IGNORE
            );
            event
        }
    }

    fn init<'e, 'o: 'e, O: PollableKobj>(&'e mut self, kobj: &'o O, mode: PollMode) {
        unsafe {
            zephyr_sys::raw::k_poll_event_init(
                self,
                O::POLL_TYPE,
                mode as c_int,
                kobj.as_void_ptr(),
            )
        }
    }

    fn ready(&self) -> bool {
        self.state() != K_POLL_STATE_NOT_READY
    }
}

pub trait PollSyscalls {
    fn k_poll(events: &mut [KPollEvent], timeout: k_timeout_t) -> c_int;
}

macro_rules! trait_impl {
    ($context:ident, $context_struct:path) => {
        impl PollSyscalls for $context_struct {
            fn k_poll(events: &mut [KPollEvent], timeout: k_timeout_t) -> c_int {
                unsafe {
                    zephyr_sys::syscalls::$context::k_poll(
                        events.as_mut_ptr(),
                        events.len() as c_int,
                        timeout,
                    )
                }
            }
        }
    };
}

trait_impl!(kernel, crate::context::Kernel);
trait_impl!(user, crate::context::User);
trait_impl!(any, crate::context::Any);

#[derive(Clone, Copy, Debug)]
pub enum PollError {
    Canceled,
}

pub trait PollEventsFuncs {
    fn poll<C: PollSyscalls>(&mut self) -> Result<(), PollError>;
    /// Returns true if events are ready, false if timeout.
    fn poll_timeout<C: PollSyscalls>(
        &mut self,
        timeout: Option<Timeout>,
    ) -> Result<bool, PollError>;
}

impl PollEventsFuncs for [KPollEvent] {
    fn poll<C: PollSyscalls>(&mut self) -> Result<(), PollError> {
        match C::k_poll(self, zephyr_sys::raw::K_FOREVER).neg_err() {
            Ok(_) => Ok(()),
            Err(zephyr_sys::raw::EINTR) => Err(PollError::Canceled),
            Err(zephyr_sys::raw::ENOMEM) => panic!("k_poll OOM"),
            Err(e) => panic!("k_poll error {}", e),
        }
    }

    fn poll_timeout<C: PollSyscalls>(
        &mut self,
        timeout: Option<Timeout>,
    ) -> Result<bool, PollError> {
        let timeout = timeout.map(|x| x.0).unwrap_or(zephyr_sys::raw::K_FOREVER);
        match C::k_poll(self, timeout).neg_err() {
            Ok(_) => Ok(true),
            Err(zephyr_sys::raw::EAGAIN) => Ok(false),
            Err(zephyr_sys::raw::EINTR) => Err(PollError::Canceled),
            Err(zephyr_sys::raw::ENOMEM) => panic!("k_poll OOM"),
            Err(e) => panic!("k_poll error {}", e),
        }
    }
}
