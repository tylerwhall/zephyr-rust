Rust on Zephyr RTOS
###################

Overview
********
Zephyr_ module for building a Cargo project and linking it into a Zephyr image.
Add this directory to ZEPHYR_EXTRA_MODULES to build a Cargo library project
(located in the Zephyr app's source directory by default) and link it into the
Zephyr app.

Version Compatibility
=====================
**Zephyr**: v2.3, v2.7.3, v3.7. 3.0-3.6 not supported.

**Rust**: exactly 1.75.0

Please use one of the above Zephyr releases before reporting issues! At the
time you are reading this, Zephyr's main branch will likely not work, though it
is usually one 1-2 minor changes to support a new release. The project aims to
support 2.3, the LTS releases, and the latest release.

Features
========

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


.. _Zephyr: https://github.com/zephyrproject-rtos/zephyr

Building and Running
********************

Clone the repo
==============

Make sure to clone the submodules recursively. This points to modified Rust libstd.

.. code-block:: console

    git clone --recurse-submodules https://github.com/tylerwhall/zephyr-rust.git

Zephyr setup
============

Refer to the Zephyr getting started guide_. This includes installing west,
getting Zephyr source, and the Zephyr toolchain. Make sure you can build a C
sample within Zephyr.

.. _guide: https://docs.zephyrproject.org/2.5.0/getting_started/index.html

See above for tested compatible Zephyr releases. Please try a release if master
does not work. Due to differences in the syscall header generation, v1.14 LTS
is no longer supported.
See `issue 16 <https://github.com/tylerwhall/zephyr-rust/issues/16>`_.

Rust toolchain
==============

The compiler version must exactly match the version of standard library
included as a submodule of this project. In practice, using a different
compiler version often fails to compile because of Rust internally making heavy
use of unstable compiler features.

The current base is stable-1.75.0. Rustup is the default workflow, and the
rust-toolchain file in this repo should cause rustup to automatically install
and use the right version. If not, manually install:

.. code-block:: console

    rustup toolchain install 1.75.0

If supplying your own rustc and cargo, make sure they are the version above.
The build will fail if it detects a version mismatch.

Also install clang from your distro. This is required by bindgen to generate
syscall bindings. Else you will get this error

.. code-block:: console

    thread 'main' panicked at 'Unable to find libclang: "couldn\'t find any valid shared libraries matching: [\'libclang.so\', \'libclang-*.so\', \'libclang.so.*\']

Build
=====

.. code-block:: console

    west build -p auto -b <board name> samples/rust-app/

Native:

.. code-block:: console

    west build -p auto -b native_posix samples/rust-app/

qemu_x86:

.. code-block:: console

    west build -p auto -b qemu_x86 samples/rust-app/

ARM Cortex-M:

.. code-block:: console

    west build -p auto -b qemu_cortex_m3 samples/rust-app/

These errors are normal. Needs investigation, but the binary is still created
successfully.

.. code-block:: console

    x86_64-zephyr-elf-objdump: DWARF error: mangled line number section (bad file number)

Run (QEMU targets):

.. code-block:: console

    cd build
    ninja run

Sample Output
=============

.. code-block:: console

    *** Booting Zephyr OS build zephyr-v2.2.0  ***
    Hello Rust println
    Hello from Rust kernel with direct kernel call
    Hello from Rust kernel with runtime-detect syscall
    Hello from second thread
    second thread: f = 1
    second thread: now f = 55
    Time InstantMs(20)
    Time Instant(InstantMs(20))
    Locking
    Unlocking
    No device
    Boxed value 1
    main thread: f = 1
    main thread: now f = 2
    Hello from Rust userspace with forced user-mode syscall
    Locking
    Unlocking
    INFO app: TEST: info!()
    WARN app: TEST: warn!()
    ERROR app: TEST: error!()
    main thread: f = 2
    main thread: now f = 3
    Hello from Rust userspace with forced user-mode syscall
    Hello from Rust userspace with runtime-detect syscall
    Next call will crash if userspace is working.
    FAILED: zephyr/CMakeFiles/run

Failure is from an intentional crash at the end of the sample.

Testing
*******

The Zephyr test runner can be used:

.. code-block:: console

    $ZEPHYR_BASE/scripts/sanitycheck --testcase-root tests -p native_posix -N

Or you can build and run the test manually:

.. code-block:: console

    west build -p auto -b native_posix tests/rust
    cd build
    ninja run

Supported Architectures
***********************

* native_posix
* x86
* armv7m
* armv7r
* thumbv7em

Really anything that works with Zephyr and Rust should work. Only need to
define a target.json and add a case for it in CMakelists.

TODO
****

* Figure out how to fail tests through assertions in code
* Support #[test]
* Ability to build multiple independent apps
* More safe bindings (e.g. GPIO)

Features Not Planned to Support
===============================

* std::thread. Requires thread resources to be dynamically allocated. This is
  possible, but not common for Zephyr.
* Defining static threads in Rust. Zephyr uses many layers of
  architecture-specific C macros that would not be wise to try to duplicate
  exactly in Rust. Possibly could generate C code like in the "cpp" crate, but
  for now just define threads in C and point them at a Rust FFI entry point.
* std::sync::{Mutex, RwLock}. Mutex should work when built without userspace
  support. Userspace would require (at least) CONFIG_DYNAMIC_OBJECTS. While
  this is possible, I don't want to require it to use libstd. May revisit.
  The small number of uses in libstd are patched out.

License
*******

Licensed under either of

* Apache License, Version 2.0 http://www.apache.org/licenses/LICENSE-2.0
* MIT license http://opensource.org/licenses/MIT

at your option.

Contribution
============

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
