#!/bin/bash

ZEPHYR_RUST="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )/.."
RUST_VERSION="$( rustup show active-toolchain | awk '{print $1}' )"
ZEPHYR_VERSION=${ZEPHYR_VERSION:-2.4.0}

echo RUST_VERSION=$RUST_VERSION
echo ZEPHYR_VERSION=$ZEPHYR_VERSION
