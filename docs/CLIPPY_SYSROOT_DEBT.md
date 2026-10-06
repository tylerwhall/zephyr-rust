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
3. The Rust 1.85 inventory below has been reproduced and the handwritten
   crate warnings fixed per lint type. Resolve the remaining generated
   bindgen warnings before changing CI. Updating bindgen from 0.69 to 0.71
   was tested and did not remove these warnings; that experiment was reverted.
4. Validate fixes on every affected Zephyr version, then the full matrix,
   before making the new lint roots warning-fatal in CI.

## Rust 1.85 inventory

The initial standalone, `--locked` Clippy pass against the Zephyr 3.7.0
qemu_x86 rust-app image found:

- `time-convert`: no warnings.
- `zephyr-core`: `crate_in_macro_def`, `missing_safety_doc`,
  `needless_lifetimes`, `needless_borrow`, and `useless_conversion`.
- `zephyr-sys`: generated bindgen helpers trigger `missing_safety_doc`,
  `useless_transmute`, and `transmute_int_to_bool`.
- Libc's Zephyr module also exposed the stale `libc_core_cvoid`
  `unexpected_cfgs` warning.

The handwritten `zephyr-core` warnings are now fixed: static wrapper macros
resolve helpers through `$crate`, unsafe public APIs document their safety
contracts, and redundant lifetimes, borrows, and identity conversions are
removed. Each fix passed rust-app builds and standalone Clippy with the
respective lint denied on qemu_x86 with Zephyr 2.3.0, 2.7.3, and 3.7.0.
Libc now uses its shared `core::ffi::c_void` definition rather than the stale
conditional Zephyr definition. Standalone `zephyr-core` Clippy passes with
`-D warnings` on all three versions.

`zephyr-sys`'s generated helper warnings remain unresolved. The common pass
has not yet switched to standalone manifests and must not be described as
providing genuine low-level coverage. Full matrix validation and the standard
application/library pass are still required before enabling the new CI roots.

No new lint suppression was introduced by the build-std migration. This
remaining coverage gap must not be described as a clean Clippy pass for the
low-level crates. The standard application/library pass remains required.
