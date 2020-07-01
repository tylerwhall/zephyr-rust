use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

use zephyr_sys::raw::k_mutex;

use super::NegErr;
use crate::kobj::*;

// Declare the Zephyr struct to be a kernel object
unsafe impl KObj for k_mutex {}

pub use zephyr_sys::raw::k_mutex as KMutex;

crate::make_static_wrapper!(k_mutex, zephyr_sys::raw::k_mutex);

/// Raw syscall API
pub trait MutexSyscalls {
    unsafe fn k_mutex_init(mutex: *mut zephyr_sys::raw::k_mutex);
    unsafe fn k_mutex_lock(
        mutex: *mut zephyr_sys::raw::k_mutex,
        timeout: zephyr_sys::raw::k_timeout_t,
    ) -> libc::c_int;
    unsafe fn k_mutex_unlock(mutex: *mut zephyr_sys::raw::k_mutex);
}

/// Safer API implemented for the mutex kobject.
///
/// Still not safe because it doesn't implement a lock guard.
pub trait RawMutex {
    unsafe fn init<C: MutexSyscalls>(self);
    unsafe fn lock<C: MutexSyscalls>(self);
    unsafe fn unlock<C: MutexSyscalls>(self);
    unsafe fn try_lock<C: MutexSyscalls>(self) -> bool;
}

impl<'a> RawMutex for &'a KMutex {
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
    pub unsafe fn new(mutex: &'m KMutex, data: T) -> Self {
        Mutex {
            mutex,
            data: MutexData::new(data),
        }
    }

    pub unsafe fn kobj(&self) -> *mut libc::c_void {
        self.mutex as *const _ as *mut _
    }

    pub fn lock<'a, C: MutexSyscalls>(&'a self) -> MutexGuard<'a, T, C> {
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
impl<'m, 'd, T> Clone for Mutex<'m, &'d T> {
    fn clone(&self) -> Self {
        Mutex {
            mutex: self.mutex,
            data: unsafe { MutexData::new(&*self.data.0.get()) },
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
