#!/bin/bash

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
. "${DIR}/env.sh"

set -ex

exec docker run \
    -i --rm \
    -v ${DIR}/..:/zephyr-rust:ro \
    -w /zephyr-rust \
    ${DOCKER_ARGS[@]} \
    zephyr-rust:${ZEPHYR_VERSION}-${RUST_VERSION} \
    "$@"
