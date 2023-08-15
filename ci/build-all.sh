#!/bin/bash -e

#rm -rf log

ZEPHYR_VERSIONS="2.7.3 2.6.0 2.5.0 2.4.0 2.3.0"

parallel \
    -j1 \
    --results log/container \
    --resume \
    --halt now,fail=1 \
    ZEPHYR_VERSION={1} ./container-build.sh \
    ::: $ZEPHYR_VERSIONS

# First build the main sample for zephyr versions
parallel \
    -j4 \
    --results log/build \
    --resume \
    --halt now,fail=1 \
    ZEPHYR_VERSION={1} ./build-cmd.sh west build -d /tmp/build -p auto -b {2} {3} \
    ::: $ZEPHYR_VERSIONS \
    ::: qemu_x86 \
    ::: samples/rust-app

# Full set. --resume removes duplicates from above
parallel \
    -j4 \
    --results log/build \
    --resume \
    ZEPHYR_VERSION={1} ./build-cmd.sh west build -d /tmp/build -p auto -b {2} {3} \
    ::: $ZEPHYR_VERSIONS \
    ::: qemu_x86 qemu_cortex_m3 \
    ::: samples/rust-app samples/serial samples/futures
