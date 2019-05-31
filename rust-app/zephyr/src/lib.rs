#![no_std]
#![feature(never_type)]

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

    use super::NegErr;

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
    zephyr_bindings!(user);
}

pub mod any {
    zephyr_bindings!(any);
}
