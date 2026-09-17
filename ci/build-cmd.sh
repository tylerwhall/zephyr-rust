#!/bin/bash

# Run a command inside the CI container with the repo mounted at /zephyr-rust.
# The mount is read-only by default so container runs never write to the
# source tree. Runs that need write access (cargo clippy --fix, regenerating
# a Cargo.lock) can opt in with WRITABLE=1.

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
. "${DIR}/env.sh"

set -ex

MOUNT_OPTS="ro"
if [ "${WRITABLE:-0}" = "1" ]; then
    MOUNT_OPTS="rw"
fi

exec docker run \
    -i --rm \
    -v ${DIR}/..:/zephyr-rust:${MOUNT_OPTS} \
    -w /zephyr-rust \
    ${DOCKER_ARGS[@]} \
    ${CONTAINER_IMAGE} \
    "$@"