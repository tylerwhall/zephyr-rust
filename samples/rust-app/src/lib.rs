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

// OS-TLS initializes lazily even with const syntax; use From to avoid
// Clippy's missing_const_for_thread_local false positive in that macro path.
thread_local!(static TLS: RefCell<u8> = RefCell::from(1));

zephyr_macros::k_mutex_define!(MUTEX);
zephyr_macros::k_sem_define!(TLS_SEM, 0, 1);
zephyr_macros::k_sem_define!(STD_MUTEX_START, 0, 1);
zephyr_macros::k_sem_define!(STD_MUTEX_CHECKED, 0, 1);
zephyr_macros::k_sem_define!(STD_MUTEX_DONE, 0, 1);

static STD_MUTEX: std::sync::Mutex<u8> = std::sync::Mutex::new(0);

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

fn std_mutex_test() {
    use zephyr::context::Any as C;

    // Exercise lazy native mutex allocation in both kernel and user mode.
    println!("std::sync::Mutex::new");
    let lock = std::sync::Mutex::new(0u8);
    println!("std::sync::Mutex::lock");
    *lock.lock().unwrap() = 1;
    assert_eq!(*lock.try_lock().unwrap(), 1);

    // A single-threaded lock/unlock test also passes with std's no_threads
    // backend. Have another Zephyr thread try, then block on, a held lock.
    let mut guard = STD_MUTEX.lock().unwrap();
    *guard = 1;
    STD_MUTEX_START.give::<C>();
    STD_MUTEX_CHECKED.take::<C>();
    // Let the contender try to enter lock() while this thread still holds the guard.
    std::thread::sleep(Duration::from_millis(1));
    assert_eq!(*guard, 1);
    drop(guard);
    STD_MUTEX_DONE.take::<C>();
    assert_eq!(*STD_MUTEX.lock().unwrap(), 2);
    println!("std::sync::Mutex contention passed");
}

fn mutex_pool_test() {
    use zephyr::context::Any as C;
    use zephyr::mutex_alloc::{DynMutex, MUTEX_POOL_SIZE};

    if MUTEX_POOL_SIZE == 0 {
        return;
    }
    let fill = || {
        // Stack storage avoids allocator growth while running in userspace.
        let mut mutexes: [Option<DynMutex>; MUTEX_POOL_SIZE] = std::array::from_fn(|_| None);
        let mut count = 0;
        while let Some(mutex) = DynMutex::new::<C>() {
            assert!(count < MUTEX_POOL_SIZE);
            let ptr = &*mutex as *const _;
            assert!(mutexes.iter().flatten().all(|other| &**other as *const _ != ptr));
            mutexes[count] = Some(mutex);
            count += 1;
        }
        (mutexes, count)
    };
    // Exhaustion, uniqueness, and free/reuse, including a partial bitmap byte.
    let available = fill().1;
    assert!(available > 0);
    assert_eq!(fill().1, available);

    // std and app wrappers must reserve slots from the SAME bitmap, even when
    // Cargo compiles independent instances of zephyr-core for them.
    let std_mutex = std::sync::Mutex::new(0u8);
    let guard = std_mutex.lock().unwrap();
    let app_mutexes = fill();
    assert_eq!(app_mutexes.1, available - 1);
    drop(app_mutexes);
    drop(guard);
    println!("Shared mutex pool exhaustion and reuse passed");
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

    // The main thread runs std_mutex_test() once in kernel mode and once in
    // user mode. Use Zephyr's thread because std::thread::spawn is unsupported.
    for _ in 0..2 {
        use zephyr::context::Kernel as C;

        STD_MUTEX_START.take::<C>();
        assert!(matches!(
            STD_MUTEX.try_lock(),
            Err(std::sync::TryLockError::WouldBlock)
        ));
        STD_MUTEX_CHECKED.give::<C>();
        {
            let mut guard = STD_MUTEX.lock().unwrap();
            assert_eq!(*guard, 1);
            *guard = 2;
        }
        STD_MUTEX_DONE.give::<C>();
    }
}

#[no_mangle]
pub extern "C" fn rust_main() {
    use zephyr::context::Kernel as Context;

    println!("Hello from Rust on Zephyr {} via println!", zephyr::KERNEL_VERSION);
    zephyr::kernel::k_str_out("Hello from Rust kernel with direct kernel call\n");
    zephyr::any::k_str_out("Hello from Rust kernel with runtime-detect syscall\n");

    std::thread::sleep(Duration::from_millis(1));
    println!("Time {:?}", zephyr::any::k_uptime_ticks().as_millis());
    println!("Time {:?}", std::time::Instant::now());

    let current = Context::k_current_get();
    current.k_object_access_grant::<Context, _>(&MUTEX);
    current.k_object_access_grant::<Context, _>(&TLS_SEM);
    current.k_object_access_grant::<Context, _>(&STD_MUTEX_START);
    current.k_object_access_grant::<Context, _>(&STD_MUTEX_CHECKED);
    current.k_object_access_grant::<Context, _>(&STD_MUTEX_DONE);
    mutex_test();
    std_mutex_test();
    mutex_pool_test();

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
        std_mutex_test();
        mutex_pool_test();

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
