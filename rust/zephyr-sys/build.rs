use std::{env, path::Path};

fn main() {
    // CMake generates these once from this image's headers/devicetree/Kconfig.
    // Both std-private and ordinary application instances consume the same ABI.
    println!("cargo:rerun-if-env-changed=ZEPHYR_RUST_BINDINGS");
    let bindings = env::var("ZEPHYR_RUST_BINDINGS").expect("ZEPHYR_RUST_BINDINGS unset");
    for name in ["bindings.rs", "syscalls.rs"] {
        let path = Path::new(&bindings).join(name);
        assert!(path.is_file(), "missing generated bindings: {}", path.display());
        println!("cargo:rerun-if-changed={}", path.display());
    }
    println!("cargo:rustc-env=ZEPHYR_RUST_BINDINGS={bindings}");
}
