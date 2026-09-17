#!/bin/bash
#
# Drive a clippy fixup pass: run ci/clippy.sh and summarize the remaining
# warnings grouped by lint with their locations, so each warning type can
# be fixed and committed on its own.
#
# Must be run in the same environment as ci/clippy.sh (CI container or a
# native setup with west, Zephyr, and the Zephyr SDK):
#
#   cd ci && ./build-cmd.sh ci/clippy-fix.sh                 # report
#   cd ci && ./build-cmd.sh ci/clippy-fix.sh lib             # lib crates only
#   cd ci && ./build-cmd.sh ci/clippy-fix.sh serial          # one app
#
# Arguments are passed through to ci/clippy.sh (lib, APP names, or nothing
# for everything). The full pass output is saved to
# ${CLIPPY_BUILD_DIR}/clippy-fix.log. Fixes are applied manually, one
# warning type per commit; see AGENTS.md "Fixing clippy warnings".

set -euo pipefail

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

BUILD_DIR="${CLIPPY_BUILD_DIR:-/tmp/zephyr-rust-clippy}"
LOG="${BUILD_DIR}/clippy-fix.log"
mkdir -p "${BUILD_DIR}"

"${DIR}/clippy.sh" "$@" > "${LOG}" 2>&1
rc=$?

REPORT="$(awk '
    /^warning: / { w = 1; next }
    w && /^ *--> / {
        loc = $0
        sub(/^ *--> /, "", loc)                 # strip the span marker
        sub(/^\/zephyr-rust\//, "", loc)        # repo-relative
        sub(/:[0-9]+$/, "", loc)                # keep file:line
        next
    }
    w && / = note: .*#\[warn\(/ {
        n = $0
        sub(/^.*#\[warn\(/, "", n)
        sub(/\)\].*$/, "", n)
        print n "\t" loc
        w = 0
    }
' "${LOG}" | sort | awk -F'\t' '
    { c[$1]++; l[$1] = l[$1] " " $2 }
    END { for (n in c) printf "%d\t%s\t%s\n", c[n], n, l[n] }
' | sort -k1,1rn -k2,2)"

echo
echo "=== clippy report (${LOG})"
if [ -z "${REPORT}" ]; then
    echo "  no warnings"
else
    printf '%s\n' "${REPORT}" | while IFS=$'\t' read -r count lint locs; do
        printf '  %s (%d):%s\n' "${lint}" "${count}" "${locs}"
    done
fi

if [ "${rc}" != 0 ]; then
    echo "note: the clippy pass failed (exit ${rc}); last lines of ${LOG}:" >&2
    tail -n 15 "${LOG}" >&2
fi
exit "${rc}"