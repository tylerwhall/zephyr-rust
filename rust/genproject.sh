#!/bin/sh -ex

crate_dir=$1
outdir=$2

rm -rf $outdir
mkdir -p $outdir/src

echo "extern crate app;" > $outdir/src/lib.rs

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
EOF
