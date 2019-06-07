#[cfg(feature = "have_std")]
use std::ffi::CStr;

use crate::kobj::KObj;

pub use zephyr_sys::raw::device as Device;

unsafe impl KObj for Device {}

pub trait DeviceSyscalls {
    #[cfg(feature = "have_std")]
    fn device_get_binding(device_name: &CStr) -> Option<&'static Device>;
}

macro_rules! trait_impl {
    ($context:ident, $context_struct:path) => {
        impl DeviceSyscalls for $context_struct {
            #[cfg(feature = "have_std")]
            #[inline(always)]
            fn device_get_binding(device_name: &CStr) -> Option<&'static Device> {
                unsafe {
                    // All devices are static in Zephyr, so static lifetime
                    // Option<&T> is guaranteed to have the null pointer optimization, so we can cast
                    // https://doc.rust-lang.org/nomicon/ffi.html#the-nullable-pointer-optimization
                    core::mem::transmute(zephyr_sys::syscalls::user::device_get_binding(
                        device_name.as_ptr(),
                    ))
                }
            }
        }
    };
}

trait_impl!(kernel, crate::context::Kernel);
trait_impl!(user, crate::context::User);
trait_impl!(any, crate::context::Any);
