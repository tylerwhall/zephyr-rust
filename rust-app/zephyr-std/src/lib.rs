#![feature(staged_api)]

#![stable(feature = "rust1", since = "1.0.0")]

#![no_std]

#[stable(feature = "rust1", since = "1.0.0")]
pub use core::*;

pub mod io {
    #![stable(feature = "rust1", since = "1.0.0")]

    use core::fmt::Write;

    #[stable(feature = "rust1", since = "1.0.0")]
    pub struct Stdout;

    #[stable(feature = "rust1", since = "1.0.0")]
    impl Write for Stdout {
        #[inline(always)]
        fn write_str(&mut self, s: &str) -> core::result::Result<(), core::fmt::Error> {
            unsafe { zephyr_sys::syscalls::k_str_out(s.as_ptr() as *mut _, s.len()) };
            Ok(())
        }
    }
}

mod panicking;
