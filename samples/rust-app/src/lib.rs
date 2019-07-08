#[macro_use]
extern crate cstr;
#[macro_use]
extern crate log;

extern crate zephyr_macros;
extern crate zephyr;
extern crate zephyr_logger;

use std::cell::RefCell;
use std::time::Duration;

use core::ffi::c_void;
use log::LevelFilter;

use zephyr::device::DeviceSyscalls;
use zephyr::mutex::*;
use zephyr::semaphore::*;
use zephyr::thread::ThreadSyscalls;

thread_local!(static TLS: RefCell<u8> = RefCell::new(1));

zephyr_macros::k_mutex_define!(MUTEX);
zephyr_macros::k_sem_define!(TLS_SEM, 0, 1);

fn mutex_test() {
    let data = 1u32;

    // Bind the static mutex to our local data. This would make more sense if
    // the data were static, but that requires app mem regions for user mode.
    let mutex = unsafe { Mutex::new(&MUTEX, &data) };

    // Should allow cloning directly if the data is a reference.
    let _other_mutex = mutex.clone();
    zephyr::any::k_str_out("Locking\n");
    let _val = mutex.lock::<zephyr::context::Any>();
    zephyr::any::k_str_out("Unlocking\n");
}

fn thread_join_std_mem_domain(_context: zephyr::context::Kernel) {
    use zephyr::context::Kernel as C;
    zephyr::static_mem_domain!(rust_std_domain).add_thread::<C>(C::k_current_get());
}

#[no_mangle]
pub extern "C" fn rust_second_thread(
    _a: *const c_void,
    _b: *const c_void,
    _c: *const c_void,
) {
    thread_join_std_mem_domain(zephyr::context::Kernel);

    println!("Hello from second thread");

    TLS.with(|f| {
        println!("second thread: f = {}", *f.borrow());
        assert!(*f.borrow() == 1);
        *f.borrow_mut() = 55;
        println!("second thread: now f = {}", *f.borrow());
        assert!(*f.borrow() == 55);
    });

    // Let thread 1 access TLS after we have already set it. Value should not be seen on thread 1
    TLS_SEM.give::<zephyr::context::Kernel>();
}

#[no_mangle]
pub extern "C" fn rust_main() {
    use zephyr::context::Kernel as Context;

    println!("Hello Rust println");
    zephyr::kernel::k_str_out("Hello from Rust kernel with direct kernel call\n");
    zephyr::any::k_str_out("Hello from Rust kernel with runtime-detect syscall\n");

    std::thread::sleep(Duration::from_millis(1));
    println!("Time {:?}", zephyr::any::k_uptime_get_ms());
    println!("Time {:?}", std::time::Instant::now());

    let current = Context::k_current_get();
    current.k_object_access_grant::<Context, _>(&MUTEX);
    current.k_object_access_grant::<Context, _>(&TLS_SEM);
    mutex_test();

    if let Some(_device) = Context::device_get_binding(cstr!("nonexistent")) {
        println!("Got device");
    } else {
        println!("No device");
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

    TLS_SEM.take::<Context>();
    assert!(!TLS_SEM.try_take::<Context>());
    TLS.with(|f| {
        println!("main thread: f = {}", *f.borrow());
        assert!(*f.borrow() == 1);
        *f.borrow_mut() = 2;
        println!("main thread: now f = {}", *f.borrow());
        assert!(*f.borrow() == 2);
    });
    TLS_SEM.give::<Context>();

    thread_join_std_mem_domain(Context);
    zephyr::kernel::k_thread_user_mode_enter(|| {
        use zephyr::context::User as Context;

        zephyr::user::k_str_out("Hello from Rust userspace with forced user-mode syscall\n");

        mutex_test();

        zephyr_logger::init(LevelFilter::Info);

        trace!("TEST: trace!()");
        debug!("TEST: debug!()");
        info!("TEST: info!()");
        warn!("TEST: warn!()");
        error!("TEST: error!()");

        assert!(TLS_SEM.try_take::<Context>());
        TLS.with(|f| {
            println!("main thread: f = {}", *f.borrow());
            assert!(*f.borrow() == 2);
            *f.borrow_mut() = 3;
            println!("main thread: now f = {}", *f.borrow());
            assert!(*f.borrow() == 3);
        });

        zephyr::user::k_str_out("Hello from Rust userspace with forced user-mode syscall\n");

        zephyr::any::k_str_out("Hello from Rust userspace with runtime-detect syscall\nNext call will crash if userspace is working.\n");

        // This will compile, but crash if CONFIG_USERSPACE is working
        zephyr::kernel::k_str_out("Hello from Rust userspace with direct kernel call\n");
    });
}
