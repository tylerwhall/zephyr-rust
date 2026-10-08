# Rust on Zephyr RTOS

## Overview

[Zephyr](https://github.com/zephyrproject-rtos/zephyr) module for building a Cargo project and linking it into a Zephyr image.
Add this directory to ZEPHYR_EXTRA_MODULES to build a Cargo library project
(located in the Zephyr app's source directory by default) and link it into the
Zephyr app.

## Version Compatibility

**Zephyr**: v2.3, v2.7.3, v3.7. 3.0-3.6 not supported.

**Rust**: exactly 1.88.0

Use one of these supported releases before reporting issues; other releases
and Zephyr's main branch are not covered by this repository's CI.

## Features

* Generated bindings for all syscalls
* Safe wrappers for some Zephyr APIs (mutex, semaphore, timers, k_poll, UART)
* Basic libstd port (no_std not necessary)
* Heap (std::alloc) see CONFIG_RUST_ALLOC_POOL
* Thread-local storage
* Kernel or user-mode Rust

  * Rust globals and heap in a Rust-specific memory segment that can be granted to specific threads
  * Syscalls compile to direct C function calls when !CONFIG_USERSPACE
  * Note: running kernel and user-mode Rust at the same time could pose a security risk, since there is one shared global allocator

* Minimal std::futures executor

  * Supports dynamic tasks and timers
  * Currently single-threaded
  * async/await UART example

* Implemented as a Zephyr module for inclusion in existing Zephyr projects
* No modifications to Zephyr source

## Building and Running

### Clone the repo

Make sure to clone the submodules recursively. This points to modified Rust libstd.

```console
git clone --recurse-submodules https://github.com/tylerwhall/zephyr-rust.git
```

### Zephyr setup

Refer to the Zephyr getting started [guide](https://docs.zephyrproject.org/3.7.0/develop/getting_started/index.html). This includes installing west,
getting Zephyr source, and the Zephyr toolchain. Make sure you can build a C
sample within Zephyr.

Use the matching toolchain and setup instructions for your supported Zephyr
release; see the version list above.

### Rust toolchain

The compiler version must exactly match the version of standard library
included as a submodule of this project. In practice, using a different
compiler version often fails to compile because of Rust internally making heavy
use of unstable compiler features.

The current base is stable-1.88.0. Rustup is the default workflow, and the
rust-toolchain file in this repo should cause rustup to automatically install
and use the right version. If not, manually install:

```console
rustup toolchain install 1.88.0
```

If supplying your own rustc and cargo, make sure they are the version above.
The build will fail if it detects a version mismatch.

Also install clang from your distro. This is required by bindgen to generate
syscall bindings. Else you will get this error

```console
thread 'main' panicked at 'Unable to find libclang: "couldn't find any valid shared libraries matching: ['libclang.so', 'libclang-*.so', 'libclang.so.*']
```

### Build

```console
west build -p auto -b <board name> samples/rust-app/
```

Native:

```console
west build -p auto -b native_posix samples/rust-app/
```

qemu_x86:

```console
west build -p auto -b qemu_x86 samples/rust-app/
```

ARM Cortex-M:

```console
west build -p auto -b qemu_cortex_m3 samples/rust-app/
```

Run the default qemu_x86 sample from the repository root with process-group
cleanup (other sample/board combinations may not exit automatically):

```console
BUILD_DIR=build bash ci/run-sample.sh
```

### Sample Output

The default sample checks TLS isolation and shared mutex-pool allocation in
kernel and user mode. Successful output reaches:

```console
Hello from Rust userspace with runtime-detect syscall
Next call will crash if userspace is working.
```

It then intentionally faults and returns status 1 to prove userspace isolation.

## Testing

Execute the tests on Zephyr 2.3.0/qemu_x86 and qemu_cortex_m3 in the CI container:

```console
cd ci
RUST_VERSION=1.88.0 ./sanitycheck.sh
```

Later-version tests are currently build-only; twister execution is pending.
Build an individual test with a board from its testcase.yaml whitelist:

```console
west build -p auto -b qemu_x86 tests/semaphore
```

For full build coverage, run `cd ci && RUN=1 ./build-all.sh`. The six verified
sample runs are not a substitute for executing the test suite. See
[BUILD_MATRIX_TODO.md](docs/BUILD_MATRIX_TODO.md) for coverage and remaining work.

## Supported targets

The build matrix covers

- x86
- Cortex-M/R (including ARMv8-M)
- RISC-V 32/64
- native_posix

Board/version restrictions are defined in `ci/matrix.py`. Porting another
target requires a matching Rust target JSON, CMake target selection, and
build/runtime validation; support is not automatic.

## Structure: Submodules pointing to forks

This repository points two top-level submodules at Tyler Hall forks:

* ``rust/rust`` -> https://github.com/tylerwhall/rust.git
* ``rust/libc`` -> https://github.com/tylerwhall/libc.git

Net change relative to upstream Rust:

* ``rust/rust`` is the Zephyr standard library port (sys integration,
  stdout/stderr, allocator, mutex, instant/time, sleep, thread parking,
  interruption, plus build/submodule adjustments).
* ``rust/libc`` is the Zephyr-specific libc additions for Zephyr OS support
  and API/type glue used by the standard library port.

Nested Rust submodules that remain upstream:

* ``rust/rust/library/backtrace`` -> https://github.com/rust-lang/backtrace-rs.git
* ``rust/rust/library/stdarch`` -> https://github.com/rust-lang/stdarch.git

## Rust crate layering

- `zephyr-sys` contains the generated low-level FFI and syscall bindings.
- `zephyr-core` contains no_std-capable wrappers used by both std and apps.
- `zephyr` adds application-facing APIs and std adapters.

Applications and helpers declare their Zephyr crate dependencies in Cargo.
The std port builds private instances of core/sys; these do not share Rust
crate identity with application instances. CMake generates bindings once
per image, the C runtime owns shared mutex-pool bookkeeping, and the generated
application root registers the allocator once. Use
`zephyr::time::instant_ticks(instant)` for conversion to Zephyr ticks.

CMake runs `rust/cargo.sh` to build std and the app together with Cargo
`-Zbuild-std`. A build-local toolchain/source overlay leaves your installed
toolchain untouched; neither upstream `rust-src` nor a compiler wrapper is
required. The reviewed std resolution is in `rust/Cargo.lock`. See
[BUILD_STD.md](docs/BUILD_STD.md) for details.

For a core/alloc-only image, set `CONFIG_RUST_STD=n`, use only no_std-capable
dependencies, and supply a `#[panic_handler]`. `CONFIG_RUST_ALLOC_POOL`
registers the allocator automatically; otherwise supply your own. See
`samples/no_std` (the default remains std-enabled).

### Porting applications to Rust 1.85

- Add explicit Cargo dependencies for each directly used `zephyr_core`,
  `zephyr_sys`, or `libc` crate, pointing to this checkout's `rust/zephyr-core`,
  `rust/zephyr-sys`, or `rust/libc`. Do not enable `rustc-dep-of-std` for apps.
  Object-definition macros from `zephyr-macros` also require a direct
  `zephyr-core` dependency. Refresh the affected `Cargo.lock` files.
- Replace `Ticks::from(instant)` or `instant.into()` with
  `zephyr::time::instant_ticks(instant)`.
- With `CONFIG_RUST_ALLOC_POOL`, the generated app root registers the global
  allocator; do not register a second one. If you bypass that root, register
  your allocator explicitly (e.g. `zephyr_core::global_sys_mem_pool!(rust_std_mem_pool)`).

## TODO

* Figure out how to fail tests through assertions in code
* Support #[test]
* Ability to build multiple independent apps
* More safe bindings (e.g. GPIO)

### Features Not Planned to Support

* std::thread. Requires thread resources to be dynamically allocated. This is
  possible, but not common for Zephyr.
* Defining static threads in Rust. Zephyr uses many layers of
  architecture-specific C macros that would not be wise to try to duplicate
  exactly in Rust. Possibly could generate C code like in the "cpp" crate, but
  for now just define threads in C and point them at a Rust FFI entry point.
* std::sync::RwLock. std::sync::Mutex is supported, including userspace
  through CONFIG_RUST_MUTEX_POOL; it does not require CONFIG_DYNAMIC_OBJECTS.

## License

Licensed under either of

* Apache License, Version 2.0 http://www.apache.org/licenses/LICENSE-2.0
* MIT license http://opensource.org/licenses/MIT

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
