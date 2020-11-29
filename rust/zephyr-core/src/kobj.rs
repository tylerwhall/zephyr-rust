use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::ops::Deref;

use zephyr_sys::raw::k_objects;

/// A kernel object that is usable via system calls.
///
/// This implies locking is done in the kernel and this is therefore Send + Sync. e.g. mutex,
/// semaphore, fifo. Implement this for the raw Zephyr C structs.
pub unsafe trait KObj {
    const OTYPE: k_objects;

    // This is safe, itself, but obviously unsafe to use the mut void pointer
    fn as_void_ptr(&self) -> *mut libc::c_void {
        self as *const _ as *mut _
    }
}

// On behalf of the 'zephyr' crate
unsafe impl KObj for zephyr_sys::raw::device {
    const OTYPE: k_objects = zephyr_sys::raw::k_objects_K_OBJ_ANY;
}

pub struct StaticKObj<T>(UnsafeCell<MaybeUninit<T>>);

unsafe impl<T: KObj> Send for StaticKObj<T> {}
unsafe impl<T: KObj> Sync for StaticKObj<T> {}

impl<T> StaticKObj<T> {
    // Unsafe because there must be some provision to initialize this before it is referenced and
    // it must be declared static because we hand out KObjRef with static lifetime.
    pub const unsafe fn uninit() -> Self {
        StaticKObj(UnsafeCell::new(MaybeUninit::uninit()))
    }
}

impl<T: KObj> StaticKObj<T> {
    pub fn as_ptr(&self) -> *const T {
        unsafe { (*self.0.get()).as_ptr() }
    }

    pub fn as_mut_ptr(&self) -> *mut T {
        unsafe { (*self.0.get()).as_mut_ptr() }
    }
}

impl<T: KObj> Deref for StaticKObj<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.as_ptr() }
    }
}

#[macro_export]
macro_rules! make_static_wrapper {
    ($k_type:ident, $k_path:path) => {
        /// Defines a newtype struct k_foo appropriate for static initialization that
        /// looks to zephyr like its own.
        ///
        /// Creating uninitialized variables in Rust requires a union which is in the
        /// implementation of MaybeUninit. gen_kobject_list.py ignores members of unions
        /// so fails to recognise that our StaticKobj contains a struct k_mutex. By using
        /// the same structure name, we trick gen_kobject_list.py into whitelisting the
        /// address of this struct as a kernel object.
        pub mod global {
            use crate::kobj::*;
            use core::ops::Deref;

            #[allow(non_camel_case_types)]
            pub struct $k_type(StaticKObj<$k_path>);

            unsafe impl KObj for $k_type {
                const OTYPE: zephyr_sys::raw::k_objects = <$k_path as KObj>::OTYPE;
            }

            impl $k_type {
                pub const unsafe fn uninit() -> Self {
                    $k_type(StaticKObj::uninit())
                }

                /// Get the real k_obj type. Same as deref twice.
                pub fn kobj(&self) -> &$k_path {
                    self.deref().deref()
                }
            }

            impl Deref for $k_type {
                type Target = StaticKObj<$k_path>;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }
        }
    };
}
