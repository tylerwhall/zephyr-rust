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
#   rust/cargo.sh), and the std build is app-specific: zephyr-sys
#   bindings depend on the app's headers/devicetree, and
#   zephyr-core's cfgs depend on the app's Kconfig. So for every sample and
#   test this script:
#     1. runs a regular `west build` in its own build dir, which produces
#        bindings and build-local source/toolchain overlay,
#     2. sources rust-env.sh, the environment file CMake generates into the
#        build dir (toolchain, target, bindgen flags, Kconfig values; see
#        CMakeLists.txt), so clippy sees exactly the same configuration as a
#        real build,
#     3. runs `rust/cargo.sh clippy` with the same build-std configuration.
#        Library roots are linted explicitly: ordinary non-member path
#        dependencies do not receive Clippy's workspace wrapper.
#   The passes, in order:
#     1. host crates (zephyr-bindgen, zephyr-macros), linted without a
#        target,
#     2. common code: a west build of samples/rust-app (whose build tree
#        provides bindings and environment), then the
#        low-level crates (zephyr-sys, zephyr-core, time-convert) and
#        the app-layer library crates, linted against that environment.
#        Linting the common code first fails fast, before the per-app pass.
#        Each library is a standalone Clippy root with its own manifest
#        and committed lockfile, including the low-level crates used by std.
#        Selecting non-member dependencies with -p would run only rustc,
#        not Clippy; see docs/BUILD_STD_INVESTIGATION.md (Clippy section).
#     3. per-app west builds + clippy, parallelized; when pass 2 ran, the
#        samples/rust-app build from it is reused (its west build is a
#        no-op).
#   Pass 2 runs for 'lib' or with no arguments; pass 3 runs for the apps
#   given as arguments, or for all of them with no arguments.
#
# No files are written to the source tree (every crate that is used as a
# clippy root has a committed Cargo.lock, which is enforced by running every
# clippy with --locked); all build/clippy artifacts go under
# CLIPPY_BUILD_DIR. Runs that need to write to the source tree (e.g. cargo
# clippy --fix, regenerating a Cargo.lock) can opt in with WRITABLE=1, which
# ci/build-cmd.sh honors by mounting the repo writable.
#
# Environment variables:
#   CLIPPY_BOARD         board for all builds (default: qemu_x86)
#   CLIPPY_BUILD_DIR     base build dir (default: /tmp/zephyr-rust-clippy)
#   CLIPPY_JOBS          parallel app builds (default: number of CPUs)
#   CLIPPY_ARGS          extra args appended after `--` on every clippy
#                        invocation (e.g. CLIPPY_ARGS="-D warnings")
#   CLIPPY_STRICT=0      report apps that cannot be *built* on CLIPPY_BOARD as
#                        skipped instead of failing. The default is strict.

set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ZEPHYR_RUST="$(cd "${DIR}/.." && pwd)"
cd "${ZEPHYR_RUST}"

BOARD="${CLIPPY_BOARD:-qemu_x86}"
BUILD_DIR="${CLIPPY_BUILD_DIR:-/tmp/zephyr-rust-clippy}"
JOBS="${CLIPPY_JOBS:-$(nproc 2>/dev/null || echo 2)}"
CLIPPY_ARGS="${CLIPPY_ARGS:-}"
# Fail by default when an app cannot build. Set CLIPPY_STRICT=0 to retain the
# legacy skip behavior for board-incompatible apps.
CLIPPY_STRICT="${CLIPPY_STRICT:-1}"

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
# TARGET_CFLAGS, RUST_BUILD_TOOLCHAIN, RUST_BUILD_STD, CARGO_MANIFEST,
# RUSTC_BOOTSTRAP, CONFIG_*.
derive_env() {
    local bdir=$1
    local env_file="${bdir}/rust-env.sh"
    export RUST_ENV="${env_file}"
    if [ ! -f "${env_file}" ]; then
        echo "error: ${env_file} not found; did the west build configure successfully?" >&2
        exit 1
    fi
    # shellcheck disable=SC1090
    . "${env_file}"
    local k
    for k in RUST_TARGET RUST_TARGET_SPEC TARGET_CFLAGS RUST_BUILD_TOOLCHAIN RUST_BUILD_STD CARGO_MANIFEST; do
        if [ -z "${!k:-}" ]; then
            echo "error: ${k} is missing or empty in ${env_file};" >&2
            echo "       the CMake-generated environment may have changed" >&2
            exit 1
        fi
    done
}

fail=0
run_clippy() {
    echo
    echo "=== $*"
    # shellcheck disable=SC2086
    if ! "$@" --locked -- ${CLIPPY_ARGS}; then
        fail=1
        return 1
    fi
}

# ---------------------------------------------------------------------------
# 1. Host crates (no --target: proc-macros and host tools build for the
#    host, without build-std).
# ---------------------------------------------------------------------------
run_clippy cargo clippy --manifest-path zephyr-bindgen/Cargo.toml --all-targets
run_clippy cargo clippy --manifest-path rust/zephyr-macros/Cargo.toml --all-targets

# Common code pass: build samples/rust-app to get bindings and the
# environment, then lint each low-level and app-layer library from its own
# manifest so Cargo applies Clippy's workspace wrapper to that crate.
# zephyr-uart-buffered is not linted here: it only compiles in builds with
# CONFIG_UART_BUFFERED, covered by the samples/serial pass.)
run_common_pass() {
    echo
    echo "=== west build -b ${BOARD} samples/rust-app"
    if west build -d "${BUILD_DIR}/rust-app" -p auto -b "${BOARD}" samples/rust-app \
        > "${STATUS_DIR}/rust-app.build.log" 2>&1; then
        derive_env "${BUILD_DIR}/rust-app"
        for m in rust/zephyr-sys rust/zephyr-core rust/zephyr-core/time-convert \
            rust/zephyr rust/zephyr-logger rust/zephyr-futures; do
            run_clippy rust/cargo.sh clippy --manifest-path "${m}/Cargo.toml" --lib
        done
    else
        echo "note: samples/rust-app did not build on ${BOARD}; last lines of"
        echo "      ${STATUS_DIR}/rust-app.build.log:"
        tail -n 15 "${STATUS_DIR}/rust-app.build.log"
        echo "note: skipping the low-level and app-layer library crate passes"
        if [ "${CLIPPY_STRICT:-1}" != "0" ]; then
            fail=1
        fi
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
    {
        echo "=== west build -b ${BOARD} ${app}"
        if west build -d "${bdir}" -p auto -b "${BOARD}" "${app}"; then
            # Contain any fatal `exit` from derive_env in this worker.
            (
                derive_env "${bdir}"
                echo "Rust target: ${RUST_TARGET}"
                rust/cargo.sh clippy --manifest-path "${app}/Cargo.toml" \
                    --locked --lib -- ${CLIPPY_ARGS}
            ) || clippy_rc=$?
        else
            build_rc=1
        fi
    } > "${STATUS_DIR}/${name}.log" 2>&1
    echo "${build_rc} ${clippy_rc}" > "${STATUS_DIR}/${name}.exit"
}

# ---------------------------------------------------------------------------
# 3. Per-app: west build + clippy on the app crate. When the common pass ran, the
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
        if [ "${CLIPPY_STRICT:-1}" != "0" ]; then
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
if [ "${skipped}" != 0 ] && [ "${CLIPPY_STRICT:-1}" = "0" ]; then
    echo "note: some apps were skipped because they do not build on ${BOARD}"
    echo "      (set CLIPPY_STRICT=1 to treat that as a failure)"
fi
if [ "${fail}" != 0 ]; then
    echo "clippy: FAILED"
    exit 1
fi
echo "clippy: OK"
