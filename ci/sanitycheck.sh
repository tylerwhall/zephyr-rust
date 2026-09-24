#!/bin/bash

# Runs the pre-twister sanitycheck runner, which only exists on Zephyr 2.3.0
# (testcase.yaml schema changed after 2.3). The runner hardcodes
# -DEXTRA_CFLAGS="-Werror" and -Wl,--fatal-warnings for test builds, which
# turns inherent Zephyr 2.3 warnings into errors on some boards: the
# native_posix kernel's noinit section attribute conflict (kernel/init.c vs
# kernel_internal.h) and the cortex_r5 DT_TEXTREL link warning. qemu_riscv*
# are excluded because riscv boards are only supported on Zephyr 3.x. So the
# platform list is limited to the boards that pass on 2.3.0 — both execute
# the tests under QEMU; build coverage for the excluded boards is in the
# main build matrix (ci/matrix.py). Expanding this scope: see
# docs/BUILD_MATRIX_TODO.md.
ZEPHYR_VERSION=2.3.0

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

rm -rf sanity-out
DOCKER_ARGS=(-e ZEPHYR_TOOLCHAIN_VARIANT=zephyr -v ${DIR}/sanity-out:/sanity-out)

. "${DIR}/build-cmd.sh" sh -c "\$ZEPHYR_BASE/scripts/sanitycheck -N -O /sanity-out/out -c -p qemu_x86 -p qemu_cortex_m3 -T /zephyr-rust/tests"
