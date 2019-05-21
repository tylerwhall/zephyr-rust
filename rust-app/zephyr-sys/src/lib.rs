#![feature(lang_items)]
#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)] // Zero size struct for k_spinlock

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
