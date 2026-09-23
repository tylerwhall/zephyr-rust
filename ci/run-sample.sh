#!/bin/bash
# Run the sample built in /tmp/build via `ninja run`, in its own process
# group, and clean up the whole group (ninja + emulator) if it does not exit
# on its own.
#
# Only use this for samples verified to exit automatically (see the RUN list
# in ci/build-all.sh and the Task 3 inventory in docs/BUILD_MATRIX_TODO.md).
# The deadline below is a safety net against a regression that makes the
# sample hang; it is not the success criterion.
#
# Exit status: the sample's own status if it exited on its own, 124 if the
# deadline was hit. The complete run output is printed on stdout.

set -u

TIMEOUT=${TIMEOUT:-120}
BUILD_DIR=${BUILD_DIR:-/tmp/build}
cd "$BUILD_DIR"

setsid ninja run > /tmp/ninja-run.log 2>&1 &
pid=$!
# Last-resort cleanup of the whole process group (ninja + emulator).
trap 'kill -KILL -- -"$pid" 2>/dev/null' EXIT

start=$(date +%s)
while kill -0 "$pid" 2>/dev/null; do
    if [ $(( $(date +%s) - start )) -ge "$TIMEOUT" ]; then
        echo "run-sample: timed out after ${TIMEOUT}s; killing process group" >&2
        kill -TERM -- "-$pid" 2>/dev/null
        sleep 2
        kill -KILL -- "-$pid" 2>/dev/null
        wait "$pid" 2>/dev/null
        cat /tmp/ninja-run.log
        exit 124
    fi
    sleep 1
done
wait "$pid"
status=$?
cat /tmp/ninja-run.log
exit "$status"
