use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

use zephyr_sys::raw::{k_mutex, k_objects};

use super::NegErr;
use crate::kobj::*;

// Declare the Zephyr struct to be a kernel object
unsafe impl KObj for k_mutex {
    const OTYPE: k_objects = zephyr_sys::raw::k_objects_K_OBJ_MUTEX;
}

pub use zephyr_sys::raw::k_mutex as KMutex;

crate::make_static_wrapper!(k_mutex, zephyr_sys::raw::k_mutex);

/// Raw syscall API
pub trait MutexSyscalls {
    /// Initialize mutex storage.
    ///
    /// # Safety
    ///
    /// `mutex` must point to aligned, writable storage for a Zephyr mutex.
    /// No other thread may access it during initialization. The caller must
    /// satisfy the syscall context's privilege and object-access requirements.
    unsafe fn k_mutex_init(mutex: *mut zephyr_sys::raw::k_mutex);
    /// Lock an initialized mutex.
    ///
    /// # Safety
    ///
    /// `mutex` must point to a live, initialized mutex for the duration of the
    /// call, accessible in this syscall context. This must be called from a
    /// thread, not an ISR; `timeout` must be valid for the built Zephyr image.
    unsafe fn k_mutex_lock(
        mutex: *mut zephyr_sys::raw::k_mutex,
        timeout: zephyr_sys::raw::k_timeout_t,
    ) -> libc::c_int;
    /// Unlock a mutex owned by the current thread.
    ///
    /// # Safety
    ///
    /// `mutex` must point to a live, initialized mutex accessible in this
    /// syscall context. The current thread must own the lock, and unlocking
    /// must not invalidate outstanding references to protected data.
    unsafe fn k_mutex_unlock(mutex: *mut zephyr_sys::raw::k_mutex);
}

/// Safer API implemented for the mutex kobject.
///
/// Still not safe because it doesn't implement a lock guard.
pub trait RawMutex {
    /// Initialize the mutex before use.
    ///
    /// # Safety
    ///
    /// The storage must be valid and no other thread may access it during
    /// initialization. The caller must be permitted to initialize it in `C`.
    unsafe fn init<C: MutexSyscalls>(self);
    /// Lock the mutex indefinitely.
    ///
    /// # Safety
    ///
    /// The mutex must be live, initialized, and accessible in `C`. The caller
    /// must be a thread and must maintain the locking protocol for its data.
    unsafe fn lock<C: MutexSyscalls>(self);
    /// Release one level of ownership of the mutex.
    ///
    /// # Safety
    ///
    /// The mutex must be live, initialized, accessible in `C`, and owned by
    /// this thread. No protected reference may outlive the required lock.
    unsafe fn unlock<C: MutexSyscalls>(self);
    /// Try to lock the mutex without waiting.
    ///
    /// # Safety
    ///
    /// The same requirements as `lock` apply. Access protected data only if
    /// this returns true, and release the acquired lock when finished.
    unsafe fn try_lock<C: MutexSyscalls>(self) -> bool;
}

impl RawMutex for &KMutex {
    #[inline]
    unsafe fn init<C: MutexSyscalls>(self) {
        C::k_mutex_init(self as *const _ as *mut _)
    }

    unsafe fn lock<C: MutexSyscalls>(self) {
        C::k_mutex_lock(self as *const _ as *mut _, zephyr_sys::raw::K_FOREVER)
            .neg_err()
            .expect("mutex lock");
    }

    unsafe fn unlock<C: MutexSyscalls>(self) {
        C::k_mutex_unlock(self as *const _ as *mut _);
    }

    unsafe fn try_lock<C: MutexSyscalls>(self) -> bool {
        match C::k_mutex_lock(self as *const _ as *mut _, zephyr_sys::raw::K_NO_WAIT).neg_err() {
            Ok(_) => Ok(true),
            Err(zephyr_sys::raw::EBUSY) => Ok(false),
            Err(e) => Err(e),
        }
        .expect("mutex try_lock")
    }
}

/// Safe mutex container like that in std
///
/// Using this is safe, but creating it is not. Creator must ensure it is not
/// possible to get a reference to the data elsewhere. Lifetime bounds ensure the
/// mutex kobject lives at least as long as the data it protects.
pub struct Mutex<'m, T> {
    mutex: &'m KMutex,
    data: MutexData<T>,
}

impl<'m, T> Mutex<'m, T> {
    /// Associate data with a kernel mutex.
    ///
    /// # Safety
    ///
    /// `mutex` must remain initialized and usable for `'m`. All access to the
    /// protected data, including data reachable through references in `T`,
    /// must use this same mutex. External code must not reinitialize or unlock
    /// the mutex while this wrapper or any of its guards is in use.
    pub const unsafe fn new(mutex: &'m KMutex, data: T) -> Self {
        Mutex {
            mutex,
            data: MutexData::new(data),
        }
    }

    /// Expose the underlying kernel object pointer.
    ///
    /// # Safety
    ///
    /// The pointer must not be used beyond the kernel mutex's lifetime or to
    /// bypass the locking protocol, reinitialize the mutex, or invalidate a
    /// guard. Calls through it must satisfy the selected context's privileges.
    pub unsafe fn kobj(&self) -> *mut libc::c_void {
        self.mutex as *const _ as *mut _
    }

    pub fn lock<C: MutexSyscalls>(&self) -> MutexGuard<'_, T, C> {
        unsafe {
            self.mutex.lock::<C>();
        }
        MutexGuard {
            mutex: self,
            _syscalls: PhantomData,
        }
    }
}

/// Allow cloning a mutex where the data is a reference. This allows multiple references to static
/// data with a static lock without wrapping those references in another Arc layer.
impl<T> Clone for Mutex<'_, &T> {
    fn clone(&self) -> Self {
        Mutex {
            mutex: self.mutex,
            data: unsafe { MutexData::new(*self.data.0.get()) },
        }
    }
}

pub struct MutexGuard<'a, T: 'a, C: MutexSyscalls> {
    mutex: &'a Mutex<'a, T>,
    _syscalls: PhantomData<C>,
}

impl<'a, T: 'a, C: MutexSyscalls> Drop for MutexGuard<'a, T, C> {
    fn drop(&mut self) {
        unsafe { self.mutex.mutex.unlock::<C>() }
    }
}

impl<'a, T: 'a, C: MutexSyscalls> Deref for MutexGuard<'a, T, C> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.mutex.data.0.get() }
    }
}

impl<'a, T: 'a, C: MutexSyscalls> DerefMut for MutexGuard<'a, T, C> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.data.0.get() }
    }
}

pub struct MutexData<T>(UnsafeCell<T>);

unsafe impl<T> Sync for MutexData<T> {}

impl<T> MutexData<T> {
    pub const fn new(data: T) -> Self {
        MutexData(UnsafeCell::new(data))
    }
}
