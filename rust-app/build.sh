#!/bin/sh -ex

HOST=$(rustc -vV | grep host: | cut -d ' ' -f 2)
VERSION="+nightly-2019-05-22"
cargo ${VERSION} -v build --target=${RUST_TARGET} --target-dir=${CARGO_TARGET_DIR}/sysroot-build --release --manifest-path=./sysroot/Cargo.toml -p std

SYSROOT_LIB="${SYSROOT}/lib/rustlib/${RUST_TARGET}/lib"
SYSROOT_LIB_HOST="${SYSROOT}/lib/rustlib/${HOST}/lib"
mkdir -p ${SYSROOT_LIB} ${SYSROOT_LIB_HOST}
cp -a ${CARGO_TARGET_DIR}/sysroot-build/${RUST_TARGET}/release/deps/* ${SYSROOT_LIB}
cp -a ${CARGO_TARGET_DIR}/sysroot-build/release/deps/* ${SYSROOT_LIB_HOST}

export RUSTFLAGS="${RUSTFLAGS} --sysroot ${SYSROOT}"
cargo ${VERSION} -v build --target=${RUST_TARGET} --target-dir=${CARGO_TARGET_DIR} --release
