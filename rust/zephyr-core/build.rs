fn main() {
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
}
