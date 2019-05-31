#[macro_use]
extern crate log;

extern crate zephyr_logger;

use log::LevelFilter;
use zephyr_macros::k_mutex_define;

k_mutex_define!(TEST_MUTEX);

#[no_mangle]
pub extern "C" fn hello_rust() {
    println!("Hello Rust println");
    zephyr::kernel::k_str_out("Hello from Rust kernel with direct kernel call\n");
    zephyr::any::k_str_out("Hello from Rust kernel with runtime-detect syscall\n");

    println!("Time {:?}", zephyr::any::k_uptime_get_ms());
    println!("Time {:?}", std::time::Instant::now());

    unsafe {
        TEST_MUTEX.lock();
        TEST_MUTEX.unlock();
    }

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
