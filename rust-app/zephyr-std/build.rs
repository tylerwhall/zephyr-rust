use std::env;

fn main() {
    let userspace = env::var("CONFIG_USERSPACE").expect("CONFIG_USERSPACE must be set") == "y";
    eprintln!("userspace: {}", userspace);

    // Always work from the kernel for now. This has the effect of using direct
    // kernel calls if userspace is disabled, and using runtime-detect calls if
    // enabled.
    println!("cargo:rustc-cfg=feature=\"kernelmode\"");
    if userspace {
        println!("cargo:rustc-cfg=feature=\"usermode\"");
    }
}