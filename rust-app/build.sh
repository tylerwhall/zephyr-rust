#!/bin/sh -ex

HOST=$(rustc -vV | grep host: | cut -d ' ' -f 2)
VERSION="+nightly-2019-05-22"
CARGO_ARGS="${VERSION} -v build --target=${RUST_TARGET} --release"
cargo ${CARGO_ARGS} --target-dir=${SYSROOT_BUILD} --manifest-path=./sysroot/Cargo.toml -p std

SYSROOT_LIB="${SYSROOT}/lib/rustlib/${RUST_TARGET}/lib"
SYSROOT_LIB_HOST="${SYSROOT}/lib/rustlib/${HOST}/lib"

copy_dir_if_changed() {
    mkdir -p $2
    if ! diff --brief --recursive $1 $2; then
        rm -r $2
        cp -a $1 $2
    fi
}

copy_dir_if_changed ${SYSROOT_BUILD}/${RUST_TARGET}/release/deps ${SYSROOT_LIB}
copy_dir_if_changed ${SYSROOT_BUILD}/release/deps ${SYSROOT_LIB_HOST}

export RUSTFLAGS="${RUSTFLAGS} --sysroot ${SYSROOT}"
cargo ${CARGO_ARGS} --target-dir=${APP_BUILD} --manifest-path=${CARGO_MANIFEST}
