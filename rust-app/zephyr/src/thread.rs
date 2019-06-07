use crate::kobj::KObj;

#[derive(Clone, Copy)]
pub struct ThreadId(zephyr_sys::raw::k_tid_t);

impl ThreadId {
    pub fn k_object_access_grant<C: ThreadSyscalls, K: KObj>(&self, kobj: &K) {
        C::k_object_access_grant(kobj, *self)
    }
}

pub trait ThreadSyscalls {
    fn k_current_get() -> crate::thread::ThreadId;
    fn k_object_access_grant<K: KObj>(kobj: &K, thread: ThreadId);
}

macro_rules! trait_impl {
    ($context:ident, $context_struct:path) => {
        impl ThreadSyscalls for $context_struct {
            fn k_current_get() -> crate::thread::ThreadId {
                ThreadId(unsafe { zephyr_sys::syscalls::$context::k_current_get() })
            }

            fn k_object_access_grant<K: KObj>(kobj: &K, thread: ThreadId) {
                unsafe {
                    if !zephyr_sys::raw::RUST_CONFIG_USERSPACE {
                        // Avoid unnecessary call to stub function
                        return;
                    }

                    zephyr_sys::syscalls::$context::k_object_access_grant(
                        kobj.as_void_ptr(),
                        thread.0,
                    );
                }
            }
        }
    };
}

trait_impl!(kernel, crate::context::Kernel);
trait_impl!(user, crate::context::User);
trait_impl!(any, crate::context::Any);
