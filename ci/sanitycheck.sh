#!/bin/bash

# testcase.yaml schema changed after 2.3 :(
ZEPHYR_VERSION=2.3.0

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

rm -rf sanity-out
DOCKER_ARGS=(-e ZEPHYR_TOOLCHAIN_VARIANT=zephyr -v ${DIR}/sanity-out:/sanity-out)

. "${DIR}/build-cmd.sh" sh -c "\$ZEPHYR_BASE/scripts/sanitycheck -N -O /sanity-out/out -c --all -T /zephyr-rust/tests"
