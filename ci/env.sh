#!/bin/bash

ZEPHYR_RUST="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )/.."
RUST_VERSION="$( rustc --version | awk '{print $2}' )"
ZEPHYR_VERSION=${ZEPHYR_VERSION:-2.4.0}
# Set CONTAINER_REGISTRY to something like "zephyr-rust:" to use local images
CONTAINER_REGISTRY=${CONTAINER_REGISTRY:-ghcr.io/tylerwhall/zephyr-rust:zephyr-rust-}

echo RUST_VERSION=$RUST_VERSION
echo ZEPHYR_VERSION=$ZEPHYR_VERSION
