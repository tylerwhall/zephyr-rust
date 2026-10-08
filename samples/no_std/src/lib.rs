#![no_std]

extern crate alloc;
use alloc::{boxed::Box, format};
use core::ffi::c_void;
use zephyr_core::mutex::*;
use zephyr_core::semaphore::*;
use zephyr_core::thread::ThreadSyscalls;

zephyr_macros::k_mutex_define!(MUTEX);
zephyr_macros::k_sem_define!(THREAD_SEM, 0, 1);

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    zephyr_core::any::k_str_out("Rust panic in no_std sample\n");
    loop {
        core::hint::spin_loop();
    }
}

fn mutex_test() {
    let data = 1u32;
    // Static kernel object bound to local data, accessible from user mode.
    let mutex = unsafe { Mutex::new(&MUTEX, &data) };
    let _other_mutex = mutex.clone();
    zephyr_core::any::k_str_out("Locking\n");
    let _val = mutex.lock::<zephyr_core::context::Any>();
    zephyr_core::any::k_str_out("Unlocking\n");
}

fn thread_join_std_mem_domain() {
    use zephyr_core::context::Kernel as C;
    zephyr_core::static_mem_domain!(rust_std_domain).add_thread::<C>(C::k_current_get());
}

#[no_mangle]
pub extern "C" fn rust_second_thread(
    _a: *const c_void,
    _b: *const c_void,
    _c: *const c_void,
) {
    thread_join_std_mem_domain();
    zephyr_core::any::k_str_out("Hello from second thread\n");
    THREAD_SEM.give::<zephyr_core::context::Kernel>();
}

#[no_mangle]
pub extern "C" fn rust_main() {
    use zephyr_core::context::Kernel as Context;

    zephyr_core::kernel::k_str_out("Hello from Rust kernel with direct kernel call\n");
    zephyr_core::any::k_str_out("Hello from Rust kernel with runtime-detect syscall\n");
    zephyr_core::any::k_str_out(
        format!("Time {:?}\n", zephyr_core::any::k_uptime_ticks().as_millis()).as_str(),
    );

    let current = Context::k_current_get();
    current.k_object_access_grant::<Context, _>(&MUTEX);
    current.k_object_access_grant::<Context, _>(&THREAD_SEM);
    mutex_test();

    let device = unsafe {
        zephyr_sys::syscalls::kernel::device_get_binding(b"nonexistent\0".as_ptr().cast())
    };
    assert!(device.is_null());
    zephyr_core::any::k_str_out("No device\n");

    let boxed = Box::new(1u8);
    assert_eq!(*boxed, 1);
    zephyr_core::any::k_str_out(format!("Boxed value {boxed}\n").as_str());
    drop(boxed);

    let a: [u8; 4] = [1, 2, 3, 4];
    let len = a.len();
    assert_eq!(&a[0..len], &a[0..=len - 1]);
    assert_eq!(&a[..], &a[0..]);
    assert_eq!(&a[..len], &a[..=len - 1]);

    THREAD_SEM.take::<Context>();
    assert!(!THREAD_SEM.try_take::<Context>());
    THREAD_SEM.give::<Context>();

    thread_join_std_mem_domain();
    zephyr_core::kernel::k_thread_user_mode_enter(|| {
        use zephyr_core::context::User as Context;
        zephyr_core::user::k_str_out("Hello from Rust userspace with forced user-mode syscall\n");
        mutex_test();
        assert!(THREAD_SEM.try_take::<Context>());
        zephyr_core::any::k_str_out("Hello from Rust userspace with runtime-detect syscall\nNext call will crash if userspace is working.\n");
        // Intentional fault when CONFIG_USERSPACE is working.
        zephyr_core::kernel::k_str_out("Hello from Rust userspace with direct kernel call\n");
    });
}
