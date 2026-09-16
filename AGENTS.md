# Agent instructions for `zephyr-rust`

## High-level architecture

- This repository is a **Zephyr module** that enables Rust applications to be linked into Zephyr images. Applications add this module through `ZEPHYR_EXTRA_MODULES` (see sample/test `CMakeLists.txt`).
- Build flow is split between Zephyr CMake and Rust Cargo:
  1. Top-level `CMakeLists.txt` derives `rust_target`/`clang_target` from Zephyr `ARCH`/Kconfig.
  2. `scripts/gen_syscalls.py` generates syscall thunk C/header files from Zephyr syscall metadata.
  3. `zephyr-bindgen` is built and invoked by `rust/zephyr-sys/build.rs` to generate Rust FFI bindings (`bindings.rs`, `syscalls.rs`).
  4. `rust/genproject.sh` creates a generated Cargo project that depends on the app crate (from the sample/test directory).
  5. `rust/build.sh` builds a custom sysroot (`rust/sysroot-stage1`) and then builds the app staticlib (`librust_app.a`), which CMake imports and links into the Zephyr app.
- Crate layering is intentional:
  - `zephyr-sys`: generated/raw FFI and syscall bindings
  - `zephyr-core`: core/no_std-safe wrappers and context-aware syscall traits
  - `zephyr`: std-facing API layer built on `zephyr-core`
  - helper crates: `zephyr-macros`, `zephyr-futures`, `zephyr-logger`, `zephyr-uart-buffered`
- C shims (`src/main.c`) are the ABI bridge: Zephyr C entrypoints call exported Rust symbols (`extern "C"`, `#[no_mangle]`).

## Build, test, and run commands

Builds can be done using natively with local versions of rust toolchain,
zephyr-sdk, west, and the zephyr source. A container-based build environment
for CI exists in ./ci. Using the containers is preferred for performing
development/maintenance on this repo, where modifications to Zephyr source is
not required.

The default application and machine target if not specified is samples/rust-app
on qemu_x86.

### Prerequisites used by this repository
- Make sure submodules are in sync: `git submodule status --recursive`, `git diff --submodule`
  - There may be local commits above the submodule version, but the base should be the submodule rev
- Rust toolchain is pinned to the version in rust-toolchain.toml (enforced by `rust/build.sh`).
- Use a Zephyr version this repo targets (documented in `README.md`).

### Build natively
- General form: `west build -p auto -b <board> <sample-or-test-path>`
- ./samples/rust-app is the catch-all example/integration test. When run, it exits non-zero by design: it intentionally triggers a page fault at the end ("Next call will crash if userspace is working") to prove user-mode isolation. Success is the full "Hello from Rust userspace..." console output before the fatal error.
- Example for different machines:
  - `west build -p auto -b qemu_x86 samples/rust-app/`
  - `west build -p auto -b native_posix samples/rust-app/`

### Run a built QEMU/native image
- Native, from build dir: `ninja run`
- CI container, single step (build + run in one ephemeral container; `-d /tmp/build` is required because the repo is mounted read-only):
  - `cd ci && ./build-cmd.sh bash -c "west build -d /tmp/build -p auto -b qemu_x86 samples/rust-app -t run"`
- CI container, multiple steps (persist the build dir across invocations with a host volume via `DOCKER_ARGS`):
  - `cd ci && DOCKER_ARGS="-v /tmp/zr-build:/tmp/build" ./build-cmd.sh west build -d /tmp/build -p auto -b qemu_x86 samples/rust-app`
  - `cd ci && DOCKER_ARGS="-v /tmp/zr-build:/tmp/build" ./build-cmd.sh ninja -C /tmp/build run`

### Clippy
- `ci/clippy.sh` runs `cargo clippy` on all Rust crates: the host crates
  (`zephyr-bindgen`, `zephyr-macros`), the sysroot-layer crates
  (`zephyr-sys`, `zephyr-core`, `time-convert`), and every sample/test app
  crate plus the app-layer library crates. Each app is `west build`-ed in its
  own build dir first, because the cross-compiled sysroot (and the
  `zephyr-sys` bindings generated from the app's headers/devicetree/Kconfig)
  is app-specific; clippy then reuses that build's sysroot and environment.
- CI container: `cd ci && ./build-cmd.sh ci/clippy.sh`
- Natively (west, Zephyr, Zephyr SDK, and the clippy component must be
  available): `./ci/clippy.sh`
- Apps that cannot be *built* on the selected board are reported as skipped
  (some tests only build on certain Zephyr versions); set `CLIPPY_STRICT=1`
  to treat that as a failure. Warnings are not fatal by default; set
  `CLIPPY_ARGS="-D warnings"` to make them so. Other knobs: `CLIPPY_BOARD`
  (default `qemu_x86`), `CLIPPY_BUILD_DIR`, `CLIPPY_JOBS`.
- A `clippy` job in `.github/workflows/main.yml` runs this on Zephyr 3.7.0.
- Every crate used as a clippy root has a committed `Cargo.lock` (the script
  never writes to the source tree, so it works with the read-only repo mount
  used by `ci/build-cmd.sh`).

### Run tests
- Full repository tests via Zephyr sanitycheck (from `README.rst`):
  - `$ZEPHYR_BASE/scripts/sanitycheck --testcase-root tests -p native_posix -N`
- Single test (build + run one test directory):
  - `west build -p auto -b native_posix tests/semaphore`
  - `cd build && ninja run`

### CI container validation workflow
- CI images are built from `ci/Dockerfile.zephyr` and pin both toolchains via build args:
  - `ZEPHYR_VERSION=<...>` (clones `zephyrproject-rtos/zephyr` at `v<version>`)
  - `RUST_VERSION=<...>` (installs that exact toolchain with `rustup`)
- `ci/setup-sdk.sh` maps Zephyr versions to the matching Zephyr SDK version used in the container.
- GitHub Actions:
  - `.github/workflows/container-build.yml` builds/pushes per-Zephyr-version container images.
  - `.github/workflows/main.yml` runs repo builds inside those containers (Zephyr matrix over the pinned Rust image tags).
- Local reproduction of CI container workflow:
  - `ci/env.sh` defaults `RUST_VERSION` to the pinned `rustc` version and has a default for `ZEPHYR_VERSION`; override either env var only if a specific version is needed.
  - Build a container for a Zephyr/Rust combo (only needed once per combination; the image is reused by later invocations):
    - `cd ci && ./container-build.sh`
  - Run a build command inside that image:
    - `cd ci && ./build-cmd.sh west build -d /tmp/build -p auto -b qemu_x86 samples/rust-app`
  - Open an interactive shell in the same image:
    - `cd ci && ./devshell.sh`
  - A repo revision supports exactly one Rust version (the std port must match the compiler version); to port to a new one, change `RUST_VERSION` (and `rust-toolchain.toml`) and rerun `container-build.sh` + `build-cmd.sh`. The Rust port in rust/rust needs to be rebased/updated. Documentation TBD.

## Validation workflow for changes

When changing `zephyr-rust` (feature work, Rust or Zephyr version ports), validate in stages. Each stage must pass before expanding to the next; stop and fix at the first failure.

1. **Single-target smoketest (always, first)**: build + run the default sample on the default board in one container invocation (see "Run a built QEMU/native image"). Pass = clean build and full expected console output before the by-design final page fault.
2. **Expand the matrix based on the change type**, build-only where possible (add `-t run` only for runnable boards, at least on the default sample):
   - Rust version port: change `RUST_VERSION` and `rust-toolchain.toml` together and rebuild the container (only one Rust version is supported per revision, since the std port must exactly match the compiler). XXX: move this to TBD upgrade instructions.
   - Zephyr version port: verify all buildable/runnable samples and tests on the new `ZEPHYR_VERSION`, and confirm the other supported versions (see `README.md`) are not broken.
   - Feature/syscall/Kconfig changes: expand boards and samples (see `ci/build-all.sh` for the current matrix). Note: `native_posix` does not support `UART_INTERRUPT_DRIVEN`, so `samples/serial` is excluded there.
   - Run `tests/*` on `native_posix` when kernel-object or syscall behavior changed.
3. **Full matrix + tests (pre-PR / CI parity)**:
   - `cd ci && ./build-all.sh` — parallel build matrix with `--resume --halt now,fail=1` (fail fast; reruns resume past completed jobs).
   - `cd ci && ./sanitycheck.sh` — Zephyr sanitycheck over `tests/`; or individual tests via `west build -p auto -b native_posix tests/<name>` + `ninja run`.
   - Optionally mirror the GitHub Actions matrix (`.github/workflows/main.yml`) to confirm CI parity.

## Commit message style

- Short imperative subject line, no trailing period (e.g. `Update to Rust 1.75`, `ci: add M33 target`).
- No Conventional Commits types (`feat:`, `fix:`, etc.). Instead, optionally prefix with a lowercase component or area followed by a colon: `ci:`, `zephyr-core:`, `zephyr-bindgen:`, `rust-smem:`. A second-level file prefix is sometimes used, e.g. `ci: env.sh: default to Zephyr 3.7`.
- Optional body for non-trivial changes: blank line after the subject, then a longer explanation of what/why, wrapped near 72 columns.

## Key repository conventions

- Rust app/test crates are named `app` and expose C ABI symbols expected by C shims (`rust_main`, `rust_test_main`, etc.).
- Static Zephyr kernel objects from Rust are defined via `zephyr-macros` (`k_mutex_define!`, `k_sem_define!`, `k_poll_signal_define!`) so they land in Zephyr sections and are initialized via constructor hooks.
- Userspace builds commonly require `CONFIG_RUST_ALLOC_POOL=y`; sample/test `prj.conf` files are the source of truth for required Kconfig combinations.
- This codebase supports multiple Zephyr versions; preserve version guards such as `#if KERNEL_VERSION_MAJOR < 3` in C shims and generated syscall includes.
- `tests/*/testcase.yaml` defines board/platform allowlists; pick boards accordingly when running an individual test.
