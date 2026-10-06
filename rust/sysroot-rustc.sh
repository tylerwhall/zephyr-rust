#!/bin/sh -e

# Rust's bootstrap uses -Zforce-unstable-if-unmarked for std dependencies.
# Since Rust 1.85, this is required to record hashbrown's
# #[rustc_const_stable_indirect] annotations for std's stable const constructors.
# Apply it to hashbrown and its std consumer, as in bootstrap. Zephyr's public
# sysroot crates must remain usable without #![feature(rustc_private)] in apps.
rustc=$1
shift
previous=
for arg in "$@"; do
    if [ "$previous" = "--crate-name" ]; then
        case "$arg" in
            std|hashbrown)
                exec "$rustc" -Zforce-unstable-if-unmarked "$@"
                ;;
        esac
    fi
    previous=$arg
done
exec "$rustc" "$@"
