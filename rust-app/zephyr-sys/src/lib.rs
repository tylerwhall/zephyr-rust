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
