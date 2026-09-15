#!/bin/bash

ZEPHYR_RUST="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )/.."
# With rustup in use, `rustc --version` reports the channel pinned in
# rust-toolchain.toml (via cwd-based toolchain discovery), so this is an
# indirect read of the repo's pinned version. Note: this relies on being run
# from within the repo tree; elsewhere it falls back to the default toolchain.
RUST_VERSION="${RUST_VERSION:-$( rustc --version | awk '{print $2}' )}"
ZEPHYR_VERSION=${ZEPHYR_VERSION:-3.7.0}

# Image name/tag prefixes. LOCAL is a bare image name; REMOTE includes the
# registry. Set CONTAINER_IMAGE_PREFIX in the environment to force one of
# them (or any other prefix); this also skips the resolution below.
LOCAL_IMAGE_PREFIX="zephyr-rust:"
REMOTE_IMAGE_PREFIX="ghcr.io/tylerwhall/zephyr-rust:zephyr-rust-"
CONTAINER_TAG="${ZEPHYR_VERSION}-${RUST_VERSION}"

# Resolve CONTAINER_IMAGE, in order:
#   1. explicit CONTAINER_IMAGE_PREFIX (container-build.sh sets this so it
#      always builds unconditionally instead of resolving)
#   2. an existing local image (fast, offline check)
#   3. the remote image, pulled so later runs hit case 2 with no network
#   4. build the image locally
# Note: "local exists" does not mean "local is current"; after changing the
# Dockerfile, run `docker rmi ${LOCAL_IMAGE_PREFIX}${CONTAINER_TAG}` to force
# re-resolution.
if [ -n "${CONTAINER_IMAGE_PREFIX:-}" ]; then
    CONTAINER_IMAGE="${CONTAINER_IMAGE_PREFIX}${CONTAINER_TAG}"
elif docker image inspect "${LOCAL_IMAGE_PREFIX}${CONTAINER_TAG}" >/dev/null 2>&1; then
    CONTAINER_IMAGE="${LOCAL_IMAGE_PREFIX}${CONTAINER_TAG}"
elif docker pull "${REMOTE_IMAGE_PREFIX}${CONTAINER_TAG}" >&2; then
    CONTAINER_IMAGE="${REMOTE_IMAGE_PREFIX}${CONTAINER_TAG}"
else
    echo "no local or remote image for ${CONTAINER_TAG}; building locally" >&2
    "$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )/container-build.sh"
    CONTAINER_IMAGE="${LOCAL_IMAGE_PREFIX}${CONTAINER_TAG}"
fi

echo RUST_VERSION=$RUST_VERSION
echo ZEPHYR_VERSION=$ZEPHYR_VERSION
echo CONTAINER_IMAGE=$CONTAINER_IMAGE
