extern crate libc;
extern crate zephyr;
extern crate zephyr_macros;

use libc::c_void;
use zephyr::context::Kernel as C;
use zephyr::semaphore::*;

zephyr_macros::k_sem_define!(TEST_SEM, 0, 10);

#[no_mangle]
pub extern "C" fn rust_sem_thread(_a: *const c_void, _b: *const c_void, _c: *const c_void) {
    for i in 0..10 {
        println!("Giving {}", i);
        TEST_SEM.give::<C>();
    }
}

#[no_mangle]
pub extern "C" fn rust_test_main() {
    for i in 0..10 {
        println!("Taking {}", i);
        TEST_SEM.take::<C>();
        println!("Took {}", i);
    }
}
