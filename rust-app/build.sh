#!/bin/sh -ex

VERSION="+nightly-2019-05-22"
cargo ${VERSION} -v build --target=${RUST_TARGET} --target-dir=${CARGO_TARGET_DIR} --release --manifest-path=./sysroot/Cargo.toml -p std

SYSROOT_LIB="${SYSROOT}/lib/rustlib/${RUST_TARGET}/lib"
mkdir -p ${SYSROOT_LIB}
cp -a ${CARGO_TARGET_DIR}/${RUST_TARGET}/release/deps/* ${SYSROOT_LIB}

export RUSTFLAGS="${RUSTFLAGS} --sysroot ${SYSROOT}"
cargo ${VERSION} -v build --target=${RUST_TARGET} --target-dir=${CARGO_TARGET_DIR} --release
