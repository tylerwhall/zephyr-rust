#!/bin/bash

ZEPHYR_RUST="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )/.."
RUST_VERSION=$(<$ZEPHYR_RUST/rust-toolchain)
ZEPHYR_VERSION=${ZEPHYR_VERSION:-2.4.0}

case $ZEPHYR_VERSION in
    2.5.0)
        SDK_VERSION=0.12.2
        SDK_URL=https://github.com/zephyrproject-rtos/sdk-ng/releases/download/v0.12.2/zephyr-sdk-0.12.2-x86_64-linux-setup.run
        ;;
    2.4.0)
        # Actual version is 0.11.4, but this works and allows common container images
        SDK_VERSION=0.12.2
        SDK_URL=https://github.com/zephyrproject-rtos/sdk-ng/releases/download/v0.12.2/zephyr-sdk-0.12.2-x86_64-linux-setup.run
        ;;
    2.3.0)
        # Actual version is 0.11.3, but this works and allows common container images
        SDK_VERSION=0.12.2
        SDK_URL=https://github.com/zephyrproject-rtos/sdk-ng/releases/download/v0.12.2/zephyr-sdk-0.12.2-x86_64-linux-setup.run
        ;;
    *)
        echo "Unknown zephyr version -> sdk version mapping"
        exit 1
        ;;
esac

echo RUST_VERSION=$RUST_VERSION
echo ZEPHYR_VERSION=$ZEPHYR_VERSION
echo SDK_VERSION=$SDK_VERSION
