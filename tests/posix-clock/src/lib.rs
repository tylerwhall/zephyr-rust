extern crate zephyr_core;
extern crate zephyr_sys;

#[no_mangle]
pub extern "C" fn test_main() {
    std::thread::sleep(std::time::Duration::from_secs(1));

    let ts = zephyr_core::kernel::clock_gettime();
    println!("sec: {} nsec {}", ts.tv_sec, ts.tv_nsec);
    zephyr_core::kernel::clock_settime(ts);
}
