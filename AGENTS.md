# Agent instructions for `zephyr-rust`

## High-level architecture

- This repository is a **Zephyr module** that enables Rust applications to be linked into Zephyr images. Applications add this module through `ZEPHYR_EXTRA_MODULES` (see sample/test `CMakeLists.txt`).
- Build flow is split between Zephyr CMake and Rust Cargo:
  1. Top-level `CMakeLists.txt` derives `rust_target`/`clang_target` from Zephyr `ARCH`/Kconfig.
  2. `scripts/gen_syscalls.py` generates syscall thunk C/header files from Zephyr syscall metadata.
  3. CMake builds and invokes `zephyr-bindgen` once per Zephyr image to generate Rust FFI bindings (`bindings.rs`, `syscalls.rs`). `zephyr-sys/build.rs` tracks those shared inputs for both std-private and ordinary app crate instances.
  4. `rust/genproject.sh` creates a generated Cargo project that depends on the app crate (from the sample/test directory).
  5. `rust/cargo.sh` stages a build-local toolchain/source overlay and builds std plus the app staticlib (`librust_app.a`) together with Cargo build-std. CMake imports and links that archive; no compiled sysroot is published.
- Crate layering is intentional:
  - `zephyr-sys`: generated/raw FFI and syscall bindings
  - `zephyr-core`: core/no_std-safe wrappers and context-aware syscall traits
  - `zephyr`: std-facing API layer built on `zephyr-core`
  - helper crates: `zephyr-macros`, `zephyr-futures`, `zephyr-logger`, `zephyr-uart-buffered`
- C shims (`src/main.c`) are the ABI bridge: Zephyr C entrypoints call exported Rust symbols (`extern "C"`, `#[no_mangle]`).
- Std and apps compile independent instances of `zephyr-core`/`zephyr-sys`. Kernel resources and mutex-pool bookkeeping are C-owned; register global allocators only at the generated app root. Adapt `Instant` with `zephyr::time::instant_ticks`, not the removed std-private `From<Instant>` implementation. `CONFIG_RUST_STD=n` selects core/alloc-only builds; the app must provide a panic handler and an allocator (or select `RUST_ALLOC_POOL`). See `docs/BUILD_STD.md` for source staging and std lockfile enforcement.

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
- Rust toolchain is pinned to the version in rust-toolchain.toml (enforced by `rust/cargo.sh`). Std's independent resolution is pinned in `rust/Cargo.lock`; the helper rejects mutation of its staged copy.
- Use a Zephyr version this repo targets (documented in `README.md`).

### Build natively
- General form: `west build -p auto -b <board> <sample-or-test-path>`
- ./samples/rust-app is the catch-all example/integration test. When run, it exits non-zero by design: it intentionally triggers a page fault at the end ("Next call will crash if userspace is working") to prove user-mode isolation. Success is the full "Hello from Rust userspace..." console output before the fatal error.
- Example for different machines:
  - `west build -p auto -b qemu_x86 samples/rust-app/`
  - `west build -p auto -b native_posix samples/rust-app/`

### Run a built QEMU/native image
- `ninja run` can leave QEMU running after output has stopped; do not use a
  bare timeout pipeline as the cleanup mechanism. In CI, use
  `ci/run-sample.sh`, which launches the emulator in its own process group
  and kills the full group on timeout or exit.
- The only samples verified to exit automatically are `samples/rust-app` and
  `samples/no_std` on `qemu_x86`, across Zephyr 2.3.0, 2.7.3, and 3.7.0.
  Both intentionally trigger a user-mode page fault, print
  `Next call will crash if userspace is working.`, and make `ninja run`
  return status 1. CI asserts that output separately from the expected
  non-zero status. All other QEMU sample combinations failed to exit during
  the 10-second (Zephyr 2.7.3/3.7.0) or 30-second (2.3.0) inventory window;
  `samples/serial` waits for input. Leave those build-only. Tests
  also remain build-only here; see the sanitycheck/twister workflow below.
- CI container, single step (build + run in one ephemeral container; `-d /tmp/build` is required because the repo is mounted read-only):
  - `cd ci && RUST_VERSION=1.88.0 ZEPHYR_VERSION=3.7.0 ./build-cmd.sh bash -c 'west build -d /tmp/build -p auto -b qemu_x86 samples/rust-app && bash ci/run-sample.sh'`
- CI container, multiple steps (persist the build dir across invocations with a host volume via `DOCKER_ARGS`):
  - `cd ci && DOCKER_ARGS="-v /tmp/zr-build:/tmp/build" ./build-cmd.sh west build -d /tmp/build -p auto -b qemu_x86 samples/rust-app`
  - `cd ci && DOCKER_ARGS="-v /tmp/zr-build:/tmp/build" ./build-cmd.sh bash ci/run-sample.sh`

### Clippy
- `ci/clippy.sh` runs `cargo clippy` on all Rust crates: the host crates
  (`zephyr-bindgen`, `zephyr-macros`), every sample/test app crate, and the
  app-layer library crates, and the low-level crates (`zephyr-sys`,
  `zephyr-core`, `time-convert`). Every library is linted from its own manifest
  with a committed lockfile, so Clippy checks it as a root rather than a
  non-member dependency. See `docs/BUILD_STD.md#clippy` for
  coverage and generated-code exceptions. Each app is `west build`-ed in its
  own build dir first for image-specific bindings and Kconfig. Cross-Clippy uses `rust/cargo.sh` with the same build-std roots,
  source overlay, and std lock guard; host crates use ordinary Cargo.
- CI container: `cd ci && ./build-cmd.sh ci/clippy.sh`
- Natively (west, Zephyr, Zephyr SDK, and the clippy component must be
  available): `./ci/clippy.sh`
- Select what to lint with positional arguments: `ci/clippy.sh` (everything,
  the CI default), `ci/clippy.sh lib` (only the common library crates),
  `ci/clippy.sh serial` (one app, by `samples/`/`tests/` dir name or path),
  or any combination like `ci/clippy.sh lib serial`.
- Apps that cannot be *built* on the selected board fail the run by default
  (some tests only build on certain Zephyr versions); set `CLIPPY_STRICT=0`
  to report them as skipped instead. Warnings are not fatal by default; set
  `CLIPPY_ARGS="-D warnings"` to make them so. Other knobs: `CLIPPY_BOARD`
  (default `qemu_x86`), `CLIPPY_BUILD_DIR`, `CLIPPY_JOBS`.
- cfg'd-out code is not type-checked, so clippy on one Zephyr version does
  not check the other versions' `zephyrNNN` cfg branches of an app crate;
  lint per version when an app gates code on the Zephyr version.
- A `clippy` job in `.github/workflows/main.yml` runs this on Zephyr 3.7.0
  with `CLIPPY_ARGS="-D warnings"`, so new warnings fail CI.
- Every crate used as a clippy root has a committed `Cargo.lock`, enforced
  by running every clippy with `--locked`; a stale lock fails with cargo's
  "needs to be updated" error instead of being rewritten. The repo is
  mounted read-only by default; use `WRITABLE=1` with `ci/build-cmd.sh` for
  runs that need write access (e.g. regenerating a
  `Cargo.lock`).

#### Fixing clippy warnings

Work through native crates, then libraries (`ci/clippy.sh lib`), then
apps/tests, keeping each stage clean before moving on. One commit per
warning type, quoting a sample of the clippy output in the body. For each
type: reproduce with clippy, fix, verify, commit. Use
`ci/clippy-fix.sh` (in the CI container) to run the pass and get the
remaining warnings grouped by lint. Details:

- Persist the build dir across runs so re-runs are incremental:
  `cd ci && DOCKER_ARGS="-v /tmp/zr-clippy:/tmp/zephyr-rust-clippy" ./build-cmd.sh ci/clippy.sh lib`
- Re-lint one crate without a full pass using the build's environment, e.g.
  `./build-cmd.sh sh -c 'RUST_ENV=/tmp/zephyr-rust-clippy/rust-app/rust-env.sh CARGO_TARGET_DIR=/tmp/zephyr-rust-clippy/cargo-target rust/cargo.sh clippy --manifest-path rust/zephyr/Cargo.toml --locked --lib'`
- A stale `Cargo.lock` reported by `--locked` is regenerated with
  `cargo generate-lockfile --manifest-path <crate>/Cargo.toml` (or
  `cargo update` for dependency bumps) and committed with the change.
  `generate-lockfile` ignores the existing lock and bumps everything
  to latest-compatible: regenerate only roots whose graph actually
  changed (e.g. the libc path dep) and revert churn in unaffected
  ones.
- Add `#[allow(...)]` (with a justifying comment) only when a clean fix is
  impossible; ALWAYS stop and ask the user first when allowing a warning/lint.
- Default lint validation uses `qemu_x86`. Native_posix/3.7.0 is excluded
  because the current picolibc C compilation fails before Rust, not because
  build-std lacks target std; see `docs/BUILD_MATRIX_TODO.md`.

### Run tests
- Automated test execution: `cd ci && RUST_VERSION=1.88.0 ./sanitycheck.sh` —
  Zephyr 2.3.0 sanitycheck over `tests/`, executing on qemu_x86 and
  qemu_cortex_m3. Other boards/versions are build-only: the 2.3.0 runner
  hardcodes `-Werror`/`-Wl,--fatal-warnings`, which fails on the native_posix
  kernel's noinit attribute warning and the cortex_r5 DT_TEXTREL link
  warning, and riscv boards are Zephyr 3.x-only. See the header comment in
  `ci/sanitycheck.sh`; 2.7.3/3.7.0 execution is tracked as twister work in
  `docs/BUILD_MATRIX_TODO.md`.
- Single test (build one test directory; `ninja run` prints the output but
  the emulator does not exit):
  - `west build -p auto -b native_posix tests/semaphore`
  - `cd build && ninja run`

### CI container validation workflow
- CI images are built from `ci/Dockerfile.zephyr` and pin both toolchains via build args:
  - `ZEPHYR_VERSION=<...>` (clones `zephyrproject-rtos/zephyr` at `v<version>`)
  - `RUST_VERSION=<...>` (installs that exact toolchain with `rustup`)
- `ci/setup-sdk.sh` maps Zephyr versions to the matching Zephyr SDK version used in the container.
- GitHub Actions:
  - `.github/workflows/container-build.yml` builds/pushes per-Zephyr-version container images.
  - `.github/workflows/main.yml` runs repo builds inside those containers; the build matrix is generated by `ci/matrix.py`, so `ci/build-all.sh` reproduces CI's jobs locally.
- Local reproduction of CI container workflow:
  - `ci/env.sh` defaults `RUST_VERSION` to the pinned `rustc` version and has a default for `ZEPHYR_VERSION`; override either env var only if a specific version is needed.
  - Build a container for a Zephyr/Rust combo (only needed once per combination; the image is reused by later invocations):
    - `cd ci && ./container-build.sh`
  - Run a build command inside that image:
    - `cd ci && ./build-cmd.sh west build -d /tmp/build -p auto -b qemu_x86 samples/rust-app`
  - Open an interactive shell in the same image:
    - `cd ci && ./devshell.sh`
  - A repo revision supports exactly one Rust version; the Rust port in
    rust/rust must be rebased onto the new release tag. Full upgrade
    instructions: `docs/rust-upgrade.md` (history in
    `docs/rust-upgrade-history.md`).

## Validation workflow for changes

When changing `zephyr-rust` (feature work, Rust or Zephyr version ports),
validate in stages. Each stage must pass before expanding to the next; stop
and fix at the first failure. The build matrix — per-app board whitelists
from `testcase.yaml`/`sample.yaml`, version exclusions, and which samples
run — is single-sourced in `ci/matrix.py`; `ci/build-all.sh` runs exactly
the jobs `.github/workflows/main.yml` builds.

1. **Single-target smoketest (always, first)**: build the default sample on
   the default board and run it with the process-group-safe runner
   (`ci/run-sample.sh`), per "Run a built QEMU/native image". Pass: clean
   build, console output through `Next call will crash if userspace is
   working.`, then exit status 1 from the intentional user-mode page fault.
   That behavior is verified only for `samples/rust-app` and `samples/no_std`
   on `qemu_x86` (all three Zephyr versions); every other combination leaves
   the emulator running, so the runner's process-group cleanup is required,
   never a bare `ninja run`/`timeout` pipeline.
2. **Expand the matrix by app**: build the affected apps across their
   whitelists and all Zephyr versions with
   `cd ci && APPS=<app>... ./build-all.sh` (trim further with `BOARDS=` and
   `ZEPHYR_VERSIONS=`). Include the tests in `APPS` for kernel-object,
   syscall, or Kconfig changes. This is build coverage, not test execution;
   only the `RUN_CASES` samples execute (with `RUN=1`).
   - Rust version port: follow `docs/rust-upgrade.md` (rebase the rust/rust port, update the pins, rebuild the container). Only one Rust version is supported per revision, since the std port must exactly match the compiler.
   - Zephyr version port: build the full matrix on the new `ZEPHYR_VERSION`
     (`cd ci && ZEPHYR_VERSIONS=<new> ./build-all.sh`), and confirm the other
     supported versions (see `README.md`) are not broken.
3. **Lint version-gated code per version**: cfg'd-out code is not
   type-checked, so an app that cfg-gates on `zephyrNNN` (e.g. `tests/eeprom`)
   must be clippy-linted on every version whose branch it touches. Use a
   separate clippy volume per Zephyr version (`CLIPPY_BUILD_DIR` is not
   version-keyed):
   `cd ci && DOCKER_ARGS="-v /tmp/zr-clippy-<ver>:/tmp/zephyr-rust-clippy" RUST_VERSION=1.88.0 ZEPHYR_VERSION=<ver> ./build-cmd.sh env CLIPPY_ARGS="-D warnings" ci/clippy.sh <app>`
4. **Full matrix, tests, and clippy (pre-PR / CI parity)**:
   - `cd ci && ./build-all.sh` — the full matrix with `--resume` (reruns skip
     completed jobs; `rm -rf ci/log/build` forces a full re-run). Optionally
     `RUN=1` to execute the verified exiting samples.
   - `cd ci && RUST_VERSION=1.88.0 ./sanitycheck.sh` — executes the tests on
     Zephyr 2.3.0 (qemu_x86, qemu_cortex_m3); the only automated test
     execution today, not full-version test execution (2.7.3/3.7.0
     execution is separate twister work, see `docs/BUILD_MATRIX_TODO.md`).
   - `cd ci && DOCKER_ARGS="-v /tmp/zr-clippy-3.7.0:/tmp/zephyr-rust-clippy" RUST_VERSION=1.88.0 ZEPHYR_VERSION=3.7.0 ./build-cmd.sh env CLIPPY_ARGS="-D warnings" ci/clippy.sh`
     — the same pass the CI clippy job runs (warnings fatal, strict).
5. **Rust-version coupling for local runs**: containers are per (Zephyr,
   Rust) image. When the host has no usable rustc, pass `RUST_VERSION`
   explicitly on every `build-cmd.sh`/`sanitycheck.sh` invocation (all
   examples above do); `ci/build-all.sh` defaults it from
   `rust-toolchain.toml`. The image tags in `main.yml` are updated manually
   per `docs/rust-upgrade.md`.

## Commit message style

- Short imperative subject line, no trailing period (e.g. `Update to Rust 1.75`, `ci: add M33 target`).
- No Conventional Commits types (`feat:`, `fix:`, etc.). Instead, optionally prefix with a lowercase component or area followed by a colon: `ci:`, `zephyr-core:`, `zephyr-bindgen:`, `rust-smem:`. A second-level file prefix is sometimes used, e.g. `ci: env.sh: default to Zephyr 3.7`.
- Optional body for non-trivial changes: blank line after the subject, then a longer explanation of what/why, wrapped near 72 columns.

## Key repository conventions

- Rust app/test crates are named `app` and expose C ABI symbols expected by C shims (`rust_main`, `rust_test_main`, etc.).
- Static Zephyr kernel objects from Rust are defined via `zephyr-macros` (`k_mutex_define!`, `k_sem_define!`, `k_poll_signal_define!`) so they land in Zephyr sections and are initialized via constructor hooks.
- Userspace builds commonly require `CONFIG_RUST_ALLOC_POOL=y`; sample/test `prj.conf` files are the source of truth for required Kconfig combinations.
- This codebase supports multiple Zephyr versions; preserve version guards such as `#if KERNEL_VERSION_MAJOR < 3` in C shims and generated syscall includes.
- `tests/*/testcase.yaml` `platform_whitelist` entries drive the build matrix
  via `ci/matrix.py` (`west build` itself ignores them); keep each whitelist
  accurate to every board the test can build, and pick boards from it when
  running an individual test.

## Zephyr version compatibility (2.3, 2.7.3, 3.7)

- Changes touching C headers, devicetree, syscalls, or POSIX APIs must build
  *and run* on all three supported versions; passing on one version is not
  evidence it works on the others. Run them in the CI containers (see below)
  before considering a fix complete.
- When a Zephyr API's shape differs by version, prefer a single code path over
  hardcoding or per-version literals. For C, use the established
  `#include <version.h>` + `KERNEL_VERSION_MAJOR` guard (see
  samples/*/src/main.c). In Rust, CMake exports `zephyr250`/`zephyr270`/
  `zephyr300`/`zephyr350` cfgs via RUSTFLAGS (the single source of these
  thresholds), so app and std-private crates can cfg-gate on the version directly;
  zephyr-core's build.rs additionally emits `usermode`/`mempool`/
  `mutex_pool`/`clock` cfgs, which only reach zephyr-core itself.
- Known drift (re-verify against the pinned tree, don't trust memory):
  `<zephyr.h>` exists only pre-3, `<zephyr/kernel.h>` only 3+; `<zephyr/kernel.h>`
  does not pull in `<zephyr/device.h>` on 3.x; devicetree bindings dropped the
  `label` property in 3.x (device names fall back to the node full name), and
  nodes get renamed between versions; `__syscall` markers were dropped in 3.x,
  so a function may be a syscall thunk on 2.x but a plain function on 3.x -
  calling the wrong shape is a link error, and calling a plain function from
  userspace on 2.x bypasses z_vrfy checks.
- bindgen limitations (it parses wrapper.h but does not compile it): it cannot
  evaluate nested function-like macros (Zephyr DT macros, `DT_CAT` chains) and
  silently drops macros it cannot evaluate, including cast expressions like
  picolibc's `((clockid_t) 1)`; the symptom is a Rust E0425 long after the
  macro was written. Expose values Rust needs as simple object macros
  expanding directly to constants/string literals, file-scope `const`
  variables in wrapper.h, or definitions in the app's own C file.
- Never hardcode values that come from devicetree, Kconfig, or toolchain
  headers (device names, clock ids, sizes). Route them through wrapper.h or a
  C shim so they track the built image.

## Container investigation workflow

- Inspect per-version facts directly in the pinned Zephyr source:
  `cd ci && RUST_VERSION=1.88.0 ZEPHYR_VERSION=<ver> ./build-cmd.sh bash -c
  "grep ... /zephyrproject/zephyr/include/..."`. One build-cmd.sh invocation
  runs one command; containers are ephemeral, so pass RUST_VERSION and all
  env vars explicitly every time. Host environment variables are NOT
  propagated into the container; pass them via
  `DOCKER_ARGS="... -e VAR=value"`.
- Persist build dirs across invocations with
  `DOCKER_ARGS="-v /tmp/<name>:/tmp/build"` and `west build -d /tmp/build`;
  the repo is mounted read-only. Don't `rm` the mount point itself.
- Use the Zephyr test runner for tests and `ci/run-sample.sh` for verified
  samples. Non-exiting QEMU processes require process-group cleanup; never
  use a bare `ninja run`/`timeout` pipeline.
- To diagnose binding issues, inspect the build dir: `bindings.rs` under
  `modules/zephyr-rust/bindings/` (one huge
  line; use targeted `grep -o`), `zephyr/include/generated/` (all_syscalls.h,
  syscall_thunks.c, devicetree_generated.h), and the cflags bindgen receives
  in the build dir's `rust-env.sh` (`TARGET_CFLAGS`, which includes
  `-imacros autoconf.h` but not the devicetree generated header).
- `build-all.sh` progress while it runs: `find ci/log/build -name seq
  | wc -l` counts started jobs (of `ci/matrix.py --tsv | wc -l`),
  `docker ps -q | wc -l` counts active containers; GNU parallel writes
  no per-job done-marker, so completion is only visible in the caller's
  exit status. Full matrix takes ~20 min at `-j8`.
- Keep command output small or write results to a file under the persistent
  volume; large/truncated output obscures the lines you need.
