use zephyr_sys::raw::k_sem;

use super::NegErr;
use crate::kobj::*;
use crate::time::DurationMs;

// Declare the Zephyr struct to be a kernel object
unsafe impl KObj for k_sem {}

pub use zephyr_sys::raw::k_sem as KSem;

crate::make_static_wrapper!(k_sem, zephyr_sys::raw::k_sem);

/// Raw syscall API
pub trait SemaphoreSyscalls {
    unsafe fn k_sem_init(sem: &k_sem, initial_count: libc::c_uint, limit: libc::c_uint);
    fn k_sem_take(sem: &k_sem, timeout: DurationMs) -> libc::c_int;
    fn k_sem_give(sem: &k_sem);
    fn k_sem_reset(sem: &k_sem);
    fn k_sem_count_get(sem: &k_sem) -> libc::c_uint;
}

macro_rules! trait_impl {
    ($context:ident, $context_struct:path) => {
        impl SemaphoreSyscalls for $context_struct {
            unsafe fn k_sem_init(sem: &k_sem, initial_count: libc::c_uint, limit: libc::c_uint) {
                zephyr_sys::syscalls::$context::k_sem_init(
                    sem as *const _ as *mut _,
                    initial_count,
                    limit,
                ); // TODO: return the error from here. Ignoring now for Zephyr 2.1 compat
            }

            fn k_sem_take(sem: &k_sem, timeout: DurationMs) -> libc::c_int {
                unsafe {
                    zephyr_sys::syscalls::$context::k_sem_take(
                        sem as *const _ as *mut _,
                        timeout.into(),
                    )
                }
            }

            fn k_sem_give(sem: &k_sem) {
                unsafe { zephyr_sys::syscalls::$context::k_sem_give(sem as *const _ as *mut _) }
            }

            fn k_sem_reset(sem: &k_sem) {
                unsafe { zephyr_sys::syscalls::$context::k_sem_reset(sem as *const _ as *mut _) }
            }

            fn k_sem_count_get(sem: &k_sem) -> libc::c_uint {
                unsafe {
                    zephyr_sys::syscalls::$context::k_sem_count_get(sem as *const _ as *mut _)
                }
            }
        }
    };
}

trait_impl!(kernel, crate::context::Kernel);
trait_impl!(user, crate::context::User);
trait_impl!(any, crate::context::Any);

/// Safe API implemented on the sem struct. Converts errors.
pub trait Semaphore {
    unsafe fn init<C: SemaphoreSyscalls>(&self, initial_count: u32, limit: u32);
    /// Take with infinite timeout
    fn take<C: SemaphoreSyscalls>(&self);
    /// Take with timeout. Returns true if successful. False if timeout.
    fn take_timeout<C: SemaphoreSyscalls>(&self, timeout: DurationMs) -> bool;
    /// Take with no timeout. Returns true if successful.
    fn try_take<C: SemaphoreSyscalls>(&self) -> bool;
    fn give<C: SemaphoreSyscalls>(&self);
    fn reset<C: SemaphoreSyscalls>(&self);
    fn count<C: SemaphoreSyscalls>(&self) -> u32;
}

impl Semaphore for k_sem {
    unsafe fn init<C: SemaphoreSyscalls>(&self, initial_count: u32, limit: u32) {
        C::k_sem_init(&self, initial_count, limit)
    }

    fn take<C: SemaphoreSyscalls>(&self) {
        C::k_sem_take(self, zephyr_sys::raw::K_FOREVER.into())
            .neg_err()
            .expect("sem take");
    }

    fn take_timeout<C: SemaphoreSyscalls>(&self, timeout: DurationMs) -> bool {
        match C::k_sem_take(self, timeout).neg_err() {
            Ok(_) => Ok(true),
            Err(zephyr_sys::raw::EBUSY) => Ok(false),
            Err(zephyr_sys::raw::EAGAIN) => Ok(false),
            Err(e) => Err(e),
        }
        .expect("sem take")
    }

    fn try_take<C: SemaphoreSyscalls>(&self) -> bool {
        match C::k_sem_take(self, (zephyr_sys::raw::K_NO_WAIT as i32).into()).neg_err() {
            Ok(_) => Ok(true),
            Err(zephyr_sys::raw::EBUSY) => Ok(false),
            Err(e) => Err(e),
        }
        .expect("sem take")
    }

    fn give<C: SemaphoreSyscalls>(&self) {
        C::k_sem_give(self)
    }

    fn reset<C: SemaphoreSyscalls>(&self) {
        C::k_sem_reset(self)
    }

    fn count<C: SemaphoreSyscalls>(&self) -> u32 {
        // .into() will fail to compile on platforms where uint != u32
        // can do a conversion if that case ever occurs
        C::k_sem_count_get(self).into()
    }
}
