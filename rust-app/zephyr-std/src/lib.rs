#![feature(alloc_prelude)]
#![feature(allocator_api)]
#![feature(allocator_internals)]
#![feature(alloc_error_handler)]
#![feature(prelude_import)]
#![feature(rustc_attrs)]
#![feature(staged_api)]
#![feature(todo_macro)]
#![stable(feature = "rust1", since = "1.0.0")]
#![no_std]
#![default_lib_allocator]

#[prelude_import]
#[allow(unused)]
use prelude::v1::*;

extern crate alloc as alloc_crate;

#[stable(feature = "rust1", since = "1.0.0")]
pub use core::{assert_eq, assert_ne, debug_assert, debug_assert_eq, debug_assert_ne};
#[stable(feature = "rust1", since = "1.0.0")]
pub use core::{r#try, todo, unimplemented, unreachable, write, writeln};

#[stable(feature = "rust1", since = "1.0.0")]
pub use core::fmt;

#[cfg(all(feature = "kernelmode", feature = "usermode"))]
use ::zephyr::any as zephyr;
#[cfg(all(feature = "kernelmode", not(feature = "usermode")))]
use ::zephyr::kernel as zephyr;
#[cfg(all(not(feature = "kernelmode"), feature = "usermode"))]
use ::zephyr::user as zephyr;

pub mod alloc;

pub mod io {
    #![stable(feature = "rust1", since = "1.0.0")]

    use core::fmt::Write;

    #[stable(feature = "rust1", since = "1.0.0")]
    pub struct Stdout;

    #[stable(feature = "rust1", since = "1.0.0")]
    impl Write for Stdout {
        #[inline(always)]
        fn write_str(&mut self, s: &str) -> core::result::Result<(), core::fmt::Error> {
            crate::zephyr::k_str_out(s);
            Ok(())
        }
    }
}

pub mod prelude;

mod panicking;
