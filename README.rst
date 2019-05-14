Rust on Zephyr RTOS
###################

Overview
********
Zephyr_ project template for building a cargo project and linking it into a Zephyr image.

.. _Zephyr: https://github.com/zephyrproject-rtos/zephyr

Building and Running
********************

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

Rust toolchain
==============

.. code-block:: console

    rustup toolchain install nightly
    rustup target add i686-unknown-linux-gnu
    rustup target add thumbv7m-none-eabi

This project outputs 'Hello World' to the console.  It can be built and executed
on QEMU as follows:

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

    Hello from Rust
    Hello World! native_posix 42

Supported Architectures
***********************

* native_posix
* x86
* armv7m

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
