#![feature(lang_items)]
#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)] // Zero size struct for k_spinlock

use core::fmt::Write;

pub mod zephyr_sys {
    pub mod raw {
        include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
    }

    pub mod syscalls {
        include!(concat!(env!("OUT_DIR"), "/syscalls.rs"));
    }

    pub mod ctypes {
        use core::fmt;

        #[cfg(target_arch = "aarch64")]
        pub type c_char = u8;
        #[cfg(not(target_arch = "aarch64"))]
        pub type c_char = i8;
        pub type c_schar = i8;
        pub type c_uchar = u8;
        pub type c_short = i16;
        pub type c_ushort = u16;
        pub type c_int = i32;
        pub type c_uint = u32;
        #[cfg(target_pointer_width = "32")]
        pub type c_long = i32;
        #[cfg(target_pointer_width = "32")]
        pub type c_ulong = u32;
        #[cfg(target_pointer_width = "64")]
        pub type c_long = i64;
        #[cfg(target_pointer_width = "64")]
        pub type c_ulong = u64;
        pub type c_longlong = i64;
        pub type c_ulonglong = u64;
        pub type c_float = f32;
        pub type c_double = f64;

        #[repr(u8)]
        pub enum c_void {
            #[doc(hidden)]
            __variant1,
            #[doc(hidden)]
            __variant2,
        }

        impl fmt::Debug for c_void {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.pad("c_void")
            }
        }
    }

    #[panic_handler]
    fn panic(_panic: &core::panic::PanicInfo<'_>) -> ! {
        loop {}
    }

    #[lang = "eh_personality"]
    extern "C" fn eh_personality() {}
}

pub struct Stdout;

impl Write for Stdout {
    #[inline(always)]
    fn write_str(&mut self, s: &str) -> core::result::Result<(), core::fmt::Error> {
        unsafe { zephyr_sys::syscalls::k_str_out(s.as_ptr() as *mut _, s.len()) };
        Ok(())
    }
}

#[no_mangle]
pub extern "C" fn hello_rust() {
    use zephyr_sys::syscalls;

    writeln!(&mut Stdout, "Hello Rust writeln").unwrap();
    {
        const MSG: &str = "Hello from Rust kernel with direct kernel call\n";
        unsafe { syscalls::kernel::k_str_out(MSG.as_ptr() as *mut _, MSG.len()) };
    }
    {
        const MSG: &str = "Hello from Rust kernel with runtime-detect syscall\n";
        unsafe { syscalls::k_str_out(MSG.as_ptr() as *mut _, MSG.len()) };
    }
}

#[no_mangle]
pub extern "C" fn hello_rust_user() {
    use zephyr_sys::syscalls;

    {
        const MSG: &str = "Hello from Rust userspace with forced user-mode syscall\n";
        unsafe { syscalls::user::k_str_out(MSG.as_ptr() as *mut _, MSG.len()) };
    }
    {
        const MSG: &str = "Hello from Rust userspace with runtime-detect syscall\nNext call will crash if userspace is working.\n";
        unsafe { syscalls::k_str_out(MSG.as_ptr() as *mut _, MSG.len()) };
    }

    // This will compile, but crash if CONFIG_USERSPACE is working
    {
        const MSG: &str = "Hello from Rust userspace with direct kernel call\n";
        unsafe { syscalls::kernel::k_str_out(MSG.as_ptr() as *mut _, MSG.len()) };
    }
}
