#![no_std]

pub mod io {
    use core::fmt::Write;

    pub struct Stdout;

    impl Write for Stdout {
        #[inline(always)]
        fn write_str(&mut self, s: &str) -> core::result::Result<(), core::fmt::Error> {
            unsafe { zephyr_sys::syscalls::k_str_out(s.as_ptr() as *mut _, s.len()) };
            Ok(())
        }
    }
}
