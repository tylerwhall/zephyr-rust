# Cargo build-std in zephyr-rust

The production build uses Cargo 1.85 build-std, not a manually published
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

## Toolchain and source discovery

Cargo 1.85 discovers std sources through the host compiler's sysroot at
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

Std has its own resolution, pinned in `rust/Cargo.lock`. Cargo 1.85 does not
fully enforce --locked on that workspace, so the helper compares its staged
lock after every invocation, including failed commands. Unexpected changes
fail the build and report a diff. Review that staged resolution before
updating the committed lock; app lockfiles are independent.

## Clippy

Cross-Clippy uses `rust/cargo.sh` and the image's CMake-generated rust-env.sh,
sharing source selection, bindings, Kconfig, and the std lock guard. Host
bindgen/proc-macro crates use ordinary Cargo. CI checks west's exit status,
not merely the existence of an ELF left by an earlier build.

Core/sys/time-convert and the app-layer libraries are each linted from their
own manifests with committed lockfiles. Selecting them as non-members from
the generated app workspace would run rustc rather than real Clippy. See
[CLIPPY_SYSROOT_DEBT.md](CLIPPY_SYSROOT_DEBT.md) for coverage, generated-code
exceptions, and the warning inventory resolved when enabling these roots.

## Validation and limitations

The migration passed the 113-job matrix across Zephyr 2.3.0/2.7.3/3.7.0,
including six expected-fault sample runs, all seven 2.3.0 sanitycheck
configurations, and strict 3.7.0 Clippy. `samples/no_std` genuinely omits
target libstd; its generated root also passes dev-profile check/Clippy.
Regression checks cover std lock mutation and failed west builds with stale
ELFs. Later-version test execution still requires the twister work described
in [BUILD_MATRIX_TODO.md](BUILD_MATRIX_TODO.md).

The dedicated allocator's userspace allocation path remains a separate runtime
issue: an extra Box test on 3.7.0 faulted in MempoolAlloc's privileged
arch_irq_lock call. The no_std sample validates kernel allocation and user
syscalls, not safe userspace heap allocation. See
[rust-upgrade-history.md](rust-upgrade-history.md) for version-port history.
