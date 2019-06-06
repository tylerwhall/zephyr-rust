#![cfg_attr(not(feature = "have_std"), no_std)]
#![feature(never_type)]

#[macro_use]
extern crate derive_more;

pub mod kobj;
pub mod mutex;
pub mod thread;
mod time;

pub use time::*;

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

    use super::NegErr;

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

    use core::cell::UnsafeCell;
    use core::mem::MaybeUninit;

    pub struct KMutex(UnsafeCell<MaybeUninit<zephyr_sys::raw::k_mutex>>);

    unsafe impl Send for KMutex {}
    unsafe impl Sync for KMutex {}

    impl KMutex {
        pub const fn uninit() -> Self {
            KMutex(UnsafeCell::new(MaybeUninit::uninit()))
        }

        #[inline]
        pub unsafe fn init(&self) {
            zephyr_sys::syscalls::kernel::k_mutex_init((*self.0.get()).as_mut_ptr())
        }

        pub unsafe fn lock(&self) {
            zephyr_sys::syscalls::kernel::k_mutex_lock(
                (*self.0.get()).as_mut_ptr(),
                zephyr_sys::raw::K_FOREVER,
            )
            .neg_err()
            .expect("mutex lock");
        }

        pub unsafe fn unlock(&self) {
            zephyr_sys::syscalls::kernel::k_mutex_unlock((*self.0.get()).as_mut_ptr());
        }

        pub unsafe fn try_lock(&self) -> bool {
            match zephyr_sys::syscalls::kernel::k_mutex_lock(
                (*self.0.get()).as_mut_ptr(),
                zephyr_sys::raw::K_NO_WAIT as i32,
            )
            .neg_err()
            {
                Ok(_) => Ok(true),
                Err(zephyr_sys::raw::EAGAIN) => Ok(false),
                Err(e) => Err(e),
            }
            .expect("mutex try_lock")
        }
    }
}

pub mod user {
    #[cfg(feature = "have_std")]
    use std::ffi::CStr;

    zephyr_bindings!(user, crate::context::User);

    #[cfg(feature = "have_std")]
    pub struct ZephyrDevice(*mut zephyr_sys::raw::device);

    #[cfg(feature = "have_std")]
    #[inline(always)]
    pub fn device_get_binding(device_name: &CStr) -> ZephyrDevice {
        ZephyrDevice(unsafe { zephyr_sys::syscalls::user::device_get_binding(device_name.as_ptr()) })
    }

    #[cfg(feature = "have_std")]
    #[inline(always)]
    pub fn uart_poll_out(device: &ZephyrDevice, out_char: char) {
        unsafe { zephyr_sys::syscalls::user::uart_poll_out(device.0, out_char as u8) };
    }

    #[cfg(feature = "have_std")]
    #[inline(always)]
    pub fn uart_poll_in(device: &ZephyrDevice, in_char: &mut char) -> i32 {
        let mut munge: u8 = 0;
        let rc: i32 = unsafe { zephyr_sys::syscalls::user::uart_poll_in(device.0, &mut munge) };
        *in_char = munge as char;
        rc
    }

    #[cfg(feature = "have_std")]
    #[inline(always)]
    pub fn uart_err_check(device: &ZephyrDevice) -> i32 {
        (unsafe { zephyr_sys::syscalls::user::uart_err_check(device.0) })
    }
}

pub mod any {
    zephyr_bindings!(any, crate::context::Any);
}
