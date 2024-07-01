fn main() {
    let kernel_version_str_trimmed = std::env::var("ZEPHYR_KERNEL_VERSION_NUM")
        .expect("ZEPHYR_KERNEL_VERSION_NUM must be set")
        .trim_start_matches("0x").to_owned();
    let kernel_version = u32::from_str_radix(&kernel_version_str_trimmed, 16)
        .expect("ZEPHYR_KERNEL_VERSION_NUM must be an integer");

    if kernel_version >= 0x2_05_00 {
        println!("cargo:rustc-cfg=zephyr250");
    }
    if kernel_version >= 0x2_07_00 {
        println!("cargo:rustc-cfg=zephyr270");
    }
    if kernel_version >= 0x3_00_00 {
        println!("cargo:rustc-cfg=zephyr300");
    }
    if kernel_version >= 0x3_05_00 {
        println!("cargo:rustc-cfg=zephyr350");
    }

    if std::env::var("CONFIG_USERSPACE").expect("CONFIG_USERSPACE must be set") == "y" {
        println!("cargo:rustc-cfg=usermode");
    }
    if std::env::var("CONFIG_RUST_ALLOC_POOL").expect("CONFIG_RUST_ALLOC_POOL must be set") == "y" {
        println!("cargo:rustc-cfg=mempool");
    }
    if std::env::var("CONFIG_RUST_MUTEX_POOL").expect("CONFIG_RUST_MUTEX_POOL must be set") == "y" {
        println!("cargo:rustc-cfg=mutex_pool");
    }
    if std::env::var("CONFIG_POSIX_CLOCK").expect("CONFIG_POSIX_CLOCK must be set") == "y" {
        println!("cargo:rustc-cfg=clock");
    }
    if let Ok(tls) = std::env::var("CONFIG_THREAD_LOCAL_STORAGE") {
        if tls == "y" {
            println!("cargo:rustc-cfg=tls");
        }
    }
}
