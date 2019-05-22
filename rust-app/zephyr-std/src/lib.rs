#![feature(staged_api)]
#![stable(feature = "rust1", since = "1.0.0")]
#![no_std]

#[stable(feature = "rust1", since = "1.0.0")]
pub use core::*;

#[cfg(all(feature = "kernelmode", not(feature = "usermode")))]
use ::zephyr::kernel as zephyr;
#[cfg(all(not(feature = "kernelmode"), feature = "usermode"))]
use ::zephyr::user as zephyr;
#[cfg(all(feature = "kernelmode", feature = "usermode"))]
use ::zephyr::any as zephyr;

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

mod panicking;
