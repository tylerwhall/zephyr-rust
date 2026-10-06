#!/bin/sh -e

crate_dir=$1
outdir=$2

rm -rf $outdir
mkdir -p $outdir/src
# Copy a Cargo.lock if it exists
cp ${crate_dir}/Cargo.lock $outdir || true

echo "extern crate app;" > $outdir/src/lib.rs
# Allocator ownership belongs to the final image, never a dual-use dependency.
if [ "${3:-}" = "--alloc-pool" ]; then
    printf '%s\n' 'extern crate zephyr_core;' \
        'zephyr_core::global_sys_mem_pool!(rust_std_mem_pool);' >> $outdir/src/lib.rs
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

[profile.release]
panic = "abort"
lto = true
debug = true
opt-level = "s"
EOF
