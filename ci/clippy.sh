#!/bin/bash
#
# Run cargo clippy on all Rust crates in zephyr-rust.
#
# This must be run in an environment with the Rust toolchain pinned in
# rust-toolchain.toml (including the clippy component) and a Zephyr west
# workspace with the Zephyr SDK, i.e. inside the CI container or a native
# development setup:
#
#   # CI container (preferred; the repo is mounted read-only, which this
#   # script supports):
#   cd ci && ./build-cmd.sh ci/clippy.sh
#
#   # natively (west, Zephyr, and the Zephyr SDK must be set up):
#   ./ci/clippy.sh
#
# What to lint (arguments):
#   ci/clippy.sh             everything (the default; used by CI)
#   ci/clippy.sh lib         only the common library crates
#   ci/clippy.sh APP...      only the given samples/tests apps
#   ci/clippy.sh lib APP...  the common library crates and the given apps
# APP is a samples/ or tests/ dir name (e.g. serial) or path (e.g.
# samples/serial).
#
# How it works:
#   The cross-compiled Rust build is driven by CMake (see CMakeLists.txt and
#   rust/build.sh), and the sysroot it produces is app-specific: zephyr-sys
#   regenerating its bindings depends on the app's headers/devicetree, and
#   zephyr-core's cfgs depend on the app's Kconfig. So for every sample and
#   test this script:
#     1. runs a regular `west build` in its own build dir, which produces
#        the cross-compiled sysroot (std + zephyr-sys + zephyr-core) and the
#        zephyr-bindgen binary,
#     2. sources rust-env.sh, the environment file CMake generates into the
#        build dir (sysroot, target, bindgen flags, Kconfig values; see
#        CMakeLists.txt), so clippy sees exactly the same configuration as a
#        real build,
#     3. runs `cargo clippy` on the app crate (clippy also lints its local
#        path dependencies, e.g. the zephyr, zephyr-logger,
#        zephyr-futures, and zephyr-uart-buffered crates).
#   The passes, in order:
#     1. host crates (zephyr-bindgen, zephyr-macros), linted without a
#        target,
#     2. common code: a west build of samples/rust-app (whose build tree
#        provides the cross-compiled sysroot and environment), then the
#        sysroot-layer crates (zephyr-sys, zephyr-core, time-convert) and
#        the app-layer library crates, linted against that environment.
#        Linting the common code first fails fast, before the per-app pass,
#     3. per-app west builds + clippy, parallelized; when pass 2 ran, the
#        samples/rust-app build from it is reused (its west build is a
#        no-op).
#   Pass 2 runs for 'lib' or with no arguments; pass 3 runs for the apps
#   given as arguments, or for all of them with no arguments.
#
# No files are written to the source tree (every crate that is used as a
# clippy root has a committed Cargo.lock); all build/clippy artifacts go
# under CLIPPY_BUILD_DIR.
#
# Environment variables:
#   CLIPPY_BOARD         board for all builds (default: qemu_x86)
#   CLIPPY_BUILD_DIR     base build dir (default: /tmp/zephyr-rust-clippy)
#   CLIPPY_JOBS          parallel app builds (default: number of CPUs)
#   CLIPPY_ARGS          extra args appended after `--` on every clippy
#                        invocation (e.g. CLIPPY_ARGS="-D warnings")
#   CLIPPY_STRICT=1      also fail if an app cannot be *built* on
#                        CLIPPY_BOARD. By default such apps are reported as
#                        skipped (e.g. some tests only build on specific
#                        Zephyr versions/boards); the script only fails on
#                        clippy failures.

set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ZEPHYR_RUST="$(cd "${DIR}/.." && pwd)"
cd "${ZEPHYR_RUST}"

BOARD="${CLIPPY_BOARD:-qemu_x86}"
BUILD_DIR="${CLIPPY_BUILD_DIR:-/tmp/zephyr-rust-clippy}"
JOBS="${CLIPPY_JOBS:-$(nproc 2>/dev/null || echo 2)}"
CLIPPY_ARGS="${CLIPPY_ARGS:-}"

if ! cargo clippy --version >/dev/null 2>&1; then
    echo "error: cargo-clippy is not installed for the active toolchain." >&2
    echo "       run: rustup component add clippy" >&2
    exit 1
fi

# Shared cargo target dir for the clippy invocations: keeps the (possibly
# read-only) source tree clean and lets passes share compiled registry
# dependencies.
export CARGO_TARGET_DIR="${BUILD_DIR}/cargo-target"

# ---------------------------------------------------------------------------
# Arguments: [lib] [APP...]
# ---------------------------------------------------------------------------
usage() {
    echo "usage: ${0##*/} [lib] [APP...]" >&2
    echo "  APP is a samples/ or tests/ app (dir name or path); 'lib' selects" >&2
    echo "  the common library crates. With no arguments, everything is linted." >&2
    exit 2
}

LIB=0
APPS=()
for arg in "$@"; do
    if [ "${arg}" = "lib" ]; then
        LIB=1
        continue
    fi
    found=0
    for d in samples/*/ tests/*/; do
        d="${d%/}"
        if [ -f "${d}/Cargo.toml" ] && { [ "${d##*/}" = "${arg}" ] || [ "${d}" = "${arg}" ]; }; then
            dup=0
            for a in "${APPS[@]}"; do
                [ "${a}" = "${d}" ] && dup=1
            done
            [ "${dup}" = 1 ] || APPS+=("${d}")
            found=1
            break
        fi
    done
    [ "${found}" = 1 ] || usage
done

# No arguments: lint everything.
FULL_RUN=0
if [ "$#" -eq 0 ]; then
    FULL_RUN=1
    for d in samples/*/ tests/*/; do
        [ -f "${d}Cargo.toml" ] && APPS+=("${d%/}")
    done
fi

STATUS_DIR="${BUILD_DIR}/clippy-status"
mkdir -p "${STATUS_DIR}"

# Load the Rust build environment CMake generated into the build dir
# (rust-env.sh; see CMakeLists.txt). Exports: RUST_TARGET, RUST_TARGET_SPEC,
# CLANG_TARGET, TARGET_CFLAGS, SYSROOT, ZEPHYR_BINDGEN,
# ZEPHYR_KERNEL_VERSION_NUM, RUSTC_BOOTSTRAP, CONFIG_*.
derive_env() {
    local bdir=$1
    local env_file="${bdir}/rust-env.sh"
    if [ ! -f "${env_file}" ]; then
        echo "error: ${env_file} not found; did the west build configure successfully?" >&2
        exit 1
    fi
    # shellcheck disable=SC1090
    . "${env_file}"
    local k
    for k in RUST_TARGET RUST_TARGET_SPEC TARGET_CFLAGS SYSROOT ZEPHYR_BINDGEN ZEPHYR_KERNEL_VERSION_NUM; do
        if [ -z "${!k:-}" ]; then
            echo "error: ${k} is missing or empty in ${env_file};" >&2
            echo "       the CMake-generated environment may have changed" >&2
            exit 1
        fi
    done
}

# RUSTFLAGS for cross-compiled clippy invocations against a given sysroot.
cross_rustflags() {
    echo "${RUSTFLAGS:+${RUSTFLAGS} }--sysroot $1"
}

fail=0
run_clippy() {
    echo
    echo "=== cargo clippy $*"
    # shellcheck disable=SC2086
    if ! cargo clippy "$@" -- ${CLIPPY_ARGS}; then
        fail=1
        return 1
    fi
}

# ---------------------------------------------------------------------------
# 1. Host crates (no --target: proc-macros and host tools build for the
#    host, and must not use the cross-compiled sysroot).
# ---------------------------------------------------------------------------
run_clippy --manifest-path zephyr-bindgen/Cargo.toml --all-targets || true
run_clippy --manifest-path rust/zephyr-macros/Cargo.toml --all-targets || true

# Common code pass: build samples/rust-app to get the sysroot and
# environment, then lint the sysroot-layer and app-layer library crates.
# (The library crates are also covered as path dependencies in the per-app
# passes; this makes the coverage explicit and fails fast.
# zephyr-uart-buffered is not linted here: it only compiles in builds with
# CONFIG_UART_BUFFERED, covered by the samples/serial pass.)
run_common_pass() {
    echo
    echo "=== west build -b ${BOARD} samples/rust-app"
    west build -d "${BUILD_DIR}/rust-app" -p auto -b "${BOARD}" samples/rust-app \
        > "${STATUS_DIR}/rust-app.build.log" 2>&1 || true
    if [ -f "${BUILD_DIR}/rust-app/zephyr/zephyr.elf" ]; then
        derive_env "${BUILD_DIR}/rust-app"
        # Inline (not exported): cross_rustflags appends to RUSTFLAGS, so an
        # exported value would be doubled up by later invocations.
        rf="$(cross_rustflags "${SYSROOT}")"

        RUSTFLAGS="${rf}" run_clippy --manifest-path rust/sysroot-stage1/Cargo.toml \
            -p zephyr-sys -p zephyr-core -p time-convert \
            --target "${RUST_TARGET_SPEC}" --lib || true

        for m in rust/zephyr rust/zephyr-logger rust/zephyr-futures; do
            RUSTFLAGS="${rf}" run_clippy --manifest-path "${m}/Cargo.toml" \
                --target "${RUST_TARGET_SPEC}" --lib || true
        done
    else
        echo "note: samples/rust-app did not build on ${BOARD}; last lines of"
        echo "      ${STATUS_DIR}/rust-app.build.log:"
        tail -n 15 "${STATUS_DIR}/rust-app.build.log"
        echo "note: skipping the sysroot-layer and app-layer library crate passes"
        [ -z "${CLIPPY_STRICT:-}" ] || fail=1
    fi
}

if [ "${LIB}" = 1 ] || [ "${FULL_RUN}" = 1 ]; then
    run_common_pass
fi

app_worker() {
    local app=$1
    local name bdir build_rc=0 clippy_rc=0
    name="$(basename "${app}")"
    bdir="${BUILD_DIR}/${name}"
    # The subshell contains any `exit` from derive_env; set -e is re-enabled
    # inside because the `|| clippy_rc=$?` below would suppress it.
    (
        set -e
        echo "=== west build -b ${BOARD} ${app}"
        west build -d "${bdir}" -p auto -b "${BOARD}" "${app}"
        derive_env "${bdir}"
        echo "Rust target: ${RUST_TARGET}"
        echo "=== cargo clippy ${app}"
        RUSTFLAGS="$(cross_rustflags "${SYSROOT}")" \
            cargo clippy --manifest-path "${app}/Cargo.toml" \
            --target "${RUST_TARGET_SPEC}" --lib \
            -- ${CLIPPY_ARGS}
    ) > "${STATUS_DIR}/${name}.log" 2>&1 || clippy_rc=$?
    # Distinguish "cannot build on this board" from clippy failures: a
    # successful west build links zephyr.elf.
    if [ ! -f "${bdir}/zephyr/zephyr.elf" ]; then
        build_rc=1
    fi
    echo "${build_rc} ${clippy_rc}" > "${STATUS_DIR}/${name}.exit"
}

# ---------------------------------------------------------------------------
# 3. Per-app: west build + clippy on the app crate (clippy also lints the
#    app's local path dependencies). When the common code pass ran, the
#    samples/rust-app build from it is reused (a no-op west build).
# ---------------------------------------------------------------------------
if [ "${#APPS[@]}" -gt 0 ]; then
    echo
    echo "=== per-app builds + clippy: ${APPS[*]} (jobs: ${JOBS})"
fi
for app in "${APPS[@]}"; do
    app_worker "${app}" &
    # Throttle to JOBS concurrent workers
    while [ "$(jobs -rp | wc -l)" -ge "${JOBS}" ]; do
        wait -n || true
    done
done
wait || true

skipped=0
for app in "${APPS[@]}"; do
    name="$(basename "${app}")"
    read -r build_rc clippy_rc < "${STATUS_DIR}/${name}.exit"
    if [ "${build_rc}" != 0 ]; then
        echo
        echo "=== ${app} SKIPPED: west build failed on ${BOARD}; last lines of ${STATUS_DIR}/${name}.log:"
        tail -n 15 "${STATUS_DIR}/${name}.log"
        skipped=1
        if [ -n "${CLIPPY_STRICT:-}" ]; then
            fail=1
        fi
    elif [ "${clippy_rc}" != 0 ]; then
        echo
        echo "=== ${app} CLIPPY FAILED (exit ${clippy_rc}); last lines of ${STATUS_DIR}/${name}.log:"
        tail -n 30 "${STATUS_DIR}/${name}.log"
        fail=1
    else
        echo "${app}: OK"
    fi
done

echo
if [ "${skipped}" != 0 ]; then
    echo "note: some apps were skipped because they do not build on ${BOARD}"
    echo "      (set CLIPPY_STRICT=1 to treat that as a failure)"
fi
if [ "${fail}" != 0 ]; then
    echo "clippy: FAILED"
    exit 1
fi
echo "clippy: OK"
