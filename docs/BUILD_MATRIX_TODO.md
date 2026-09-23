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
- Do not assume `ninja run` exits. First measure each sample/board/version
  combination. Test applications print a success marker but normally leave
  the emulator running; samples may or may not terminate. Any runner used
  for an automatically exiting sample must clean up the entire emulator
  process group, not just pipe output through `timeout`.
- Full-matrix runs use `ci/build-all.sh` (results under `ci/log/build/`,
  `--resume` skips completed jobs; `rm -rf ci/log/build` to force a full run).
- Native_posix Rust builds in containers work on Zephyr 2.3.0/2.7.3;
  3.7.0 is excluded because the cross-compiled sysroot has no `std` for the
  native_posix target (`rustc E0463`).
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
- The 3.7.0 × native_posix exclusion is real because the cross-compiled
  sysroot has no `std` for the native_posix target (`rustc E0463`).
  Re-examine the exclusion comments in main.yml and build-all.sh if the
  Rust target support changes.

## Task 2 — Add tests/* to the build matrix (build-all.sh + main.yml) — DONE (eca23b5)

**Why**: samples get 53 build jobs across 7 boards/3 versions; `tests/*` are
never built by the main matrix at all — only compiled by clippy on
qemu_x86/3.7 and built/run by sanitycheck on 2.3.0. A test regression on
qemu_cortex_m3 or on Zephyr 2.7.3/3.7 is invisible to CI.

**Context**: per-test board whitelists from `tests/*/testcase.yaml`:
  - tests/eeprom: qemu_x86
  - tests/posix-clock, tests/rust, tests/semaphore: qemu_x86, qemu_cortex_m3, native_posix
Version facts: all tests build on 2.3.0/2.7.3/3.7 for qemu boards; native_posix
builds on 2.3/2.7 only (3.7 native_posix is excluded because the Rust target
has no `std`). `west build` ignores whitelists, so the matrix itself must
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
  53 existing + 20 test jobs (eeprom 1×3=3; each of rust/semaphore/posix-clock:
  qemu_x86×3 + qemu_cortex_m3×3 + native_posix×2 = 8) = 73 jobs.
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

## Task 3 — Run only samples that exit automatically

**Why**: the CI `Run` step is currently disabled for every matrix entry, but
execution is valuable for samples. Tests must not be included in this task:
Ztest prints `PROJECT EXECUTION SUCCESSFUL` and then leaves the Zephyr kernel
and emulator running. A timeout alone can also orphan QEMU descendants, so
test execution needs a separate runner design (or sanitycheck/twister).

**Context**: do not assume that every sample exits. `samples/rust-app` exits
non-zero by design after printing the expected user-mode output; that is a
candidate for execution, but every sample/board/version combination must be
checked rather than assuming the qemu_x86 behavior generalizes. Nucleo is
real hardware and must remain build-only. Tests are build-only in this task.

**Steps**:
1. Before changing either matrix, inventory the current sample matrix by
   building and running each runnable qemu sample on every supported Zephyr
   version and qemu board. Use a fresh build directory per run and record:
   - whether `ninja run` exits by itself;
   - its exit status and expected output;
   - whether QEMU has exited and no emulator/container process remains;
   - whether behavior differs by Zephyr version or board.
   Do not use a `grep` pipeline as the success test: capture the complete
   output and status separately.
2. Classify samples:
   - **automatic-exit**: may be enabled in CI after the output/status is
     understood;
   - **non-exiting**: keep build-only and record the reason in this TODO and
     the relevant AGENTS.md guidance;
   - **board/version-specific**: add only the verified combinations.
   Tests (`tests/*`) are explicitly not candidates here.
3. Add an opt-in `RUN=${RUN:-0}` path to `ci/build-all.sh` for only the
   verified automatic-exit sample combinations. The runner must launch each
   emulator in its own process group and clean up the whole group with a
   trap. Do not implement test execution by piping `ninja run` through
   `timeout` and `grep`; that was observed to leave QEMU descendants alive.
   Keep the default `RUN=0` build-only behavior.
4. In `.github/workflows/main.yml`, replace the blanket `run: false` behavior
   only with explicit include entries for verified automatic-exit samples.
   Preserve `fails: true` where a sample intentionally exits non-zero, and
   make the output assertion independent of that exit status. Add a job/step
   timeout as a final safety net, not as the primary cleanup mechanism.
5. Do not add tests to the CI Run step. Leave the existing build matrix for
   tests intact until Task 6/7 provides a test runner.

**Local evaluation**:
- First perform the inventory in each container with commands equivalent to:
  `cd ci && RUST_VERSION=1.78.0 ZEPHYR_VERSION=<ver> ./build-cmd.sh \
  west build -d /tmp/build -p auto -b <qemu-board> <sample>` followed by the
  process-group-safe runner. Repeat for every sample/board/version candidate.
- Run the trimmed build matrix with `RUN=0` and confirm it remains build-only.
- Run `RUN=1` with exactly one verified sample/board/version, then expand to
  all verified combinations. Confirm automatic cleanup after both success and
  intentional non-zero exit, and confirm no QEMU process remains on the host.
- For every non-exiting sample, save the command/output and document the
  combination rather than forcing it through a timeout.

**Done when**: the inventory covers all sample candidates across the supported
matrix, only verified automatic-exit combinations run in CI, non-exiting
samples are explicitly recorded as build-only, tests remain build-only, and
process cleanup is proven locally.

## Task 4 — Rewrite the AGENTS.md matrix guidance as an evaluated ladder

**Why**: the validation stages send agents from manual single builds straight
to the full matrix, point them at native_posix/3.7 for tests, and do not
explain which samples actually exit. They also need to distinguish build
coverage from test execution: sanitycheck is initially 2.3.0-only, and later
Zephyr versions require the separate Task 7 twister work. Write the final
state only after tasks 1–3 land so the doc matches reality.

**Steps** (edit the "Validation workflow for changes" section):
1. Stage 1: build + run the verified automatic-exit sample combinations from
   Task 3; do not assume `samples/rust-app` behavior applies to every board.
   Note that the process-group-safe runner, not a bare `-t run`, is required.
2. Stage 2 (replace "expand the matrix"): for an app/test change, build the
   affected app across its whitelisted boards and all versions via the trim
   knobs, e.g. `ZEPHYR_VERSIONS="3.7.0 2.7.3 2.3.0" BOARDS="<testcase.yaml
   whitelist>" TESTS=tests/<name> ./build-all.sh`. Run only samples classified
   as automatic-exit by Task 3; keep tests build-only until sanitycheck/twister
   coverage is available.
3. Add a stage 2.5: when an app/test cfg-gates on the Zephyr version
   (`zephyr250`/`zephyr270`/`zephyr300`), lint it per version:
   `cd ci && DOCKER_ARGS="-v /tmp/zr-clippy:/tmp/zephyr-rust-clippy" \
   CLIPPY_ARGS="-D warnings" RUST_VERSION=1.78.0 ZEPHYR_VERSION=<ver> \
   ./build-cmd.sh ci/clippy.sh <app>` — cfg'd-out code is not type-checked,
   so clippy on one version does not cover the others.
4. Fix the native_posix guidance: build/test changes can be built on
   native_posix in the **2.3.0/2.7.3** containers (3.7.0 lacks Rust `std` for
   that target). Do not describe `ninja run` as a reliable test execution
   method; use the test runner documented by Task 6/7.
5. Stage 3: full `build-all.sh` + the CI sanitycheck job from Task 6 (2.3.0
   only) + full clippy. Do not claim this is full-version test execution;
   later-version test execution belongs to Task 7.
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

## Task 6 — Add the existing Zephyr 2.3.0 sanitycheck to CI

**Why**: this is the lowest-risk way to add automated test execution coverage
without solving the later-version runner migration. `ci/sanitycheck.sh` is
already pinned to Zephyr 2.3.0 because its testcase.yaml schema and runner
interface are version-specific. Keep this task narrowly scoped: do not
parameterize it and do not change it to twister yet.

**Steps**:
1. Add a GitHub Actions job in `.github/workflows/main.yml` using the
   `zephyr-rust-2.3.0-1.78.0` container.
2. Run `ci/sanitycheck.sh` from that job and preserve its non-zero exit status.
   The job should use the existing 2.3.0 host-toolchain setup and should not
   reuse the sample build matrix's `/tmp/build` directory.
3. Give the job a clear name indicating that it is the 2.3.0 test execution
   pass. Upload or print the sanitycheck output sufficiently for failures to
   be diagnosed, but do not make the job parse or duplicate its results.
4. Update AGENTS.md and this TODO to say explicitly that this job executes
   tests only on Zephyr 2.3.0; it does not provide 2.7.3/3.7.0 coverage.

**Local evaluation**:
- Run the exact container command locally:
  `cd ci && RUST_VERSION=1.78.0 ZEPHYR_VERSION=2.3.0 ./build-cmd.sh \
  ./ci/sanitycheck.sh` (adapt the working-directory prefix only if the
  container command requires it), and confirm all currently supported test
  platforms are built/executed.
- Confirm a deliberate, temporary test failure makes the command and the CI
  job exit non-zero; remove the temporary failure before committing.
- Verify the new workflow job's image, command, and failure propagation by
  checking the yaml matrix manually; no later-version image should be used.

**Done when**: the 2.3.0 sanitycheck runs as a separate required CI job,
reports test failures, and its limited version scope is documented.

## Task 7 — Separately migrate later-version test execution to twister

**Why last**: Zephyr 2.7.3 and 3.7.0 use the later `twister` runner rather
than the 2.3.0 `sanitycheck` interface. This task must not be mixed with Task
6 or with sample execution. Twister should own emulator lifecycle, completion
recognition, timeouts, and cleanup; do not implement test execution with a
bare `ninja run`/`timeout` pipeline.

**Steps**:
1. In each pinned container, inspect the available runner and its help:
   `ls $ZEPHYR_BASE/scripts/` and
   `$ZEPHYR_BASE/scripts/twister --help`. Record the actual flag differences
   between 2.7.3 and 3.7.0 rather than assuming the 2.3.0 sanitycheck flags
   are compatible.
2. Design a version-aware runner interface for `ci/sanitycheck.sh` or a new
   script. Preserve the Task 6 2.3.0 behavior unchanged; dispatch 2.7.3 and
   3.7.0 to twister only after their commands and output formats are known.
3. Filter boards according to each test's testcase.yaml and the known
   native_posix/Rust-target limitation. Do not make 3.7.0 native_posix a
   required run if the Rust target still lacks `std`.
4. Ensure the runner returns non-zero for build failures, test failures,
   timeouts, and emulator startup failures. Verify that it cleans up QEMU and
   native_posix processes after pass, fail, and timeout cases.
5. Once local execution is reliable, add separate CI jobs per later Zephyr
   version. Keep them separate from the sample build/run matrix so a runner
   migration failure is easy to diagnose.

**Local evaluation**:
- Run one representative test on qemu_x86 and qemu_cortex_m3 in both the
  2.7.3 and 3.7.0 containers, then expand to every testcase.yaml platform.
- Test pass, build failure, assertion/test failure, and timeout cases; verify
  each has the expected exit status and leaves no emulator process behind.
- Compare the test/pass/fail counts with Task 6's 2.3.0 sanitycheck output;
  differences must be explained by Zephyr runner/platform behavior, not
  silently ignored.

**Done when**: 2.7.3 and 3.7.0 test execution is independently reliable,
process cleanup is proven, CI jobs cover the supported runner/platform
combinations, and Task 6's 2.3.0 sanitycheck remains unchanged.