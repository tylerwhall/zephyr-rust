# Low-level crate Clippy coverage

`zephyr-sys`, `zephyr-core`, and `time-convert` are ordinary app dependencies
as well as private std dependencies. The common pass in `ci/clippy.sh` selects
them with `-p` from the generated app workspace. They are not workspace
members, so Cargo does not apply Clippy's `RUSTC_WORKSPACE_WRAPPER` to them:
that selection runs rustc and does not provide genuine Clippy coverage.

The app-facing library roots and app/test roots are linted explicitly, but
that does not make their non-member dependencies Clippy roots either.

## Remaining work

1. Give each low-level crate a committed Cargo.lock and lint it from its own
   manifest (or deliberately introduce a lint workspace). Their current
   gitignore policy excludes these standalone locks.
2. Invoke those roots through `rust/cargo.sh clippy` with an image's RUST_ENV,
   retaining --locked and the independent std lock guard. The build-std
   toolchain overlay supplies host std; no custom-sysroot workaround is needed.
3. Inventory the current warnings before changing CI. Fix them per warning
   type, with safety documentation for unsafe APIs and careful macro hygiene.
   Do not carry forward the old Rust 1.75 warning counts or blindly apply
   --fix: conversions and kernel-object code depend on the Zephyr version.
4. Validate fixes on every affected Zephyr version, then the full matrix,
   before making the new lint roots warning-fatal in CI.

No new lint suppression was introduced by the build-std migration. This
remaining coverage gap must not be described as a clean Clippy pass for the
low-level crates. The standard application/library pass remains required.
