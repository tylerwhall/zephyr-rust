use std::env;

fn main() {
    // This build.rs just invokes a binary from a different project.  We do it this way to avoid
    // having any build-dependencies in this project.  Cargo hopelessly conflates dependencies and
    // build-dependencies in a way that makes it impossible to build libstd
    let zb = env::var("ZEPHYR_BINDGEN").expect("ZEPHYR_BINDGEN unset");
    let rc = std::process::Command::new(zb)
        .status()
        .expect("cargo run failed");
    assert!(rc.success());
}
