#!/bin/bash -e

# Run the same build matrix as .github/workflows/main.yml: the job list
# comes from ci/matrix.py, the single source of the matrix (its header
# comment lists the rules and the trim knobs).
#
# Like CI, each job runs in its own container and builds into /tmp/build
# inside that container, so jobs never share build state. Re-running the
# script skips jobs that already succeeded (--resume); remove log/build to
# force a full re-run. Per-job output is under log/build/.
#
# RUN=1 also executes the matrix entries marked run=true in ci/matrix.py
# (the verified qemu_x86 rust-app/no_std cases); the default RUN=0 remains
# build-only.

RUN=${RUN:-0}
case "$RUN" in
    0|1) ;;
    *) echo "RUN must be 0 or 1" >&2; exit 2 ;;
esac
export RUN

# RUST_VERSION must be set for the container image tag; default to the
# toolchain channel pinned by the repo (the same value env.sh derives from
# rustc when a toolchain is installed).
RUST_VERSION=${RUST_VERSION:-$( sed -n 's/^channel *= *"\(.*\)"/\1/p' "$(dirname "$0")/../rust-toolchain.toml" )}
export RUST_VERSION

MATRIX_PY="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )/matrix.py"

run() {
    local version=$1 board=$2 app=$3 runjob=$4 expected_status=$5 expected=$6
    # RUN is read here on the host; it is not propagated into containers.
    if [ "$RUN" = 1 ] && [ "$runjob" = true ]; then
        runjob=true
    else
        runjob=false
    fi
    ZEPHYR_VERSION=$version ./build-cmd.sh bash -c '
        west build -d /tmp/build -p auto -b "$1" "$2" || exit $?
        if [ "$3" = true ]; then
            set +e
            bash ci/run-sample.sh > /tmp/sample-run.log 2>&1
            status=$?
            set -e
            cat /tmp/sample-run.log
            grep -F "$4" /tmp/sample-run.log
            if [ "$status" -ne "$5" ]; then
                echo "sample exited with $status, expected $5" >&2
                exit 1
            fi
        fi
    ' _ "$board" "$app" "$runjob" "$expected" "$expected_status"
}
export -f run

# Keep RUN=0 and RUN=1 results separate so --resume cannot skip a job just
# because the same build tuple completed in the other mode.
"$MATRIX_PY" --tsv | parallel -j8 --colsep '\t' \
    --results "log/build/run-$RUN" --resume --halt now,fail=1 \
    run {1} {2} {3} {4} {5} {6}
