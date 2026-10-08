# Cargo build-std in zephyr-rust

The production build uses Cargo 1.87 build-std, not a manually published
sysroot. CMake generates the syscall thunks, one pair of Rust binding files,
and an application staticlib project, then invokes `rust/cargo.sh` once to
build the standard-library roots and application together.

## Crate boundaries

- `zephyr-sys`: generated low-level FFI/syscall bindings.
- `zephyr-core`: context-aware wrappers usable by std and applications.
- `zephyr`: application-facing APIs and std adapters.

Cargo resolves std and application dependencies independently; matching
features do not merge their Rust crate identities. Std's core/sys instances
are private implementation details. Both instances consume image-specific
bindings and address the same C-owned resources, including the mutex pool's
atomic bitmap. The generated image root registers the global allocator once.

Do not expose private-core ownership types through std APIs. For example,
std exposes primitive Instant ticks, while `zephyr::time::instant_ticks`
converts them to the application's Ticks. Object macros use zephyr-core;
applications using those macros need a direct Cargo dependency on that crate.

## Generated FFI bindings

Bindgen's compile-time layout checks remain enabled. C `long double` fields
in `max_align_t` and `z_max_align_t` cannot be represented directly in Rust
on x86, so these types use opaque bindings that preserve C size and alignment.
CMake tracks the generator's source, manifest, and lockfile to rebuild it when
these inputs change, and builds the host tool with `--locked`.

## Toolchain and source discovery

Cargo 1.87 discovers std sources through the host compiler's sysroot at
`lib/rustlib/src/rust/library`; it does not consult RUST_LIB_SRC. The build
uses the pinned port sources, not upstream rust-src.

Each image gets a `modules/zephyr-rust/toolchain` overlay. Copied rustc/clippy
drivers and librustc_driver establish its private default sysroot; other
installed host resources are symlinked. Real Cargo is invoked directly,
avoiding the rustup proxy's original-toolchain dynamic-library path. This
relocates the compiler without introducing a compiler-argument wrapper or
modifying the installed toolchain.

Source symlinks preserve the std port's relative paths to core/sys/libc.
Only the library workspace manifest and lockfile are copied. The staged
workspace patches libc to the pinned fork. Cargo supplies std's bootstrap
compilation flags; there is no manual rlib publication or sysroot-copy step.

## Roots, features, and locks

- `CONFIG_RUST_STD=y` (default): `-Zbuild-std=std,panic_abort`.
- `CONFIG_RUST_STD=n`: `-Zbuild-std=core,alloc`; the generated root is no_std.
  All application dependencies must support no_std and the app must supply
  a panic handler. `CONFIG_RUST_ALLOC_POOL` registers the allocator at the
  image root; otherwise the application must supply one.
- `-Zbuild-std-features=` preserves the absence of optional std
  backtrace/unwind features. Generated dev and release profiles use panic=abort.

Std has its own resolution, pinned in `rust/Cargo.lock`. Cargo 1.87's
build-std resolver neither enforces --locked nor writes the std lockfile.
The helper first validates the complete staged workspace with
`cargo metadata --locked`, then compares its staged lock after every build
or Clippy invocation, including failed commands. A stale resolution fails
before compilation; unexpected lock changes report a diff. The lock includes
optional and development dependencies for this metadata check, though normal
builds still compile only the selected roots/features. Review updates in the
staged workspace before committing them; app lockfiles are independent.

## Clippy

Cross-Clippy uses `rust/cargo.sh` and the image's CMake-generated rust-env.sh,
sharing source selection, bindings, Kconfig, and the std lock guard. Host
bindgen/proc-macro crates use ordinary Cargo. CI checks west's exit status,
not merely the existence of an ELF left by an earlier build.

`zephyr-sys`, `zephyr-core`, `time-convert`, and the app-layer libraries are
each linted from their own manifests with committed lockfiles and `--locked`.
Selecting non-member dependencies with `-p` from the generated app workspace
would not apply Cargo's `RUSTC_WORKSPACE_WRAPPER`, running rustc rather than
Clippy. Standalone roots do not change the crates' workspace membership or
independent private std instances.

`ci/clippy.sh lib` builds `samples/rust-app` for its bindings and Kconfig,
then lints the low-level and app-layer libraries. The default full pass also
builds and lints every app/test root. Inside the CI container:

```sh
CLIPPY_ARGS="-D warnings" ci/clippy.sh lib
```

To lint a low-level crate against a different built image:

```sh
RUST_ENV=/tmp/build/rust-env.sh CARGO_TARGET_DIR=/tmp/clippy-target \
    rust/cargo.sh clippy --manifest-path rust/zephyr-core/Cargo.toml \
    --locked --lib -- -D warnings
```

Use `rust/zephyr-sys/Cargo.toml` or
`rust/zephyr-core/time-convert/Cargo.toml` for the other low-level roots. The
common pass checks the rust-app configuration, not every Kconfig or version
branch. Changes to cfg-gated code require runs against the corresponding
images and every affected Zephyr version. The standard application/library
pass remains required; see [AGENTS.md](../AGENTS.md#clippy) for the workflow.

### Generated-code exceptions

Bindgen's incomplete-array and bitfield helpers trigger `missing_safety_doc`,
`useless_transmute`, `transmute_int_to_bool`, and `ptr_offset_with_cast`.
Exceptions for only these four Clippy lints are scoped to the private generated
`bindings` module in `rust/zephyr-sys/src/lib.rs`. Reexports preserve the public
`raw` API. Handwritten kernel object wrappers and all of `zephyr-core` retain
full lint coverage; there is no crate-wide Clippy suppression. Recheck these
exceptions when updating bindgen.

## Known limitations

- Userspace heap allocation through `MempoolAlloc` is not established as safe:
  a Zephyr 3.7.0 allocation probe faulted in its privileged `arch_irq_lock`
  call. `samples/no_std` covers kernel allocation and user syscalls, not safe
  userspace heap allocation.
- Tests on Zephyr 2.7.3/3.7.0 remain build-only. Twister integration is tracked
  in [BUILD_MATRIX_TODO.md](BUILD_MATRIX_TODO.md); automated test execution
  currently uses Zephyr 2.3.0 sanitycheck.
