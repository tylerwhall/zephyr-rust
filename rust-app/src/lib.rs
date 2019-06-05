#[macro_use]
extern crate log;

extern crate zephyr_logger;

use std::time::Duration;

use log::LevelFilter;

use zephyr::mutex::*;

zephyr_macros::k_mutex_define!(MUTEX);

fn mutex_test() {
    let data = MutexData::new(1u32);

    // Bind the static mutex to our local data. This would make more sense if
    // the data were static, but that requires app mem regions for user mode.
    let mutex = unsafe { Mutex::new(&MUTEX, &data) };

    unsafe {
        // No safe interface implemented yet
        let current = zephyr_sys::syscalls::any::k_current_get();
        zephyr_sys::syscalls::any::k_object_access_grant(mutex.kobj(), current);
    }

    zephyr::any::k_str_out("Locking\n");
    let _val = mutex.lock::<zephyr::context::Any>();
    zephyr::any::k_str_out("Unlocking\n");
}

#[no_mangle]
pub extern "C" fn hello_rust() {
    println!("Hello Rust println");
    zephyr::kernel::k_str_out("Hello from Rust kernel with direct kernel call\n");
    zephyr::any::k_str_out("Hello from Rust kernel with runtime-detect syscall\n");

    std::thread::sleep(Duration::from_millis(1));
    println!("Time {:?}", zephyr::any::k_uptime_get_ms());
    println!("Time {:?}", std::time::Instant::now());

    mutex_test();

    {
        let boxed = Box::new(1u8);
        println!("Boxed value {}", boxed);
    }

    // test std::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive}
    {
        let a: [u8; 4] = [1, 2, 3, 4];
        let len = a.iter().len();
        for _ in &a[0..len] {}
        for _ in &a[0..=(len - 1)] {}
        for _ in &a[..] {}
        for _ in &a[0..] {}
        for _ in &a[..len] {}
        for _ in &a[..=(len - 1)] {}
    }

    zephyr::kernel::k_thread_user_mode_enter(|| {
        zephyr::user::k_str_out("Hello from Rust userspace with forced user-mode syscall\n");

        mutex_test();

        zephyr_logger::init(LevelFilter::Info);

        trace!("TEST: trace!()");
        debug!("TEST: debug!()");
        info!("TEST: info!()");
        warn!("TEST: warn!()");
        error!("TEST: error!()");

        zephyr::user::k_str_out("Hello from Rust userspace with forced user-mode syscall\n");

        zephyr::any::k_str_out("Hello from Rust userspace with runtime-detect syscall\nNext call will crash if userspace is working.\n");

        // This will compile, but crash if CONFIG_USERSPACE is working
        zephyr::kernel::k_str_out("Hello from Rust userspace with direct kernel call\n");
    });
}
