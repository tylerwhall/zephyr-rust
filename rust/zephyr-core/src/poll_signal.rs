use libc::{c_int, c_uint};

use crate::kobj::*;

pub use zephyr_sys::raw::k_poll_signal as KPollSignal;

unsafe impl KObj for KPollSignal {}

crate::make_static_wrapper!(k_poll_signal, zephyr_sys::raw::k_poll_signal);

pub trait KPollSignalSyscalls {
    unsafe fn k_poll_signal_init(signal: &KPollSignal);
    fn k_poll_signal_reset(signal: &KPollSignal);
    fn k_poll_signal_check(signal: &KPollSignal, signaled: &mut c_uint, result: &mut c_int);
    fn k_poll_signal_raise(signal: &KPollSignal, result: c_int) -> c_int;
}

macro_rules! trait_impl {
    ($context:ident, $context_struct:path) => {
        impl KPollSignalSyscalls for $context_struct {
            unsafe fn k_poll_signal_init(signal: &KPollSignal) {
                zephyr_sys::syscalls::$context::k_poll_signal_init(signal as *const _ as *mut _)
            }

            fn k_poll_signal_reset(signal: &KPollSignal) {
                unsafe {
                    zephyr_sys::syscalls::$context::k_poll_signal_reset(
                        signal as *const _ as *mut _,
                    )
                }
            }

            fn k_poll_signal_check(
                signal: &KPollSignal,
                signaled: &mut c_uint,
                result: &mut c_int,
            ) {
                unsafe {
                    zephyr_sys::syscalls::$context::k_poll_signal_check(
                        signal as *const _ as *mut _,
                        signaled,
                        result,
                    )
                }
            }

            fn k_poll_signal_raise(signal: &KPollSignal, result: c_int) -> c_int {
                unsafe {
                    zephyr_sys::syscalls::$context::k_poll_signal_raise(
                        signal as *const _ as *mut _,
                        result,
                    )
                }
            }
        }
    };
}

trait_impl!(kernel, crate::context::Kernel);
trait_impl!(user, crate::context::User);
trait_impl!(any, crate::context::Any);

pub trait Signal {
    unsafe fn init<C: KPollSignalSyscalls>(&self);
    fn reset<C: KPollSignalSyscalls>(&self);
    fn check<C: KPollSignalSyscalls>(&self) -> Option<c_int>;
    fn raise<C: KPollSignalSyscalls>(&self, result: c_int);
}

impl Signal for KPollSignal {
    unsafe fn init<C: KPollSignalSyscalls>(&self) {
        C::k_poll_signal_init(self)
    }

    fn reset<C: KPollSignalSyscalls>(&self) {
        C::k_poll_signal_reset(self)
    }

    fn check<C: KPollSignalSyscalls>(&self) -> Option<c_int> {
        let mut signaled = 0;
        let mut result = 0;
        C::k_poll_signal_check(self, &mut signaled, &mut result);
        if signaled != 0 {
            Some(result)
        } else {
            None
        }
    }

    fn raise<C: KPollSignalSyscalls>(&self, result: c_int) {
        C::k_poll_signal_raise(self, result);
    }
}
