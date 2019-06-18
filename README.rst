Rust on Zephyr RTOS
###################

Overview
********
Zephyr_ module for building a cargo project and linking it into a Zephyr image.
Add this directory to ZEPHYR_MODULES to build a Cargo library project (located
in the Zephyr app's source directory by default) and link it into the Zephyr
app.

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

Set up a Zephyr toolchain (e.g. Zephyr SDK_)

.. _SDK: https://docs.zephyrproject.org/latest/getting_started/installation_linux.html#zephyr-sdk

.. code-block:: console

    wget https://github.com/zephyrproject-rtos/sdk-ng/releases/download/v0.10.0/zephyr-sdk-0.10.0-setup.run
    sh zephyr-sdk-0.10.0-setup.run

Add toolchain to ~/.zephyrrc. This is sourced by the Zephyr env script.

.. code-block:: shell

    export ZEPHYR_TOOLCHAIN_VARIANT=zephyr
    export ZEPHYR_SDK_INSTALL_DIR=<sdk installation directory>


Acquire Zephyr source, export ZEPHYR_BASE, and source the Zephyr env script.

.. code-block:: console

    git clone https://github.com/zephyrproject-rtos/zephyr.git $HOME/src/zephyr
    export ZEPHYR_BASE=$HOME/src/zephyr
    . $ZEPHYR_BASE/zephyr-env.sh

Install Zephyr's "West" build tool. Needed for ZEPHYR_MODULES support.

.. code-block:: console

    pip3 install --user west

Rust toolchain
==============

A nightly compiler is required to use unstable features that are unavoidable
when implementing libstd. The nightly date is arbitrary, but needs to be locked
so long as we are using rust-src from rustup.

.. code-block:: console

    rustup toolchain install nightly-2019-05-22
    rustup default nightly-2019-05-22
    rustup component add rustfmt
    rustup component add rust-src

Also install clang. This is required by bindgen to generate syscall bindings.
Else you will get this error

.. code-block:: console

    thread 'main' panicked at 'Unable to find libclang: "couldn\'t find any valid shared libraries matching: [\'libclang.so\', \'libclang-*.so\', \'libclang.so.*\']

Build
=====

Build and run on QEMU (or posix native) as follows:

.. code-block:: console

    cd samples/rust-app

Native:

.. code-block:: console

    mkdir -p build-posix && cd build-posix
    cmake -GNinja -DBOARD=native_posix ..

qemu_x86:

.. code-block:: console

    mkdir -p build-x86 && cd build-x86
    cmake -GNinja -DBOARD=qemu_x86 ..

ARM Cortex-M:

.. code-block:: console

    mkdir -p build-arm && cd build-arm
    cmake -GNinja -DBOARD=qemu_cortex_m3 ..

Build and run:

.. code-block:: console

    ninja run

Sample Output
=============

.. code-block:: console

    SeaBIOS (version rel-1.12.0-0-ga698c8995f-prebuilt.qemu.org)
    Booting from ROM..***** Booting Zephyr OS zephyr-v1.14.0-752-gfd97e44011f6 *****
    Hello from Rust kernel with direct kernel call
    Hello from Rust kernel with runtime-detect syscall
    Entering user mode
    Hello from Rust userspace with forced user-mode syscall
    Hello from Rust userspace with runtime-detect syscall
    Next call will crash if userspace is working.
    ***** CPU Page Fault (error code 0x00000004)
    User thread read address 0x00408000
    PDE: 0x027 Present, Writable, User, Execute Enabled
    PTE: 0x800000002 Non-present, Writable, Supervisor, Execute Disable
    Current thread ID = 0x00400060
    eax: 0x00000048, ebx: 0x000086aa, ecx: 0x0000002b, edx: 0x00000064
    esi: 0x000086da, edi: 0x004043e8, ebp: 0x004043ac, esp: 0x004043a0
    eflags: 0x00000207 cs: 0x002b
    call trace:
    eip: 0x0000140b
         0x0000035d (0x86a9)
         Fatal fault in thread 0x00400060! Aborting.


Supported Architectures
***********************

* native_posix
* x86
* armv7m

Really anything that works with Zephyr and Rust should work. Only need to
define a target.json and add a case for it in CMakelists.

TODO
****

* Build as a Zephyr module
* Separate Rust app from zephyr crates and sysroot (be able to build multiple apps)
* Kconfig for enabling Rust and configuring the heap
* test runner

Complete
========

* generate syscall bindings (including inline functions in kernel mode)
* minimal port of rust libstd
* println
* alloc from kernel mode (Box)
* split into zephyr-sys and zephyr crates
* thread local storage
* alloc from user mode
* abstraction for pointers to kernel objects
* safe wrappers (threads, semaphores, etc.)
* panic

Features Not Planned to Support
===============================

* std::thread. Requires thread resources to be dynamically allocated. This is
  possible, but not common for Zephyr.
* Defining static threads in Rust. Zephyr uses many layers of
  architecture-specific C macros that would not be wise to try to duplicate
  exactly in Rust. Possibly could generate C code like in the "cpp" crate, but
  for now just define threads in C and point them at a Rust FFI entry point.
* std::sync::{Mutex, RwLock}. Might be possible but would at least require
  dynamic kernel object allocation. The small number of uses in libstd are
  patched out.

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
