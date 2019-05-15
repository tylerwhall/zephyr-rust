extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    let flags = env::var("TARGET_CFLAGS").unwrap_or("".to_string());
    eprintln!("cflags: {}", flags);

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        .use_core()
        .ctypes_prefix("ctypes")
        // XXX: doesn't handle args with spaces in quotes
        .clang_args(flags.split(" "))
        .blacklist_item(".*x86_mmu.*")
        .blacklist_item(".*x86_.*pdpt")
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
