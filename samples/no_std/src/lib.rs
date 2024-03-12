#![cfg_attr(not(feature = "have_std"), no_std)]

#[macro_use]
extern crate cstr;
#[macro_use]
extern crate log;

extern crate zephyr_macros;
#[cfg(feature = "have_std")]
extern crate zephyr;
extern crate zephyr_core;
extern crate zephyr_logger;

use core::ffi::c_void;
use log::LevelFilter;

extern crate alloc;
use alloc::format;
use alloc::boxed::Box;

use zephyr::device::DeviceSyscalls;
use zephyr_core::mutex::*;
use zephyr_core::semaphore::*;
use zephyr_core::thread::ThreadSyscalls;

#[cfg(feature = "have_std")]
use std::cell::RefCell;
#[cfg(feature = "have_std")]
thread_local!(static TLS: std::cell::RefCell<u8> = RefCell::new(1));

zephyr_macros::k_mutex_define!(MUTEX);
zephyr_macros::k_sem_define!(TLS_SEM, 0, 1);

fn mutex_test() {
    let data = 1u32;

    // Bind the static mutex to our local data. This would make more sense if
    // the data were static, but that requires app mem regions for user mode.
    let mutex = unsafe { Mutex::new(&MUTEX, &data) };

    // Should allow cloning directly if the data is a reference.
    let _other_mutex = mutex.clone();
    zephyr_core::any::k_str_out("Locking\n");
    let _val = mutex.lock::<zephyr_core::context::Any>();
    zephyr_core::any::k_str_out("Unlocking\n");
}

#[cfg(feature = "have_std")]
fn std_mutex_test() {
    println!("std::sync::Mutex::new");
    let lock = std::sync::Mutex::new(0u8);
    println!("std::sync::Mutex::lock");
    *lock.lock().unwrap() = 1;
}

fn thread_join_std_mem_domain(_context: zephyr_core::context::Kernel) {
    use zephyr_core::context::Kernel as C;
    zephyr_core::static_mem_domain!(rust_std_domain).add_thread::<C>(C::k_current_get());
}

#[no_mangle]
pub extern "C" fn rust_second_thread(
    _a: *const c_void,
    _b: *const c_void,
    _c: *const c_void,
) {
    thread_join_std_mem_domain(zephyr_core::context::Kernel);

    zephyr_core::any::k_str_out("Hello from second thread\n");

    #[cfg(feature = "have_std")]
    TLS.with(|f| {
        zephyr_core::any::k_str_out(format!("second thread: f = {}\n", *f.borrow()).as_str());
        assert!(*f.borrow() == 1);
        *f.borrow_mut() = 55;
        zephyr_core::any::k_str_out(format!("second thread: now f = {}\n", *f.borrow()).as_str());
        assert!(*f.borrow() == 55);
    });

    // Let thread 1 access TLS after we have already set it. Value should not be seen on thread 1
    TLS_SEM.give::<zephyr_core::context::Kernel>();
}

#[no_mangle]
pub extern "C" fn rust_main() {
    use zephyr_core::context::Kernel as Context;

    #[cfg(feature = "have_std")]
    println!("Hello from Rust on Zephyr {} via println!", zephyr_core::KERNEL_VERSION);
    zephyr_core::kernel::k_str_out("Hello from Rust kernel with direct kernel call\n");
    zephyr_core::any::k_str_out("Hello from Rust kernel with runtime-detect syscall\n");

    #[cfg(feature = "have_std")]
    std::thread::sleep(std::time::Duration::from_millis(1));
    zephyr_core::any::k_str_out(format!("Time {:?}\n", zephyr_core::any::k_uptime_ticks().as_millis()).as_str());
    #[cfg(feature = "have_std")]    
    println!("Time {:?}", std::time::Instant::now());

    let current = Context::k_current_get();
    current.k_object_access_grant::<Context, _>(&MUTEX);
    current.k_object_access_grant::<Context, _>(&TLS_SEM);
    mutex_test();
    #[cfg(feature = "have_std")]
    std_mutex_test();

    if let Some(_device) = Context::device_get_binding(cstr!("nonexistent")) {
        zephyr_core::any::k_str_out("Got device\n")
    } else {
        zephyr_core::any::k_str_out("No device\n")
    }

    {
        let boxed = Box::new(1u8);
        zephyr_core::any::k_str_out(format!("Boxed value {}\n", boxed).as_str());
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

    TLS_SEM.take::<Context>();
    assert!(!TLS_SEM.try_take::<Context>());
    #[cfg(feature = "have_std")]
    TLS.with(|f| {
        println!("main thread: f = {}\n", *f.borrow());
        assert!(*f.borrow() == 1);
        *f.borrow_mut() = 2;
        println!("main thread: now f = {}\n", *f.borrow());
        assert!(*f.borrow() == 2);
    });
    TLS_SEM.give::<Context>();

    thread_join_std_mem_domain(Context);
    zephyr_core::kernel::k_thread_user_mode_enter(|| {
        use zephyr_core::context::User as Context;

        zephyr_core::user::k_str_out("Hello from Rust userspace with forced user-mode syscall\n");

        mutex_test();
        #[cfg(feature = "have_std")]
        std_mutex_test();

        zephyr_logger::init(LevelFilter::Info);

        trace!("TEST: trace!()");
        debug!("TEST: debug!()");
        info!("TEST: info!()");
        warn!("TEST: warn!()");
        error!("TEST: error!()");

        assert!(TLS_SEM.try_take::<Context>());
        #[cfg(feature = "have_std")]
        TLS.with(|f| {
            println!("main thread: f = {}", *f.borrow());
            assert!(*f.borrow() == 2);
            *f.borrow_mut() = 3;
            println!("main thread: now f = {}", *f.borrow());
            assert!(*f.borrow() == 3);
        });

        zephyr_core::user::k_str_out("Hello from Rust userspace with forced user-mode syscall\n");

        zephyr_core::any::k_str_out("Hello from Rust userspace with runtime-detect syscall\nNext call will crash if userspace is working.\n");

        // This will compile, but crash if CONFIG_USERSPACE is working
        zephyr_core::kernel::k_str_out("Hello from Rust userspace with direct kernel call\n");
    });
}
