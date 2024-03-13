#!/bin/bash

targets=(
i686-unknown-zephyr
riscv32imac-unknown-zephyr-elf
# riscv32ima-unknown-zephyr-elf # need to update script to generically handle RISCV extensions. Manually update from above for now.
riscv32imc-unknown-zephyr-elf
riscv64imac-unknown-zephyr-elf
thumbv7em-zephyr-eabihf
thumbv7em-zephyr-eabi
thumbv7m-zephyr-eabi
thumbv7r-zephyr-eabihf
thumbv7r-zephyr-eabi
)

for target in "${targets[@]}"; do
    # Replace "zephyr" with "none"
    rust_target="${target/zephyr/none}"

    # Special case mapping for i686-unknown-none
    case $rust_target in
    i686-unknown-none)
        rust_target=i686-unknown-linux-gnu
        extra_filter='| .["features"] = "-mmx,-sse,+soft-float"'
        ;;
    thumbv7r-*)
        # Rust does not have a thumbv7 target. Use armv7 and add thumb features
        rust_target=${rust_target/thumbv7/armv7}
        extra_filter='| .["features"] += ",+v7,+thumb-mode,+thumb2,+rclass"'
        ;;
    *)
        extra_filter=""
        ;;
    esac
    echo "Updating $target from $rust_target"
    # Set target-family and os to zephyr
    # Remove is-builtin, linker, linker-flavor
    filter='.["target-family"] = "zephyr" |
        .["os"] = "zephyr" |
        .["position-independent-executables"] = "false" |
        .["relocation-model"] = "static" |
        .["dynamic-linking"] = "false" |
        .["has-thread-local"] = "false" |
        del(.["is-builtin"])'
    filter="$filter $extra_filter"
    RUSTC_BOOTSTRAP=1 rustc --print target-spec-json -Z unstable-options --target $rust_target |
        jq "$filter" > $target.json
done
