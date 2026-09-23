#!/bin/bash -e

# Run the same build matrix as .github/workflows/main.yml.
#
# Like CI, each job runs in its own container and builds into /tmp/build
# inside that container, so jobs never share build state. Re-running the
# script skips jobs that already succeeded (--resume); remove log/build to
# force a full re-run. Per-job output is under log/build/.
#
# The matrix can be trimmed for quick runs, e.g.:
#   ZEPHYR_VERSIONS=3.7.0 BOARDS=qemu_x86 ./build-all.sh
# RUN=1 also executes only the verified qemu_x86 rust-app/no_std cases;
# the default RUN=0 remains build-only.

ZEPHYR_VERSIONS=${ZEPHYR_VERSIONS:-"3.7.0 2.7.3 2.3.0"}
BOARDS=${BOARDS:-"qemu_x86 qemu_cortex_m3 qemu_cortex_r5 nucleo_l552ze_q \
    native_posix qemu_riscv32 qemu_riscv64"}
SAMPLES=${SAMPLES:-"samples/rust-app samples/no_std samples/serial"}
TESTS=${TESTS:-"tests/rust tests/semaphore tests/posix-clock tests/eeprom"}
RUN=${RUN:-0}
case "$RUN" in
    0|1) ;;
    *) echo "RUN must be 0 or 1" >&2; exit 2 ;;
esac
export RUN

# The same matrix and exclusions as .github/workflows/main.yml. Tests are
# built on the boards from their tests/*/testcase.yaml platform_whitelist
# (west build ignores the whitelist, so it must be enforced here): rust,
# semaphore, and posix-clock on qemu_x86, qemu_cortex_m3, native_posix;
# eeprom on qemu_x86 only (its devicetree eeprom node only exists there).
gen_jobs() {
    local v b t boards
    for v in $ZEPHYR_VERSIONS; do
        for b in $BOARDS; do
            for t in $SAMPLES; do
                case "$v-$b-$t" in
                    # riscv boards are only supported on Zephyr 3.x
                    2.3.0-qemu_riscv*|2.7.3-qemu_riscv*) continue ;;
                    # serial/uart does not exist on posix
                    *-native_posix-samples/serial) continue ;;
                    # the cross-compiled sysroot has no std for the
                    # native_posix target on Zephyr 3.x (rustc E0463)
                    3.7.0-native_posix-*) continue ;;
                esac
                echo "$v $b $t"
            done
        done
        for t in $TESTS; do
            case "$t" in
                tests/eeprom) boards="qemu_x86" ;;
                *) boards="qemu_x86 qemu_cortex_m3 native_posix" ;;
            esac
            for b in $boards; do
                # the cross-compiled sysroot has no std for the
                # native_posix target on Zephyr 3.x (rustc E0463)
                if [ "$v" = "3.7.0" ] && [ "$b" = "native_posix" ]; then
                    continue
                fi
                echo "$v $b $t"
            done
        done
    done
}

run() {
    local version=$1 board=$2 app=$3 mode=$4 run_sample=0
    if [ "$mode" = 1 ]; then
        case "$version" in
            # Verified automatic-exit cases; see the Task 3 inventory below.
            3.7.0|2.7.3|2.3.0)
                case "$board-$app" in
                    qemu_x86-samples/rust-app|qemu_x86-samples/no_std) run_sample=1 ;;
                esac
                ;;
        esac
    fi
    ZEPHYR_VERSION=$version ./build-cmd.sh bash -c '
        west build -d /tmp/build -p auto -b "$1" "$2" || exit $?
        if [ "$3" = 1 ]; then
            set +e
            bash ci/run-sample.sh > /tmp/sample-run.log 2>&1
            status=$?
            set -e
            cat /tmp/sample-run.log
            grep -F "Next call will crash if userspace is working." /tmp/sample-run.log
            if [ "$status" -ne 1 ]; then
                echo "sample exited with $status, expected 1" >&2
                exit 1
            fi
        fi
    ' _ "$board" "$app" "$run_sample"
}
export -f run

# Keep RUN=0 and RUN=1 results separate so --resume cannot skip a job just
# because the same build tuple completed in the other mode.
gen_jobs | parallel -j8 --colsep ' ' --results "log/build/run-$RUN" --resume \
    --halt now,fail=1 run {1} {2} {3} $RUN
