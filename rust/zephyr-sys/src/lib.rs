#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)] // Zero size struct for k_spinlock

pub mod raw {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

    unsafe impl Send for k_mutex {}
    unsafe impl Sync for k_mutex {}
    unsafe impl Send for k_sem {}
    unsafe impl Sync for k_sem {}
    unsafe impl Send for device {}
    unsafe impl Sync for device {}

    // Recreate what the K_FOREVER macro does
    pub const K_FOREVER: k_timeout_t = k_timeout_t {
        ticks: -1 as k_ticks_t,
    };

    pub const K_NO_WAIT: k_timeout_t = k_timeout_t { ticks: 0 };
}

pub mod syscalls {
    include!(concat!(env!("OUT_DIR"), "/syscalls.rs"));
}
