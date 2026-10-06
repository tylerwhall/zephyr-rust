#[cfg(not(mutex_pool))]
use alloc::boxed::Box;
use core::ops::Deref;
use core::ptr::NonNull;

use crate::mutex::*;

/// Number of preallocated mutexes, or zero when mutexes are heap allocated.
#[cfg(mutex_pool)]
pub const MUTEX_POOL_SIZE: usize = zephyr_sys::raw::CONFIG_RUST_MUTEX_POOL_SIZE as usize;
#[cfg(not(mutex_pool))]
pub const MUTEX_POOL_SIZE: usize = 0;

pub struct DynMutex(NonNull<KMutex>);

impl DynMutex {
    pub fn new<C: MutexSyscalls>() -> Option<Self> {
        unsafe {
            #[cfg(not(mutex_pool))]
            let m = {
                let m = Box::into_raw(Box::new(**crate::mutex::global::k_mutex::uninit()));
                (*m).init::<C>();
                m
            };
            #[cfg(mutex_pool)]
            let m = mutex_pool::alloc_mutex()?;

            Some(DynMutex(NonNull::new_unchecked(m)))
        }
    }

    pub fn into_raw(self) -> *mut KMutex {
        let ptr = self.0.as_ptr();
        core::mem::forget(self);
        ptr
    }

    /// Reclaim ownership transferred by `into_raw`.
    ///
    /// # Safety
    ///
    /// `m` must be a non-null pointer returned by `DynMutex::into_raw` using
    /// this image's allocation backend. Ownership must not already have been
    /// reclaimed or freed. No locks or borrowed references may remain when
    /// the reconstructed owner is dropped.
    pub unsafe fn from_raw(m: *mut KMutex) -> Self {
        DynMutex(NonNull::new_unchecked(m))
    }
}

impl Drop for DynMutex {
    fn drop(&mut self) {
        unsafe {
            #[cfg(not(mutex_pool))]
            Box::from_raw(self.0.as_mut());
            #[cfg(mutex_pool)]
            mutex_pool::free_mutex(self.0.as_mut());
        }
    }
}

impl Deref for DynMutex {
    type Target = KMutex;

    fn deref(&self) -> &KMutex {
        unsafe { self.0.as_ref() }
    }
}

#[cfg(mutex_pool)]
mod mutex_pool {
    use crate::mutex::*;
    use core::sync::atomic::{AtomicU8, Ordering};

    const NUM_MUTEX: usize = zephyr_sys::raw::CONFIG_RUST_MUTEX_POOL_SIZE as usize;

    extern "C" {
        #[allow(improper_ctypes)]
        static rust_mutex_pool: [KMutex; NUM_MUTEX];
    }

    const NUM_USED: usize = (NUM_MUTEX + 7) / 8;
    extern "C" {
        // C owns one zero-initialized byte array in rust_std_partition.
        // Every crate instance accesses it exclusively through AtomicU8.
        static rust_mutex_pool_used: [AtomicU8; NUM_USED];
    }

    pub fn alloc_mutex() -> Option<*mut KMutex> {
        let mut ret = None;

        for (i, byte) in unsafe { &rust_mutex_pool_used }.iter().enumerate() {
            // Valid bits in this byte
            let valid = core::cmp::min(NUM_MUTEX - i * 8, 8);
            if byte
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |val| {
                    ret = None;
                    for bit in 0..valid {
                        let mask = 1 << bit;
                        if val & mask == 0 {
                            unsafe {
                                ret = Some(&rust_mutex_pool[i * 8 + bit] as *const _ as *mut _)
                            };
                            return Some(val | mask);
                        }
                    }
                    None
                })
                .is_ok()
            {
                break;
            }
        }

        ret
    }

    pub fn free_mutex(mutex: *mut KMutex) {
        let index = unsafe { mutex.offset_from(&rust_mutex_pool[0] as *const _) } as usize;
        let byte = index / 8;
        let bit = index % 8;
        unsafe { rust_mutex_pool_used[byte].fetch_and(!(1 << bit), Ordering::Relaxed) };
    }
}
