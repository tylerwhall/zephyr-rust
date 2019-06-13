#![cfg_attr(not(feature = "have_std"), no_std)]
#![feature(never_type)]

#[macro_use]
extern crate derive_more;

pub mod device;
pub mod kobj;
pub mod memdomain;
pub mod mempool;
pub mod mutex;
pub mod semaphore;
pub mod thread;
mod time;
pub mod uart;

pub use time::*;

// Set from environment from build.rs
pub const CONFIG_USERSPACE: bool = cfg!(usermode);

/// Convert a negative error code to a Result
trait NegErr {
    fn neg_err(&self) -> Result<u32, u32>;
}

impl NegErr for i32 {
    fn neg_err(&self) -> Result<u32, u32> {
        if *self >= 0 {
            Ok(*self as u32)
        } else {
            Err((-*self) as u32)
        }
    }
}

pub mod context {
    /// Kernel, user, or runtime-detect (any)
    pub unsafe trait Context {}

    pub struct Kernel;
    unsafe impl Context for Kernel {}

    pub struct User;
    unsafe impl Context for User {}

    pub struct Any;
    unsafe impl Context for Any {}
}

macro_rules! zephyr_bindings {
    ($context:ident, $context_struct:path) => {
        #[inline(always)]
        pub fn k_str_out_raw(s: &[u8]) {
            unsafe { zephyr_sys::syscalls::$context::k_str_out(s.as_ptr() as *mut _, s.len()) };
        }

        #[inline(always)]
        pub fn k_str_out(s: &str) {
            k_str_out_raw(s.as_bytes())
        }

        #[inline(always)]
        pub fn k_uptime_get_ms() -> crate::InstantMs {
            unsafe { crate::InstantMs::from(zephyr_sys::syscalls::$context::k_uptime_get()) }
        }

        #[inline(always)]
        pub fn k_sleep(ms: crate::DurationMs) -> crate::DurationMs {
            unsafe { crate::DurationMs::from(zephyr_sys::syscalls::$context::k_sleep(ms.into())) }
        }

        #[inline(always)]
        pub fn k_thread_custom_data_get() -> *mut u8 {
            unsafe { zephyr_sys::syscalls::$context::k_thread_custom_data_get() as *mut u8 }
        }

        #[inline(always)]
        pub fn k_thread_custom_data_set(value: *mut u8) {
            unsafe { zephyr_sys::syscalls::$context::k_thread_custom_data_set(value as *mut _) };
        }

        impl crate::mutex::MutexSyscalls for $context_struct {
            unsafe fn k_mutex_init(mutex: *mut zephyr_sys::raw::k_mutex) {
                zephyr_sys::syscalls::$context::k_mutex_init(mutex)
            }

            unsafe fn k_mutex_lock(
                mutex: *mut zephyr_sys::raw::k_mutex,
                timeout: zephyr_sys::raw::s32_t,
            ) -> libc::c_int {
                zephyr_sys::syscalls::$context::k_mutex_lock(mutex, timeout)
            }

            unsafe fn k_mutex_unlock(mutex: *mut zephyr_sys::raw::k_mutex) {
                zephyr_sys::syscalls::$context::k_mutex_unlock(mutex)
            }
        }
    };
}

/// Functions only accessible from kernel mode
pub mod kernel {
    use core::alloc::{GlobalAlloc, Layout};
    use core::ptr;
    use libc::c_void;

    zephyr_bindings!(kernel, crate::context::Kernel);

    pub fn k_thread_user_mode_enter<F>(mut f: F) -> !
    where
        F: FnOnce() + Send + Sync,
    {
        extern "C" fn run_closure<F>(p1: *mut c_void, _p2: *mut c_void, _p3: *mut c_void)
        where
            F: FnOnce() + Send + Sync,
        {
            let f = unsafe { ptr::read(p1 as *mut F) };
            f();
        }
        unsafe {
            zephyr_sys::raw::k_thread_user_mode_enter(
                Some(run_closure::<F>),
                &mut f as *mut _ as *mut c_void,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        }
        unreachable!()
    }

    pub struct KMalloc;

    unsafe impl GlobalAlloc for KMalloc {
        #[inline]
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            zephyr_sys::raw::k_malloc(layout.size()) as *mut _
        }

        #[inline]
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            zephyr_sys::raw::k_calloc(1, layout.size()) as *mut _
        }

        #[inline]
        unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
            zephyr_sys::raw::k_free(ptr as *mut _)
        }
    }
}

pub mod user {
    zephyr_bindings!(user, crate::context::User);
}

pub mod any {
    zephyr_bindings!(any, crate::context::Any);
}
