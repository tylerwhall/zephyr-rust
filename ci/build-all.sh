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

ZEPHYR_VERSIONS=${ZEPHYR_VERSIONS:-"3.7.0 2.7.3 2.3.0"}
BOARDS=${BOARDS:-"qemu_x86 qemu_cortex_m3 qemu_cortex_r5 nucleo_l552ze_q \
    native_posix qemu_riscv32 qemu_riscv64"}
SAMPLES=${SAMPLES:-"samples/rust-app samples/no_std samples/serial"}

# The same matrix and exclusions as .github/workflows/main.yml.
gen_jobs() {
    local v b t
    for v in $ZEPHYR_VERSIONS; do
        for b in $BOARDS; do
            for t in $SAMPLES; do
                case "$v-$b-$t" in
                    # riscv boards are only supported on Zephyr 3.x
                    2.3.0-qemu_riscv*|2.7.3-qemu_riscv*) continue ;;
                    # serial/uart does not exist on posix
                    *-native_posix-samples/serial) continue ;;
                    # posix has header issues on Zephyr 3.x
                    3.7.0-native_posix-*) continue ;;
                esac
                echo "$v $b $t"
            done
        done
    done
}

run() {
    ZEPHYR_VERSION=$1 ./build-cmd.sh \
        west build -d /tmp/build -p auto -b $2 $3
}
export -f run

gen_jobs | parallel -j8 --colsep ' ' --results log/build --resume \
    --halt now,fail=1 run {1} {2} {3}
