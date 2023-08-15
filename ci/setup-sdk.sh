if [ -z "$ZEPHYR_VERSION" ]; then
    echo "error: ZEPHYR_VERSION must be set"
    exit 1
fi

case $ZEPHYR_VERSION in
    2.7.*)
        SDK_VERSION=0.13.2
        SDK_URL=https://github.com/zephyrproject-rtos/sdk-ng/releases/download/v0.13.2/zephyr-sdk-0.13.2-linux-x86_64-setup.run
        ;;
    2.6.0)
        SDK_VERSION=0.12.2
        SDK_URL=https://github.com/zephyrproject-rtos/sdk-ng/releases/download/v0.12.2/zephyr-sdk-0.12.2-x86_64-linux-setup.run
        ;;
    2.5.0)
        SDK_VERSION=0.12.2
        SDK_URL=https://github.com/zephyrproject-rtos/sdk-ng/releases/download/v0.12.2/zephyr-sdk-0.12.2-x86_64-linux-setup.run
        ;;
    2.4.0)
        SDK_VERSION=0.12.2
        SDK_URL=https://github.com/zephyrproject-rtos/sdk-ng/releases/download/v0.12.2/zephyr-sdk-0.12.2-x86_64-linux-setup.run
        ;;
    2.3.0)
        SDK_VERSION=0.12.2
        SDK_URL=https://github.com/zephyrproject-rtos/sdk-ng/releases/download/v0.12.2/zephyr-sdk-0.12.2-x86_64-linux-setup.run
        ;;
    *)
        echo "Unknown zephyr version -> sdk version mapping"
        exit 1
        ;;
esac

wget ${SDK_URL} -O ./zephyr-sdk.run && chmod +x ./zephyr-sdk.run
./zephyr-sdk.run -- -d /opt/zephyr-sdk-${SDK_VERSION} && rm ./zephyr-sdk.run
