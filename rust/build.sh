#!/bin/sh -ex

HOST=$(rustc -vV | grep host: | cut -d ' ' -f 2)
CARGO_ARGS="-v build --target=${RUST_TARGET} --release"
VERSION="1.37.0"
CURRENT_CARGO_VERSION=$(cargo -vV | grep ^release: | cut -d ' ' -f 2)

# Assert cargo version matches the certified version
if [ "${CURRENT_CARGO_VERSION}x" != "${VERSION}x" ]; then
    echo "Error: Current cargo version: ${CURRENT_CARGO_VERSION}, expected: ${VERSION}"
    echo "If using rustup, it should be automatically installed. If not, run"
    echo "rustup toolchain install ${VERSION}"
    exit 1
fi

publish_sysroot() {
    local CLEAN_DIR_IF_CHANGED=$1
    shift
    local SYSROOT=$1
    shift
    local SYSROOT_LIB="${SYSROOT}/lib/rustlib/${RUST_TARGET}/lib"
    local SYSROOT_LIB_HOST="${SYSROOT}/lib/rustlib/${HOST}/lib"
    mkdir -p ${SYSROOT_LIB} ${SYSROOT_LIB_HOST}
    mkdir -p ${SYSROOT_LIB}-new ${SYSROOT_LIB_HOST}-new

    for src in $@; do
        cp -a $src/${RUST_TARGET}/release/deps/. ${SYSROOT_LIB}-new
        cp -a $src/release/deps/. ${SYSROOT_LIB_HOST}-new
    done
    if ! diff -qr ${SYSROOT_LIB} ${SYSROOT_LIB}-new || ! diff -qr ${SYSROOT_LIB_HOST} ${SYSROOT_LIB_HOST}-new; then
        rm -rf ${CLEAN_DIR_IF_CHANGED}
    fi
    rm -r ${SYSROOT_LIB} ${SYSROOT_LIB_HOST}
    mv ${SYSROOT_LIB}-new ${SYSROOT_LIB}
    mv ${SYSROOT_LIB_HOST}-new ${SYSROOT_LIB_HOST}
}

# Unstable features are required for building std. Also allow in the app
# project for now because they're often needed for low level embedded.
export RUSTC_BOOTSTRAP=1
# Build std
cargo ${CARGO_ARGS} \
    --target-dir=${SYSROOT_BUILD}-stage1 \
    --manifest-path=./sysroot-stage1/Cargo.toml -p std
publish_sysroot ${APP_BUILD} ${SYSROOT} ${SYSROOT_BUILD}-stage1

export RUSTFLAGS="${RUSTFLAGS} --sysroot ${SYSROOT}"
cargo ${CARGO_ARGS} --target-dir=${APP_BUILD} --manifest-path=${CARGO_MANIFEST}
