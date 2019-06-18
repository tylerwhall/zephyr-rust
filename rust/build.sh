#!/bin/sh -ex

HOST=$(rustc -vV | grep host: | cut -d ' ' -f 2)
VERSION="+nightly-2019-05-22"
CARGO_ARGS="${VERSION} -v build --target=${RUST_TARGET} --release"

publish_sysroot() {
    local SYSROOT=$1
    local SYSROOT_LIB="${SYSROOT}/lib/rustlib/${RUST_TARGET}/lib"
    local SYSROOT_LIB_HOST="${SYSROOT}/lib/rustlib/${HOST}/lib"
    shift
    rm -rf ${SYSROOT}
    mkdir -p ${SYSROOT_LIB} ${SYSROOT_LIB_HOST}

    for src in $@; do
        cp -a $src/${RUST_TARGET}/release/deps/. ${SYSROOT_LIB}
        cp -a $src/release/deps/. ${SYSROOT_LIB_HOST}
    done
}

# Build std
cargo ${CARGO_ARGS} \
    --target-dir=${SYSROOT_BUILD}-stage1 \
    --manifest-path=./sysroot-stage1/Cargo.toml -p std
publish_sysroot ${SYSROOT}-stage1 ${SYSROOT_BUILD}-stage1
# Build Zephyr crates
RUSTFLAGS="${RUSTFLAGS} --sysroot ${SYSROOT}-stage1" cargo ${CARGO_ARGS} \
    --target-dir=${SYSROOT_BUILD}-stage2 \
    --manifest-path=./sysroot-stage2/Cargo.toml

publish_sysroot ${SYSROOT} ${SYSROOT_BUILD}-stage1 ${SYSROOT_BUILD}-stage2

export RUSTFLAGS="${RUSTFLAGS} --sysroot ${SYSROOT}"
cargo ${CARGO_ARGS} --target-dir=${APP_BUILD} --manifest-path=${CARGO_MANIFEST}
