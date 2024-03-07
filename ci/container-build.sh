#!/bin/sh -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
. "${DIR}/env.sh"

set -ex

docker build -f Dockerfile.zephyr \
    --build-arg ZEPHYR_VERSION=${ZEPHYR_VERSION} \
    --build-arg RUST_VERSION=${RUST_VERSION} \
    -t ${CONTAINER_REGISTRY}${ZEPHYR_VERSION}-${RUST_VERSION} \
    .
