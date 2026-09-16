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
# How it works:
#   The cross-compiled Rust build is driven by CMake (see CMakeLists.txt and
#   rust/build.sh), and the sysroot it produces is app-specific: zephyr-sys
#   regenerating its bindings depends on the app's headers/devicetree, and
#   zephyr-core's cfgs depend on the app's Kconfig. So for every sample and
#   test this script:
#     1. runs a regular `west build` in its own build dir, which produces
#        the cross-compiled sysroot (std + zephyr-sys + zephyr-core) and the
#        zephyr-bindgen binary,
#     2. re-derives from the build tree the environment CMake passes to
#        rust/build.sh (sysroot, target, bindgen flags, Kconfig values), so
#        clippy sees exactly the same configuration as a real build,
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
#     3. per-app west builds + clippy, parallelized; the samples/rust-app
#        build from step 2 is reused (its west build is a no-op).
#
# No files are written to the source tree (every crate that is used as a
# clippy root has a committed Cargo.lock); all build/clippy artifacts go
# under CLIPPY_BUILD_DIR.
#
# Environment variables:
#   CLIPPY_BOARD         board for all builds (default: qemu_x86)
#   CLIPPY_BUILD_DIR     base build dir (default: /tmp/zephyr-rust-clippy)
#   CLIPPY_JOBS          parallel app builds (default: number of CPUs)
#   CLIPPY_CLANG_TARGET  override the clang target used in TARGET_CFLAGS
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
# Needed for the rustc-dep-of-std crates and #![feature] usage in the app
# crates; mirrors rust/build.sh.
export RUSTC_BOOTSTRAP=1

# Derive the Rust build environment CMake would use, from a completed west
# build dir. Sets/exports: SYSROOT, RUST_TARGET, RUST_TARGET_SPEC,
# ZEPHYR_BINDGEN, TARGET_CFLAGS, ZEPHYR_KERNEL_VERSION_NUM, CONFIG_*.
derive_env() {
    local bdir=$1

    local bindgen
    bindgen="$(find "${bdir}" -maxdepth 5 -type f -name zephyr-bindgen -path '*/release/*' | head -n1)"
    if [ -z "${bindgen}" ]; then
        echo "error: zephyr-bindgen not found under ${bdir}; did the build enable CONFIG_RUST?" >&2
        exit 1
    fi
    export ZEPHYR_BINDGEN="${bindgen}"
    local module_dir
    module_dir="$(cd "$(dirname "${bindgen}")/../.." && pwd)"
    export SYSROOT="${module_dir}/sysroot"

    # Rust target triple: the non-host entry in the sysroot.
    local host_triple
    host_triple="$(rustc -vV | awk '/^host:/{print $2}')"
    export RUST_TARGET="$(ls "${SYSROOT}/lib/rustlib" | grep -v "^${host_triple}$" | head -n1)"
    export RUST_TARGET_SPEC="${ZEPHYR_RUST}/rust/targets/${RUST_TARGET}.json"

    # Clang target for bindgen (mirrors the mapping in CMakeLists.txt).
    local clang_target
    if [ -n "${CLIPPY_CLANG_TARGET:-}" ]; then
        clang_target="${CLIPPY_CLANG_TARGET}"
    elif [ "${RUST_TARGET}" = "i686-unknown-zephyr" ]; then
        clang_target="i686-unknown-linux-gnu"
    elif [ "${RUST_TARGET}" = "thumbv7m-zephyr-eabi" ]; then
        clang_target="thumbv7m-none-eabi"
    elif [ "${RUST_TARGET}" = riscv* ]; then
        clang_target="${RUST_TARGET}"
    else
        clang_target="${RUST_TARGET/-zephyr-/-unknown-none-}"
    fi

    # Kconfig values consumed by build scripts (zephyr-core/build.rs,
    # zephyr-bindgen); read from the generated autoconf.h.
    local autoconf_h="${bdir}/zephyr/include/generated/zephyr/autoconf.h"
    kconfig_val() {
        local v
        v="$(sed -n "s/^#define ${1} //p" "${autoconf_h}" | head -n1)"
        if [ "${v}" = "1" ]; then echo y; else echo n; fi
    }
    export CONFIG_USERSPACE="$(kconfig_val CONFIG_USERSPACE)"
    export CONFIG_RUST_ALLOC_POOL="$(kconfig_val CONFIG_RUST_ALLOC_POOL)"
    export CONFIG_RUST_MUTEX_POOL="$(kconfig_val CONFIG_RUST_MUTEX_POOL)"
    export CONFIG_POSIX_CLOCK="$(kconfig_val CONFIG_POSIX_CLOCK)"
    export CONFIG_THREAD_LOCAL_STORAGE="$(kconfig_val CONFIG_THREAD_LOCAL_STORAGE)"
    export ZEPHYR_KERNEL_VERSION_NUM="$(
        awk '/^#define KERNEL_VERSION_NUMBER/{print $3}' \
            "${bdir}/zephyr/include/generated/zephyr/version.h")"

    # TARGET_CFLAGS for zephyr-bindgen: the include/define/target flags from
    # compile_commands.json (the same set CMake puts in
    # external_project_cflags).
    local cflags
    cflags="$(python3 - "${bdir}/compile_commands.json" <<'EOF'
import json, re, sys
cc = json.load(open(sys.argv[1]))
e = next(x for x in cc if re.search(r"/zephyrproject/.*\.c$", x["file"]))
args = e["arguments"] if "arguments" in e else e["command"].split()
keep, i = [], 0
while i < len(args):
    a = args[i]
    if a.startswith(("-I", "-D")) or a.startswith("--target="):
        keep.append(a)
    elif a in ("-isystem", "-iquote", "-imacros"):
        keep.append(a)
        i += 1
        keep.append(args[i])
    i += 1
print(" ".join(keep))
EOF
)"
    # CMake appends --target=<clang target> explicitly; the compile command
    # may not include it (e.g. on x86), so add it if missing.
    case " ${cflags} " in
        *" --target="*) ;;
        *) cflags="${cflags} --target=${clang_target}" ;;
    esac
    export TARGET_CFLAGS="${cflags}"
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

# Apps to lint in the per-app pass, and where their logs/status go.
APPS=()
for d in samples/*/ tests/*/; do
    if [ -f "${d}Cargo.toml" ]; then
        APPS+=("${d%/}")
    fi
done

STATUS_DIR="${BUILD_DIR}/clippy-status"
mkdir -p "${STATUS_DIR}"

# ---------------------------------------------------------------------------
# 2. Common code: build samples/rust-app to get the sysroot and environment,
#    then lint the sysroot-layer and app-layer library crates. (Also covered
#    as path dependencies in the per-app passes below; this makes the
#    coverage explicit and fails fast. zephyr-uart-buffered is not linted
#    here: it only compiles in builds with CONFIG_UART_BUFFERED, covered by
#    the samples/serial pass.)
# ---------------------------------------------------------------------------
echo
echo "=== west build -b ${BOARD} samples/rust-app"
west build -d "${BUILD_DIR}/rust-app" -p auto -b "${BOARD}" samples/rust-app \
    -DEXPORT_COMPILE_COMMANDS=ON > "${STATUS_DIR}/rust-app.build.log" 2>&1 || true
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
        west build -d "${bdir}" -p auto -b "${BOARD}" "${app}" \
            -DEXPORT_COMPILE_COMMANDS=ON
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
#    app's local path dependencies). The samples/rust-app build from step 2
#    is reused.
# ---------------------------------------------------------------------------
echo
echo "=== per-app builds + clippy: ${APPS[*]} (jobs: ${JOBS})"
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
