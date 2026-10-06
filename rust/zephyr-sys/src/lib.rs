#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)] // Zero size struct for k_spinlock

pub mod raw {
    // Bindgen's incomplete-array and bitfield helpers lack safety docs and
    // use generic transmutes even when the source and destination types match
    // (or a bitfield is bool). Keep these exceptions on generated code only;
    // the handwritten kernel object wrappers below still receive every lint.
    #[allow(
        clippy::missing_safety_doc,
        clippy::useless_transmute,
        clippy::transmute_int_to_bool
    )]
    mod bindings {
        include!(concat!(env!("ZEPHYR_RUST_BINDINGS"), "/bindings.rs"));
    }
    pub use self::bindings::*;

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
    include!(concat!(env!("ZEPHYR_RUST_BINDINGS"), "/syscalls.rs"));
}
