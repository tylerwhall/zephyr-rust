extern crate bindgen;

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bindgen::callbacks::ParseCallbacks;

#[derive(Debug, Default)]
struct CallbacksInner {
    /// List of all syscalls found from bindgen
    syscalls: Vec<String>,
}
#[derive(Clone, Debug, Default)]
struct Callbacks(Arc<Mutex<CallbacksInner>>);

impl ParseCallbacks for Callbacks {
    fn item_name(&self, name: &str) -> Option<String> {
        const PREFIX: &str = "z_anyctx_";
        let mut inner = self.0.lock().unwrap();
        if name.starts_with(PREFIX) {
            inner.syscalls.push(name[PREFIX.len()..].into());
        }
        None
    }
}

fn main() {
    let flags = env::var("TARGET_CFLAGS").unwrap_or("".to_string());
    eprintln!("cflags: {}", flags);
    let userspace = env::var("CONFIG_USERSPACE").expect("CONFIG_USERSPACE must be set") == "y";
    eprintln!("userspace: {}", userspace);

    let callbacks = Callbacks::default().clone();
    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        .use_core()
        .ctypes_prefix("libc")
        .parse_callbacks(Box::new(callbacks.clone()))
        // XXX: doesn't handle args with spaces in quotes
        .clang_args(flags.split(" "))
        .opaque_type("z_x86_thread_stack_header")
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // Namespace aliases to syscalls by their valid contexts
    let mut out = File::create(out_path.join("syscalls.rs")).unwrap();
    let syscalls = callbacks.0.lock().unwrap();

    writeln!(&mut out, "pub mod any {{").unwrap();
    for syscall in syscalls.syscalls.iter() {
        writeln!(
            &mut out,
            "    pub use super::super::raw::z_anyctx_{} as {};",
            syscall, syscall
        )
        .unwrap();
    }
    writeln!(&mut out, "}}").unwrap();
    writeln!(&mut out, "pub mod user {{").unwrap();
    if userspace {
        // If userspace enabled, output each userspace-context syscall here
        for syscall in syscalls.syscalls.iter() {
            writeln!(
                &mut out,
                "    pub use super::super::raw::z_userctx_{} as {};",
                syscall, syscall
            )
            .unwrap();
        }
    } else {
        // Else, import all the kernel functions since they can be called directly
        writeln!(&mut out, "pub use super::any::*;").unwrap();
    }
    writeln!(&mut out, "}}").unwrap();
    writeln!(&mut out, "pub mod kernel {{").unwrap();
    if userspace {
        // If userspace, put the kernel-specific functions in the root of the module
        for syscall in syscalls.syscalls.iter() {
            writeln!(
                &mut out,
                "    pub use super::super::raw::z_kernelctx_{} as {};",
                syscall, syscall
            )
            .unwrap();
        }
    } else {
        // Else, import all kernel functions since they can be called directly
        writeln!(&mut out, "pub use super::any::*;").unwrap();
    }
    writeln!(&mut out, "}}").unwrap();
}
