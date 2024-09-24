use core::ptr::NonNull;

use crate::kobj::KObj;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThreadId(NonNull<zephyr_sys::raw::k_thread>);

impl ThreadId {
    pub fn tid(&self) -> zephyr_sys::raw::k_tid_t {
        self.0.as_ptr()
    }

    pub fn k_wakeup<C: ThreadSyscalls>(&self) {
        C::k_wakeup(*self)
    }

    pub fn k_object_access_grant<C: ThreadSyscalls, K: KObj>(&self, kobj: &K) {
        C::k_object_access_grant(kobj, *self)
    }
}

pub trait ThreadSyscalls {
    fn k_wakeup(thread: ThreadId);
    fn k_current_get() -> crate::thread::ThreadId;
    fn k_object_access_grant<K: KObj>(kobj: &K, thread: ThreadId);
}

macro_rules! trait_impl {
    ($context:ident, $context_struct:path) => {
        impl ThreadSyscalls for $context_struct {
            fn k_wakeup(thread: ThreadId) {
                unsafe { zephyr_sys::syscalls::$context::k_wakeup(thread.tid()) }
            }

            /* If less than 2.7, always use the k_current_get syscall */
            #[cfg(not(zephyr270))]
            fn k_current_get() -> crate::thread::ThreadId {
                ThreadId(unsafe {
                    NonNull::new_unchecked(zephyr_sys::syscalls::$context::k_current_get())
                })
            }

            /* 2.7 and above with TLS allows pulling from TLS */
            #[cfg(all(zephyr270, tls))]
            fn k_current_get() -> crate::thread::ThreadId {
                extern "C" {
                    #[no_mangle]
                    static z_tls_current: *mut zephyr_sys::raw::k_thread;
                }
                ThreadId(unsafe {
                    NonNull::new_unchecked(z_tls_current)
                })
            }

            /* 2.7 without TLS renames the syscall to z_current_get */
            #[cfg(all(zephyr270, not(zephyr350), not(tls)))]
            fn k_current_get() -> crate::thread::ThreadId {
                ThreadId(unsafe {
                    NonNull::new_unchecked(zephyr_sys::syscalls::$context::z_current_get())
                })
            }

            /* 3.5 renames the syscall again to k_sched_current_thread_query */
            #[cfg(all(zephyr350, not(tls)))]
            fn k_current_get() -> crate::thread::ThreadId {
                ThreadId(unsafe {
                    NonNull::new_unchecked(zephyr_sys::syscalls::$context::k_sched_current_thread_query())
                })
            }

            fn k_object_access_grant<K: KObj>(kobj: &K, thread: ThreadId) {
                if !zephyr_sys::raw::RUST_CONFIG_USERSPACE {
                    // Avoid unnecessary call to stub function
                    return;
                }
                unsafe {
                    zephyr_sys::syscalls::$context::k_object_access_grant(
                        kobj.as_void_ptr(),
                        thread.tid(),
                    );
                }
            }
        }
    };
}

trait_impl!(kernel, crate::context::Kernel);
trait_impl!(user, crate::context::User);
trait_impl!(any, crate::context::Any);
