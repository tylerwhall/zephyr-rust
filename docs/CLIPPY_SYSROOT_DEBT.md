# Low-level crate Clippy coverage

`zephyr-sys`, `zephyr-core`, and `time-convert` are ordinary app dependencies
as well as private std dependencies. The common pass in `ci/clippy.sh` selects
them with `-p` from the generated app workspace. They are not workspace
members, so Cargo does not apply Clippy's `RUSTC_WORKSPACE_WRAPPER` to them:
that selection runs rustc and does not provide genuine Clippy coverage.

The app-facing library roots and app/test roots are linted explicitly, but
that does not make their non-member dependencies Clippy roots either.

## Remaining work

1. Standalone Cargo.lock files are now committed for each low-level crate.
   Switch the common pass to their own manifests after fixing the inventory
   below; selecting non-members with `-p` is still not genuine coverage.
2. Invoke those roots through `rust/cargo.sh clippy` with an image's RUST_ENV,
   retaining --locked and the independent std lock guard. The build-std
   toolchain overlay supplies host std; no custom-sysroot workaround is needed.
3. Inventory the current warnings before changing CI. Fix them per warning
   type, with safety documentation for unsafe APIs and careful macro hygiene.
   Do not carry forward the old Rust 1.75 warning counts or blindly apply
   --fix: conversions and kernel-object code depend on the Zephyr version.
4. Validate fixes on every affected Zephyr version, then the full matrix,
   before making the new lint roots warning-fatal in CI.

## Rust 1.85 inventory

A standalone, `--locked` Clippy pass against the Zephyr 3.7.0 qemu_x86
rust-app image found:

- `time-convert`: no warnings.
- `zephyr-core`: `crate_in_macro_def`, `missing_safety_doc`,
  `needless_lifetimes`, `needless_borrow`, and `useless_conversion`.
- `zephyr-sys`: generated bindgen helpers trigger `missing_safety_doc`,
  `useless_transmute`, and `transmute_int_to_bool`.
- The standalone resolution also exposes libc's `libc_core_cvoid`
  `unexpected_cfgs` warning, unlike the generated app's patched resolution.

No new lint suppression was introduced by the build-std migration. This
remaining coverage gap must not be described as a clean Clippy pass for the
low-level crates. The standard application/library pass remains required.
