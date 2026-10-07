#!/bin/sh -e

# One entry point for image builds and cross-Clippy. Never modify the installed
# toolchain or port sources; Cargo discovers std through a build-local overlay.
: "${RUST_ENV:?Set RUST_ENV to the CMake-generated rust-env.sh}"
. "$RUST_ENV"
rust_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
base=$(rustc --print sysroot)
version=$(awk -F '"' '/^channel =/ { print $2 }' "$rust_dir/../rust-toolchain.toml")
if [ "$(rustc -vV | awk '/^release:/ { print $2 }')" != "$version" ]; then
    echo "Error: this port requires Rust $version" >&2
    exit 1
fi

root=$RUST_BUILD_TOOLCHAIN
if [ ! -f "$root/.base" ] || ! grep -Fxq "$base" "$root/.base"; then
    mkdir -p "$root/bin" "$root/lib/rustlib"
    # The compiler derives its default sysroot from librustc_driver's location.
    # Copy that library and the small drivers; symlink everything else. Calling
    # real Cargo (not the rustup proxy) avoids its original-toolchain library path.
    cp "$base/bin/rustc" "$root/bin/"
    ln -sfn "$base/bin/cargo" "$root/bin/cargo"
    for entry in "$base"/lib/*; do
        case ${entry##*/} in
            rustlib) ;;
            librustc_driver*) cp "$entry" "$root/lib/" ;;
            *) ln -sfn "$entry" "$root/lib/" ;;
        esac
    done
    for entry in "$base"/lib/rustlib/*; do
        [ "${entry##*/}" = src ] || ln -sfn "$entry" "$root/lib/rustlib/"
    done
    printf '%s\n' "$base" > "$root/.base"
fi
# Clippy may have been installed after this overlay was first created.
for driver in cargo-clippy clippy-driver; do
    if [ -f "$base/bin/$driver" ] && ! cmp -s "$base/bin/$driver" "$root/bin/$driver"; then
        cp "$base/bin/$driver" "$root/bin/"
    fi
done

# Preserve the port's relative paths without copying the source tree. Only the
# library workspace manifest and lock are writable. Std has its own resolution,
# independent of the app's libc patch and --locked policy.
src=$root/lib/rustlib/src
lib=$src/rust/library
mkdir -p "$lib"
for entry in "$rust_dir"/libc "$rust_dir"/zephyr-core "$rust_dir"/zephyr-sys; do
    ln -sfn "$entry" "$src/"
done
for entry in "$rust_dir"/rust/library/*; do
    case ${entry##*/} in
        Cargo.toml|Cargo.lock) ;;
        *) ln -sfn "$entry" "$lib/" ;;
    esac
done
cp "$rust_dir/rust/library/Cargo.toml" "$lib/Cargo.toml"
printf '\n[patch.crates-io.libc]\npath = "../../libc"\n' >> "$lib/Cargo.toml"
cp "$rust_dir/Cargo.lock" "$lib/Cargo.lock"

export RUSTC="$root/bin/rustc"
export PATH="$root/bin:$PATH"
export LD_LIBRARY_PATH="$root/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
# Cargo 1.86's build-std resolve skips writing the std lockfile and does not
# enforce --locked. Validate the complete workspace first so an incompatible
# pin cannot silently resolve to a different version during the build.
if ! "$root/bin/cargo" metadata --manifest-path "$lib/Cargo.toml" \
    --locked --format-version 1 > /dev/null; then
    echo "Error: std resolution is stale; review $lib/Cargo.toml and update rust/Cargo.lock" >&2
    exit 1
fi

command=$1
shift
rc=0
"$root/bin/cargo" "$command" --target "$RUST_TARGET_SPEC" \
    -Zbuild-std="${RUST_BUILD_STD:-std,panic_abort}" \
    -Zbuild-std-features= "$@" || rc=$?
# Retain the post-command guard as well, in case Cargo writes the std lock.
# Never accept an unreviewed resolution, including after a failed command.
if ! cmp -s "$rust_dir/Cargo.lock" "$lib/Cargo.lock"; then
    echo "Error: std resolution changed; review $lib/Cargo.lock and update rust/Cargo.lock" >&2
    diff -u "$rust_dir/Cargo.lock" "$lib/Cargo.lock" >&2 || true
    exit 1
fi
exit "$rc"
