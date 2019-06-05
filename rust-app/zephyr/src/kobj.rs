use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::ops::Deref;

/// A kernel object that is usable via system calls.
///
/// This implies locking is done in the kernel and this is therefore Send + Sync. e.g. mutex,
/// semaphore, fifo. Implement this for the raw Zephyr C structs.
pub unsafe trait KObj {}

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
