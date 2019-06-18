fn main() {
    if std::env::var("CONFIG_USERSPACE").expect("CONFIG_USERSPACE must be set") == "y" {
        println!("cargo:rustc-cfg=usermode");
    }
}
