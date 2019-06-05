use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

use zephyr_sys::raw::k_mutex;

use super::NegErr;
use crate::kobj::*;

// Declare the Zephyr struct to be a kernel object
unsafe impl KObj for k_mutex {}

pub use zephyr_sys::raw::k_mutex as KMutex;

/// Defines a newtype struct k_mutex appropriate for static initialization that
/// looks to zephyr like its own.
///
/// Creating uninitialized variables in Rust requires a union which is in the
/// implementation of MaybeUninit. gen_kobject_list.py ignores members of unions
/// so fails to recognise that our StaticKobj contains a struct k_mutex. By using
/// the same structure name, we trick gen_kobject_list.py into whitelisting the
/// address of this struct as a kernel object.
pub mod global {
    use crate::kobj::StaticKObj;
    use core::ops::Deref;

    #[allow(non_camel_case_types)]
    pub struct k_mutex(StaticKObj<super::KMutex>);

    impl k_mutex {
        pub const unsafe fn uninit() -> Self {
            k_mutex(StaticKObj::uninit())
        }
    }

    impl Deref for k_mutex {
        type Target = StaticKObj<super::KMutex>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
}

/// Raw syscall API
pub trait MutexSyscalls {
    unsafe fn k_mutex_init(mutex: *mut zephyr_sys::raw::k_mutex);
    unsafe fn k_mutex_lock(
        mutex: *mut zephyr_sys::raw::k_mutex,
        timeout: zephyr_sys::raw::s32_t,
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
        match C::k_mutex_lock(
            self as *const _ as *mut _,
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

/// Safe mutex container like that in std
///
/// Using this is safe, but creating it is not. Creator must ensure it is not
/// possible to get a reference to the data elsewhere. Lifetime bounds ensure the
/// mutex kobject lives at least as long as the data it protects.
#[derive(Copy)]
pub struct Mutex<'m: 'd, 'd, T> {
    mutex: &'m KMutex,
    data: &'d MutexData<T>,
}

impl<'m: 'd, 'd, T> Mutex<'m, 'd, T> {
    pub unsafe fn new(mutex: &'m KMutex, data: &'d MutexData<T>) -> Self {
        Mutex { mutex, data }
    }

    pub unsafe fn kobj(&self) -> *mut libc::c_void {
        self.mutex as *const _ as *mut _
    }

    pub fn lock<C: MutexSyscalls>(&self) -> MutexGuard<'d, T, C> {
        unsafe {
            self.mutex.lock::<C>();
        }
        MutexGuard(self.clone(), PhantomData)
    }
}

impl<'m: 'd, 'd, T> Clone for Mutex<'m, 'd, T> {
    fn clone(&self) -> Self {
        Mutex {
            mutex: self.mutex,
            data: self.data,
        }
    }
}

pub struct MutexGuard<'a, T, C: MutexSyscalls>(Mutex<'a, 'a, T>, PhantomData<C>);

impl<'a, T, C: MutexSyscalls> Drop for MutexGuard<'a, T, C> {
    fn drop(&mut self) {
        unsafe { self.0.mutex.unlock::<C>() }
    }
}

impl<'a, T, C: MutexSyscalls> Deref for MutexGuard<'a, T, C> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.0.data.0.get() }
    }
}

impl<'a, T, C: MutexSyscalls> DerefMut for MutexGuard<'a, T, C> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0.data.0.get() }
    }
}

pub struct MutexData<T>(UnsafeCell<T>);

unsafe impl<T> Sync for MutexData<T> {}

impl<T> MutexData<T> {
    pub const fn new(data: T) -> Self {
        MutexData(UnsafeCell::new(data))
    }
}
