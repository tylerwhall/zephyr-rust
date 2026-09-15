#!/bin/sh -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
# Always build, unconditionally: setting CONTAINER_IMAGE_PREFIX skips the
# image resolution in env.sh. This must match LOCAL_IMAGE_PREFIX in env.sh so
# the built image is found by resolution on later runs.
CONTAINER_IMAGE_PREFIX="${CONTAINER_IMAGE_PREFIX:-zephyr-rust:}"
. "${DIR}/env.sh"

set -ex

docker build -f Dockerfile.zephyr \
    --build-arg ZEPHYR_VERSION=${ZEPHYR_VERSION} \
    --build-arg RUST_VERSION=${RUST_VERSION} \
    -t ${CONTAINER_IMAGE} \
    .
