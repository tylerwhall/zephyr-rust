fn main() {
    for var in [
        "CONFIG_USERSPACE",
        "CONFIG_RUST_ALLOC_POOL",
        "CONFIG_RUST_MUTEX_POOL",
        "CONFIG_POSIX_CLOCK",
        "CONFIG_THREAD_LOCAL_STORAGE",
    ] {
        println!("cargo:rerun-if-env-changed={var}");
    }

    // The Zephyr version cfgs (zephyr250/zephyr270/zephyr300/zephyr350) are
    // exported via RUSTFLAGS in rust-env.sh by CMakeLists.txt, so they apply
    // to every std and app crate instance.

    // Register even disabled Kconfig cfgs for Rust 1.80's cfg checking.
    for cfg in ["usermode", "mempool", "mutex_pool", "clock", "tls"] {
        println!("cargo:rustc-check-cfg=cfg({cfg})");
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
