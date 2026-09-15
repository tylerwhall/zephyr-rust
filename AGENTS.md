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
- Use a Zephyr version this repo targets (documented in `README.md`: 2.3.0, 2.7.3, 3.7.0).

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
  - `.github/workflows/main.yml` runs repo builds inside those containers (currently Zephyr matrix with Rust 1.75.0 image tag).
- Local reproduction of CI container workflow:
  - `ci/env.sh` defaults `RUST_VERSION` to the pinned `rustc` version and has a default for `ZEPHYR_VERSION`; override either env var only if a specific version is needed.
  - Build a container for a Zephyr/Rust combo:
    - `cd ci && ./container-build.sh`
  - Run a build command inside that image:
    - `cd ci && ./build-cmd.sh west build -d /tmp/build -p auto -b qemu_x86 samples/rust-app`
  - Open an interactive shell in the same image:
    - `cd ci && ./devshell.sh`
  - Validate additional Rust versions by changing only `RUST_VERSION` while keeping the same Zephyr version and rerunning `container-build.sh` + `build-cmd.sh`.

## Key repository conventions

- Rust app/test crates are named `app` and expose C ABI symbols expected by C shims (`rust_main`, `rust_test_main`, etc.).
- Static Zephyr kernel objects from Rust are defined via `zephyr-macros` (`k_mutex_define!`, `k_sem_define!`, `k_poll_signal_define!`) so they land in Zephyr sections and are initialized via constructor hooks.
- Userspace builds commonly require `CONFIG_RUST_ALLOC_POOL=y`; sample/test `prj.conf` files are the source of truth for required Kconfig combinations.
- This codebase supports multiple Zephyr versions; preserve version guards such as `#if KERNEL_VERSION_MAJOR < 3` in C shims and generated syscall includes.
- `tests/*/testcase.yaml` defines board/platform allowlists; pick boards accordingly when running an individual test.
