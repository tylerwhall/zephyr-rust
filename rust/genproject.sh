#!/bin/sh -e

crate_dir=$1
outdir=$2
rust_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

rm -rf $outdir
mkdir -p $outdir/src
# Copy a Cargo.lock if it exists
cp ${crate_dir}/Cargo.lock $outdir || true

alloc_pool=0
no_std=0
shift 2
for arg in "$@"; do
    case $arg in
        --no-std) no_std=1 ;;
        --alloc-pool) alloc_pool=1 ;;
        *) echo "Unknown option: $arg" >&2; exit 1 ;;
    esac
done
: > "$outdir/src/lib.rs"
if [ "$no_std" = 1 ]; then
    echo '#![no_std]' >> "$outdir/src/lib.rs"
fi
echo 'extern crate app;' >> "$outdir/src/lib.rs"
if [ "$alloc_pool" = 1 ]; then
    # Register once at the image root, not in a dual-use dependency.
    printf '%s\n' 'extern crate zephyr_core;' \
        'zephyr_core::global_sys_mem_pool!(rust_std_mem_pool);' >> "$outdir/src/lib.rs"
fi

cat > $outdir/Cargo.toml <<EOF
[package]
name = "rust-app"
version = "0.1.0"
edition = "2018"

[lib]
crate-type = ["staticlib"]

[dependencies]
app = { path = "${crate_dir}" }
zephyr-core = { path = "${rust_dir}/zephyr-core" }

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
lto = true
debug = true
opt-level = "s"
EOF
