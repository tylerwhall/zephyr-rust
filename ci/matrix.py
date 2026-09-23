#!/usr/bin/env python3
"""Single source of the build matrix.

Used by:
  - .github/workflows/main.yml: a "matrix" job runs this script and the
    build job expands the JSON with `fromJSON`.
  - ci/build-all.sh: feeds `--tsv` output to parallel.

Keep every matrix rule here; do not duplicate exclusions in the workflow
or other scripts. Trim knobs for local runs (environment variables):
  ZEPHYR_VERSIONS  default "3.7.0 2.7.3 2.3.0"
  BOARDS           default: all boards below
  APPS             default: all apps below
e.g. ZEPHYR_VERSIONS=3.7.0 BOARDS=qemu_x86 APPS=samples/rust-app ci/matrix.py

Rules encoded here:
  - Apps/tests are built only on the boards whitelisted in their
    tests/*/testcase.yaml platform_whitelist (west build ignores the
    whitelist, so the matrix must enumerate only whitelisted boards).
    Samples have no testcase.yaml and build on all boards.
  - qemu_riscv32/qemu_riscv64 are only supported on Zephyr 3.x.
  - native_posix is only supported on Zephyr 2.x: the cross-compiled
    sysroot has no std for the native_posix target on Zephyr 3.x
    (rustc E0463).
  - samples/serial needs CONFIG_UART_INTERRUPT_DRIVEN, which
    native_posix does not support.
  - Only the sample runs verified to exit on their own
    (docs/BUILD_MATRIX_TODO.md Task 3) are marked run=true with their
    expected output and exit status; everything else is build-only.
"""

import json
import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

ZEPHYR_VERSIONS = os.environ.get("ZEPHYR_VERSIONS", "3.7.0 2.7.3 2.3.0").split()
BOARDS = os.environ.get(
    "BOARDS",
    "qemu_x86 qemu_cortex_m3 qemu_cortex_r5 nucleo_l552ze_q"
    " native_posix qemu_riscv32 qemu_riscv64",
).split()
APPS = os.environ.get(
    "APPS",
    "samples/rust-app samples/no_std samples/serial"
    " tests/rust tests/semaphore tests/posix-clock tests/eeprom",
).split()

# Zephyr-version-scoped exclusions: (board, zephyr_version).
EXCLUDED = {
    ("qemu_riscv32", "2.3.0"),
    ("qemu_riscv64", "2.3.0"),
    ("qemu_riscv32", "2.7.3"),
    ("qemu_riscv64", "2.7.3"),
    ("native_posix", "3.7.0"),
}

# Verified automatic-exit runs: (board, app) -> (expected output, expected
# exit status). See the Task 3 inventory in docs/BUILD_MATRIX_TODO.md.
RUN_CASES = {
    ("qemu_x86", "samples/rust-app"): (
        "Next call will crash if userspace is working.", 1,
    ),
    ("qemu_x86", "samples/no_std"): (
        "Next call will crash if userspace is working.", 1,
    ),
}


# Board sets that come from the app itself instead of a testcase.yaml
# whitelist (samples have none):
APP_BOARDS = {
    # samples/serial needs CONFIG_UART_INTERRUPT_DRIVEN, which native_posix
    # does not support.
    "samples/serial": [
        "qemu_x86", "qemu_cortex_m3", "qemu_cortex_r5", "nucleo_l552ze_q",
        "qemu_riscv32", "qemu_riscv64",
    ],
}


def whitelist(app):
    """Boards the app is built on (its testcase.yaml whitelist if it has one)."""
    if app in APP_BOARDS:
        return APP_BOARDS[app]
    path = ROOT / app / "testcase.yaml"
    if path.is_file():
        match = re.search(
            r"(?m)^\s*platform_whitelist:\s*(\S.*)$", path.read_text()
        )
        if match:
            return match.group(1).split()
    return BOARDS


def jobs():
    for version in ZEPHYR_VERSIONS:
        for app in APPS:
            for board in whitelist(app):
                if (board, version) in EXCLUDED:
                    continue
                run_case = RUN_CASES.get((board, app))
                yield {
                    "zephyr_version": version,
                    "board": board,
                    "test": app,
                    "run": run_case is not None,
                    "expected_status": run_case[1] if run_case else 0,
                    "expected_output": run_case[0] if run_case else "",
                }


def main():
    entries = list(jobs())
    if "--tsv" in sys.argv[1:]:
        for entry in entries:
            print(
                "\t".join(
                    [
                        entry["zephyr_version"],
                        entry["board"],
                        entry["test"],
                        str(entry["run"]).lower(),
                        str(entry["expected_status"]),
                        entry["expected_output"],
                    ]
                )
            )
    else:
        print(json.dumps({"include": entries}))


if __name__ == "__main__":
    main()
