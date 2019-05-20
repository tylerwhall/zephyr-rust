/// Empty stub for compiler_builtins
///
/// libsyntax adds "extern crate" for core and compiler_builtins when using #[no_std]
/// The real builtins conflict with those provided by libgcc which is already used by Zephyr.

#![no_std]
// Remove implicit dependency on compiler builtins
#![feature(compiler_builtins)]
#![compiler_builtins]
// Remove dependency on libcore. Real compiler_builtins needs this but we don't.
#![feature(no_core)]
#![no_core]
