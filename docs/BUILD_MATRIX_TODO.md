# Build-matrix TODO list

Ordered TODO list for aligning the build/test matrix across `.github/workflows/main.yml`,
`ci/build-all.sh`, `ci/sanitycheck.sh`, `ci/clippy.sh`, and the AGENTS.md agent workflow.
Ordered from highest to lowest priority; sanitycheck/twister is intentionally last.
Work top to bottom; each task lists how to evaluate the result locally before
moving on. Do not start the next task until the current one is validated and
committed.

## Conventions used by every task (read first)

- All local evaluation happens in the CI containers; the repo is mounted
  read-only by default. Edit files on the host, commit on the host.
- One `ci/build-cmd.sh` invocation runs one command, and containers are
  ephemeral: pass `RUST_VERSION=1.78.0 ZEPHYR_VERSION=<ver>` and any
  `DOCKER_ARGS` on *every* invocation.
- Persist build dirs across invocations with
  `DOCKER_ARGS="-v /tmp/<name>:/tmp/build"`; never `rm` the mount point
  itself (use a fresh volume name or delete subdirectories).
- QEMU test runs do not exit; wrap `ninja run` in `timeout` (samples exit by
  design: samples/rust-app ends with an intentional page fault and a non-zero
  exit).
- Full-matrix runs use `ci/build-all.sh` (results under `ci/log/build/`,
  `--resume` skips completed jobs; `rm -rf ci/log/build` to force a full run).
- Native_posix builds in containers only on Zephyr 2.3.0/2.7.3; 3.7.0 fails
  (picolibc header issue) and is excluded from the matrix.
- Every commit: short imperative subject, optional lowercase component
  prefix (`ci:`, `build:`, `docs:`), no Conventional Commits.

---

## Task 1 — Fix CLIPPY_STRICT semantics and stale clippy documentation — DONE (b9b7425)

**Goal**: make `ci/clippy.sh`'s strictness flag behave as documented and sync
the docs that describe it.

**Why**: AGENTS.md and the main.yml clippy-job comment still describe the old
non-strict default ("reported as skipped"); `b5723e9` made strict the default.
Two script bugs hide behind this: the closing "set CLIPPY_STRICT=1..." hint is
wrong in strict mode, and `run_common_pass` treats *any* non-empty
`CLIPPY_STRICT` (including the documented "off" value `0`) as strict.

**Steps**:
1. `ci/clippy.sh`, `run_common_pass`: change
   `[ -z "${CLIPPY_STRICT:-}" ] || fail=1` so only a value other than `0`
   counts as strict, e.g. `if [ "${CLIPPY_STRICT:-1}" != "0" ]; then fail=1; fi`.
2. Same file, final summary block: only print the "set CLIPPY_STRICT=1 to
   treat that as a failure" hint when strict mode is off; in strict mode the
   SKIPPED block already sets fail=1.
3. Update the stale comment in `.github/workflows/main.yml` (clippy job): the
   default is now strict; the sentence "CLIPPY_STRICT is not enabled: the
   native_posix-only tests ... are reported as skipped" no longer matches the
   script. Decide the CI intent: keep the strict default (CI should fail if an
   app stops building on the clippy board) and reword the comment to say so.
4. Update the AGENTS.md "Clippy" section: apps that cannot build on the
   clippy board **fail by default** (`CLIPPY_STRICT=1` default); `CLIPPY_STRICT=0`
   restores skip behavior.

**Local evaluation** (as executed):
- Clean pass: `cd ci && DOCKER_ARGS="-v /tmp/zr-clippy:/tmp/zephyr-rust-clippy" \
  RUST_VERSION=1.78.0 ZEPHYR_VERSION=3.7.0 ./build-cmd.sh ci/clippy.sh eeprom`
  with `CLIPPY_ARGS="-D warnings"` → `tests/eeprom: OK`, `clippy: OK`.
- Skip path: `CLIPPY_BOARD=nonexistent_board` (a guaranteed build failure;
  note tests/eeprom *does* build on qemu_cortex_m3 despite its whitelist).
  With `CLIPPY_STRICT=0` → SKIPPED + hint, exit 0; default strict → exit 1.
  Same pair for the common pass via `ci/clippy.sh lib`.
- Confirm nothing else regressed: `ci/clippy.sh lib eeprom`.

**Done when**: both strictness modes behave as documented, docs match the
script, and the strict clean pass is green.

**Notes from execution**:
- `ci/build-cmd.sh` does NOT propagate host environment variables into the
  container; pass them with `DOCKER_ARGS="... -e VAR=value"`.
- The 3.7.0 × native_posix exclusion is real but the stated cause is wrong:
  the failure is a Rust `E0463` (no `std` for the native_posix target), not
  a picolibc `posix_cheats.h` header issue. Samples build there too.
  Re-examine the exclusion comments (main.yml, build-all.sh) in Task 2.

## Task 2 — Add tests/* to the build matrix (build-all.sh + main.yml) — DONE (eca23b5)

**Why**: samples get 53 build jobs across 7 boards/3 versions; `tests/*` are
never built by the main matrix at all — only compiled by clippy on
qemu_x86/3.7 and built/run by sanitycheck on 2.3.0. A test regression on
qemu_cortex_m3 or on Zephyr 2.7.3/3.7 is invisible to CI.

**Context**: per-test board whitelists from `tests/*/testcase.yaml`:
  - tests/eeprom: qemu_x86
  - tests/posix-clock, tests/rust, tests/semaphore: qemu_x86, qemu_cortex_m3, native_posix
Version facts: all tests build on 2.3.0/2.7.3/3.7 for qemu boards; native_posix
builds on 2.3/2.7 only (3.7 native_posix is excluded from the whole matrix for
header issues). `west build` ignores whitelists, so the matrix itself must
enumerate only whitelisted boards.

**Steps**:
1. `ci/build-all.sh`: extend the matrix generator. Introduce a `TESTS`
   variable (default: `tests/rust tests/semaphore tests/posix-clock tests/eeprom`)
   and generate jobs with per-app board sets driven from testcase.yaml
   (hardcode the whitelists in the case statement or parse
   `platform_whitelist` with grep/sed — hardcoding with a comment pointing at
   testcase.yaml is simpler and matches the existing style). Apply the
   existing version exclusions: no native_posix on 3.7 (extend the existing
   `3.7.0-native_posix-*` pattern to tests), riscv never for tests.
   Consider renaming `SAMPLES` → `APPS` (or keep `SAMPLES` and add `TESTS`;
   pick one and update the trim-knob comment at the top of the file).
2. `.github/workflows/main.yml`: extend the `test:` matrix axis with the four
   tests and add matching exclude entries: tests × {qemu_cortex_r5,
   nucleo_l552ze_q, qemu_riscv32, qemu_riscv64} (all versions), and
   tests × native_posix × 3.7.0 (the existing 3.7-native_posix exclude only
   names samples/serial — extend it to the tests).
3. Keep `gen_jobs` in build-all.sh and the main.yml excludes in sync; after
   editing, diff the two by hand (list the yaml matrix from the file and
   compare against `gen_jobs` output).

**Local evaluation**:
- `cd ci && ZEPHYR_VERSIONS="3.7.0" BOARDS="qemu_x86 qemu_cortex_m3" \
  SAMPLES="tests/posix-clock tests/eeprom" ./build-all.sh` — verify the jobs
  generated match the intended (version × whitelisted-board) cross product
  and that `log/build/` contains per-job logs. Repeat for one 2.x container.
- Verify job counts: the expected full matrix after this change is
  53 existing + 27 test jobs (eeprom 1×3=3; each of rust/semaphore/posix-clock:
  qemu_x86×3 + qemu_cortex_m3×3 + native_posix×2 = 8) = 80 jobs.
- Sanity: build one test on a board *outside* its whitelist is NOT part of
  this change; do not add it.

**Done when**: `gen_jobs` output equals the yaml matrix (spot-check at least
one exclude and one test×board×version combo), the full
`./build-all.sh` passes, and both files commit together.

**Notes from execution**:
- Final matrix: 73 jobs (53 samples + 27 tests: eeprom 1×3, the other three
  tests 8 each). Verified by diffing gen_jobs output against a python render
  of the yaml matrix.
- Tests actually build on MORE boards than their testcase.yaml whitelist
  (rust/semaphore/posix-clock build on cortex_r5, riscv, and nucleo too);
  only eeprom fails outside qemu_x86 (no devicetree eeprom node →
  E0432 on the DT macro). The whitelist was kept as the matrix source
  per the TODO; widening it is a possible follow-up.
- Running build-all.sh locally requires RUST_VERSION to be set (it is not
  inherited from env.sh defaults into the run function's containers;
  without it the image tag is `zephyr-rust:3.7.0-` and the job fails).
- Full-matrix validation was done in two trimmed runs (3.7.0 all boards +
  eeprom/posix-clock; 2.7.3 all boards + all tests); all 45 jobs linked
  zephyr.elf. The remaining combos (2.3.0 tests, 3.7.0 rust/semaphore)
  are covered by the same code paths as the run combos.

## Task 3 — Turn on execution (CI Run step + optional RUN knob)

**Why**: nothing in CI executes anything (the `Run` step in main.yml is
disabled for every combination via the `include: run: false` default), while
the local AGENTS workflow treats build+run as the primary smoketest. The
per-combo `run`/`fails` plumbing already exists — use it instead of deleting it.

**Context**: `samples/rust-app` on qemu_x86 exits non-zero by design
(intentional page fault; success = the full "Hello from Rust userspace..."
console output before the fatal error), hence `fails: true`. QEMU boards are
runnable; nucleo_l552ze_q is real hardware (never run). Tests hang without a
timeout.

**Steps**:
1. `.github/workflows/main.yml`: replace the unconditional
   `include: [fails: false, run: false]` defaults with explicit include
   entries that enable running where intended, at minimum:
   `{board: qemu_x86, test: samples/rust-app, run: true, fails: true}`.
   Optionally add `timeout-minutes: 5` on the Run step.
2. `ci/build-all.sh`: add an opt-in `RUN=${RUN:-0}` knob; when set, after a
   successful build run the app inside the same job:
   `timeout 120 ninja -C /tmp/build run` for runnable boards (qemu_*, skip
   nucleo and non-RUN builds), treating `samples/rust-app`'s expected
   non-zero exit as success (grep the console output instead: pass requires
   the "Hello from Rust userspace" line, not the exit code).
3. Default `RUN=0` so plain `build-all.sh` stays build-only/CI-identical.

**Local evaluation**:
- Single run check:
  `cd ci && DOCKER_ARGS="-v /tmp/zr-dbg:/tmp/build" RUST_VERSION=1.78.0 \
  ZEPHYR_VERSION=3.7.0 ./build-cmd.sh bash -c "west build -d /tmp/build -p auto \
  -b qemu_x86 samples/rust-app && cd /tmp/build && (timeout 120 ninja run | \
  grep 'Hello from Rust userspace')"`.
  Note: `-t run` hangs for tests; only samples are enabled here.
- With the RUN knob: `RUN=1 ZEPHYR_VERSIONS=3.7.0 BOARDS=qemu_x86 \
  SAMPLES="samples/rust-app tests/posix-clock" ./build-all.sh` — rust-app run
  accepted (expected output + non-zero exit treated as success), posix-clock
  run completes under the timeout with `PROJECT EXECUTION SUCCESSFUL`.

**Done when**: a local build-all run with `RUN=1` validates sample and test
runs, and the yaml change matches the intended combos exactly (grep the
include list).

## Task 4 — Rewrite the AGENTS.md matrix guidance as an evaluated ladder

**Why**: the validation stages send agents from manual single builds straight
to the full 53-job matrix, point them at native_posix/3.7 for tests, and never
mention the build-all trim knobs or that sanitycheck is 2.3.0-only. Write the
final state only after tasks 1–3 land so the doc matches reality.

**Steps** (edit the "Validation workflow for changes" section):
1. Stage 1 (unchanged): build + run samples/rust-app on qemu_x86 in one
   container invocation. Note `-t run` is fine for samples.
2. Stage 2 (replace "expand the matrix"): for an app/test change, build the
   affected app across its whitelisted boards and all versions via the trim
   knobs, e.g. `ZEPHYR_VERSIONS="3.7.0 2.7.3 2.3.0" BOARDS="<testcase.yaml
   whitelist>" SAMPLES=tests/<name> ./build-all.sh`. For runnable checks wrap
   `ninja run` in `timeout` (never use `-t run` for tests).
3. Add a stage 2.5: when an app/test cfg-gates on the Zephyr version
   (`zephyr250`/`zephyr270`/`zephyr300`), lint it per version:
   `cd ci && DOCKER_ARGS="-v /tmp/zr-clippy:/tmp/zephyr-rust-clippy" \
   CLIPPY_ARGS="-D warnings" RUST_VERSION=1.78.0 ZEPHYR_VERSION=<ver> \
   ./build-cmd.sh ci/clippy.sh <app>` — cfg'd-out code is not type-checked,
   so clippy on one version does not cover the others.
4. Fix the native_posix guidance: kernel-object/syscall changes are exercised
   by running tests on native_posix in the **2.3.0/2.7.3** containers
   (3.7.0 native_posix cannot build in containers; the main matrix excludes
   it). Keep the existing `west build -b native_posix tests/<name>` example
   but state the container-version requirement.
5. Stage 3 stays: full `build-all.sh` + `sanitycheck.sh` (add: sanitycheck
   runs on 2.3.0 only, with the host toolchain; it is the oldest-version
   execution pass, so do not treat it as full-version validation) + full
   clippy.
6. Mention the Rust-version coupling for local runs: containers are per
   (Zephyr, Rust) image; local invocations must pass `RUST_VERSION`
   explicitly when the host has no rustc, and the CI image tag in main.yml is
   updated manually per `docs/rust-upgrade.md`.

**Local evaluation**: perform a no-op edit (e.g. touch a comment) in a test
crate and follow each documented command end-to-end in the container,
confirming each one behaves exactly as written.

**Done when**: every command quoted in AGENTS.md was executed verbatim during
evaluation and produced the stated outcome.

## Task 5 — (Optional) Single-source the matrix definition

**Why**: `gen_jobs` in ci/build-all.sh hand-duplicates main.yml's excludes;
they agree today but drift silently (this review found them in agreement only
because they were just written together).

**Steps** (pick the lightest workable option):
1. Extract the board/version/app list and exclusion rules into
   `ci/matrix.sh` (a sourced file exporting the same env vars gen_jobs
   consumes), and reference it from build-all.sh; add a comment in main.yml
   pointing at it as the source of truth. GitHub Actions cannot source shell,
   so yaml stays authoritative for CI — add a one-line comment in both files
   cross-referencing the other and the rule "update both".
2. Alternative if a script solution is wanted: a tiny CI job (or build-all
   step) that renders the matrix from ci/matrix.sh and diffs it against
   main.yml, failing on drift.

**Local evaluation**: run `ci/build-all.sh` gen_jobs after the refactor and
compare to the pre-refactor output (identical job list), plus one trimmed
run: `ZEPHYR_VERSIONS=3.7.0 BOARDS=qemu_x86 ./build-all.sh`.

**Done when**: job list is unchanged, and both files state where the matrix
lives.

## Task 6 — (Lowest priority) Parameterize sanitycheck.sh / move to twister

**Why last**: it only improves the oldest-version execution pass; tasks 2–3
give newer versions build and run coverage through the main matrix, which is
worth more. Sanitycheck is also the only CI piece that executes anything
today, so keep it working until twister parity is proven.

**Steps**:
1. Make the version a parameter: `ZEPHYR_VERSION=${ZEPHYR_VERSION:-2.3.0}`
   (keep 2.3.0 as the default so current behavior is unchanged).
2. Dispatch the runner by version: 2.3.0 uses
   `$ZEPHYR_BASE/scripts/sanitycheck`; 2.7.3 and 3.7.0 use
   `$ZEPHYR_BASE/scripts/twister` (verify inside each container with
   `ls $ZEPHYR_BASE/scripts/` before assuming; twister is a rewrite with
   mostly compatible flags — `-N -O <dir> -c --all -T <root>` — but confirm
   each flag in `twister --help` and adapt).
3. Keep the host toolchain (`-e ZEPHYR_TOOLCHAIN_VARIANT=zephyr`) for
   native_posix builds; note the toolchain skew vs. SDK builds in a comment.
4. Decide whether tests that only whitelist boards unavailable on a given
   version should be filtered per version (e.g. extend per-version excludes
   once the main matrix carries tests from task 2).

**Local evaluation**:
- Unchanged default: `cd ci && ./sanitycheck.sh` still runs the 2.3.0 pass
  and passes exactly as before (compare the test/pass counts).
- New version: `cd ci && ZEPHYR_VERSION=3.7.0 ./sanitycheck.sh` (after
  implementing twister dispatch) — expect it to work for qemu boards and to
  skip/fail native_posix per the known 3.7 native_posix issues; if
  native_posix cannot work on 3.7, filter it and document why.
- Confirm a real failure is caught: intentionally build a broken test
  locally, run the parameterized script for the affected version, and confirm
  it reports FAIL and exits non-zero (do not commit the breakage).

**Done when**: the script runs for all three versions (or documents why a
version cannot), defaults preserve today's 2.3.0 behavior, and failures are
reported with a non-zero exit.
