# Low-level crate Clippy coverage

The low-level Clippy coverage gap is closed. `zephyr-sys`, `zephyr-core`, and
`time-convert` are ordinary app dependencies as well as private std
dependencies. `ci/clippy.sh` now lints each from its own manifest with a
committed Cargo.lock, `--locked`, and the image's `RUST_ENV` through
`rust/cargo.sh clippy`.

Previously the common pass selected these crates with `-p` from the generated
app workspace. As non-member dependencies, they did not receive Cargo's
`RUSTC_WORKSPACE_WRAPPER`, so only rustc lints ran. Explicitly linting their
app-facing dependents did not provide genuine low-level Clippy coverage.
Standalone manifests make each low-level crate a Clippy root without changing
its workspace membership or its private std dependency instance.

## Running the coverage

`ci/clippy.sh lib` builds `samples/rust-app` to supply image-specific bindings
and Kconfig, then lints the low-level and app-layer library roots. The default
full pass also builds and lints every app/test root. Host bindgen/proc-macro
crates use ordinary Cargo; cross-Clippy retains the build-std toolchain/source
overlay and independent std lock guard. No custom-sysroot workaround is needed.

For example, inside the CI container:

```sh
CLIPPY_ARGS="-D warnings" ci/clippy.sh lib
```

To check a low-level crate against a different built image:

```sh
RUST_ENV=/tmp/build/rust-env.sh CARGO_TARGET_DIR=/tmp/clippy-target \
    rust/cargo.sh clippy --manifest-path rust/zephyr-core/Cargo.toml \
    --locked --lib -- -D warnings
```

Use `rust/zephyr-sys/Cargo.toml` or
`rust/zephyr-core/time-convert/Cargo.toml` for the other roots. The common
pass checks the rust-app configuration, not every possible Kconfig or version
branch. Changes to cfg-gated low-level code still require runs against the
corresponding images and all affected Zephyr versions. The standard
application/library pass remains required.

## Generated bindings exceptions

Bindgen is updated to 0.72.1. Its generated incomplete-array and bitfield
helpers still trigger `missing_safety_doc`, `useless_transmute`,
`transmute_int_to_bool`, and `ptr_offset_with_cast`. Approved exceptions for
only these four Clippy lints
are scoped to the private generated `bindings` module in
`rust/zephyr-sys/src/lib.rs`. Reexports preserve the public `raw` API.
Handwritten kernel object wrappers and all of `zephyr-core` retain full lint
coverage; there is no crate-wide Clippy suppression.

## Resolved Rust 1.85 inventory

The initial standalone, `--locked` Clippy pass against the Zephyr 3.7.0
qemu_x86 rust-app image found:

- `time-convert`: no warnings.
- `zephyr-core`: `crate_in_macro_def`, `missing_safety_doc`,
  `needless_lifetimes`, `needless_borrow`, and `useless_conversion`.
- `zephyr-sys`: the generated helper warnings listed above.
- Libc's Zephyr module: the stale `libc_core_cvoid` `unexpected_cfgs` warning.

Fixes were committed per lint type. Static wrapper macros resolve helpers
through `$crate`; unsafe public APIs document initialization, lifetime,
context, ownership, and locking contracts. Redundant lifetimes, borrows, and
identity conversions were removed after checking the supported Zephyr APIs.
Libc now uses its shared `core::ffi::c_void` rather than the obsolete
conditional Zephyr definition.

Bindgen 0.72's compile-time layout checks also exposed C `long double` fields
in `max_align_t` and `z_max_align_t` that Rust cannot represent directly on
x86. These types now use opaque bindings that preserve C size and alignment;
layout checks remain enabled. CMake tracks the generator's source, manifest,
and lock so incremental builds cannot retain an old binding generator.

## Validation completed

Before enabling the standalone roots in warning-fatal CI, validation passed:

- Standalone low-level Clippy with `-D warnings` and rust-app builds on
  qemu_x86 with Zephyr 2.3.0, 2.7.3, and 3.7.0.
- The fresh 113-job matrix across all supported versions and boards, including
  six verified expected-fault sample runs (`RUN=1 ci/build-all.sh`).
- All seven Zephyr 2.3.0 sanitycheck configurations, executing tests on
  qemu_x86 and qemu_cortex_m3.
- The full standard application/library Clippy pass on Zephyr 3.7.0, plus
  `ci/clippy.sh lib` on 2.7.3 and 2.3.0, with `CLIPPY_ARGS="-D warnings"`
  and strict build failure handling.

Verbose Cargo traces confirmed `clippy-driver` invocations for `zephyr_sys`,
`zephyr_core`, and `time_convert`. Isolated copies of each root rejected an
injected handwritten unsafe function without safety docs, including a probe
inside `zephyr-sys::raw` outside the generated module. Each also rejected an
intentionally stale standalone lockfile under `--locked`. The production
source tree was unchanged by these probes; the independent std lock guard
remained active throughout.

See `AGENTS.md` for the ongoing staged validation workflow.
