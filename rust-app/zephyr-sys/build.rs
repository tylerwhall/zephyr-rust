use std::env;

fn main() {
    // This build.rs just invokes "cargo run" in a different project.  We do it this way to avoid
    // having any build-dependencies in this project.  Cargo hopelessly conflates dependencies and
    // build-dependencies in a way that makes it impossible to build libstd
    let mut zb = std::path::PathBuf::new();
    zb.push(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR unset"));
    zb.push("zephyr-bindgen");
    let rc = std::process::Command::new("cargo")
        .args(&["run"])
        .current_dir(zb)
        .status()
        .expect("cargo run failed");
    assert!(rc.success());
}
