#![no_std]
#![feature(never_type)]

macro_rules! zephyr_bindings {
    ($context:ident) => {
        #[inline(always)]
        pub fn k_str_out_raw(s: &[u8]) {
            unsafe { zephyr_sys::syscalls::$context::k_str_out(s.as_ptr() as *mut _, s.len()) };
        }

        #[inline(always)]
        pub fn k_str_out(s: &str) {
            k_str_out_raw(s.as_bytes())
        }
    };
}

/// Functions only accessible from kernel mode
pub mod kernel {
    use core::alloc::{GlobalAlloc, Layout};
    use core::ptr;
    use libc::c_void;

    zephyr_bindings!(kernel);

    pub fn k_thread_user_mode_enter<F>(mut f: F) -> !
    where
        F: FnOnce(),
    {
        extern "C" fn run_closure<F>(p1: *mut c_void, _p2: *mut c_void, _p3: *mut c_void)
        where
            F: FnOnce(),
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
    zephyr_bindings!(user);
}

pub mod any {
    zephyr_bindings!(any);
}
