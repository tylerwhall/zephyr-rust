extern crate futures;
extern crate futures_util;
extern crate libc;
extern crate zephyr;
extern crate zephyr_futures;
extern crate zephyr_macros;

use libc::c_void;

use futures::future;
use futures_util::stream::StreamExt;

use zephyr::semaphore::*;
use zephyr_futures::{Executor, SemaphoreStream};

zephyr_macros::k_sem_define!(TEST_SEM, 0, 10);

#[no_mangle]
pub extern "C" fn rust_sem_thread(_a: *const c_void, _b: *const c_void, _c: *const c_void) {
    use zephyr::context::Kernel as C;
    for i in 0..10 {
        println!("Giving {}", i);
        TEST_SEM.give::<C>();
    }
}

zephyr_macros::k_mutex_define!(EXECUTOR_MUTEX);

#[no_mangle]
pub extern "C" fn rust_test_main() {
    use zephyr::context::Kernel as C;

    let f = SemaphoreStream::new(&TEST_SEM)
        .take(10)
        .enumerate()
        .for_each(|(i, _val)| {
            println!("Took {}", i);
            future::ready(())
        });
    let mut executor = unsafe { Executor::new(&EXECUTOR_MUTEX) };
    executor.spawn(C, f);
    executor.run::<C>();
}
