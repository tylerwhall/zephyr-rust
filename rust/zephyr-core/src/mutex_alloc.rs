#[cfg(not(mutex_pool))]
use alloc::boxed::Box;
use core::ops::Deref;
use core::ptr::NonNull;

use crate::mutex::*;

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
            let m = mutex_pool::alloc_mutex().expect("mutex pool exhausted");

            Some(DynMutex(NonNull::new_unchecked(m)))
        }
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
    /// Bitfield tracking allocated mutexes
    static USED: [AtomicU8; NUM_USED] = unsafe { core::mem::transmute([0u8; NUM_USED]) };

    pub fn alloc_mutex() -> Option<*mut KMutex> {
        let mut ret = None;

        for (i, byte) in USED.iter().enumerate() {
            // Valid bits in this byte
            let valid = core::cmp::max(NUM_MUTEX - i * 8, 8);
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
        USED[byte].fetch_and(!(1 << bit), Ordering::Relaxed);
    }
}
