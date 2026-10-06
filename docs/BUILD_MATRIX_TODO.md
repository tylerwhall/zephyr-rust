# Build/test coverage and remaining work

`ci/matrix.py` is the single source for GitHub Actions and `ci/build-all.sh`.
It currently produces 113 build jobs across Zephyr 2.3.0/2.7.3/3.7.0, with
six verified sample runs. Test board sets come from testcase.yaml; sample
board rules and version exclusions live in the matrix generator.

The main workflow builds the matrix, runs selected samples, and runs strict
Clippy on 3.7.0/qemu_x86. It does not yet execute the repository test suite.
Local `ci/sanitycheck.sh` executes seven configurations on 2.3.0, restricted
to qemu_x86 and qemu_cortex_m3. Later-version tests remain build-only.

## Run and reproduction rules

- Only rust-app and no_std on qemu_x86 have verified automatic exits on all
  three Zephyr versions. Each reaches `Next call will crash if userspace is
  working.` and exits 1 from an intentional user-mode fault. Assert output
  separately from status. Other combinations stay build-only; serial waits
  for input and the other QEMU combinations did not exit in the inventory.
- Use `ci/run-sample.sh`: it owns an emulator process group and cleans it up
  on exit/timeout. Never substitute a bare ninja run/timeout pipeline.
- Run containers with explicit `RUST_VERSION=1.85.0 ZEPHYR_VERSION=<ver>`.
  The repo is read-only by default; pass container variables via
  `DOCKER_ARGS="... -e VAR=value"` and persist build directories with volumes.
- Trim builds with APPS, BOARDS, and ZEPHYR_VERSIONS. Results are under
  `ci/log/build`; --resume skips completed jobs. Move that directory aside
  for a fresh run. Use separate Clippy volumes per Zephyr version.
- RISC-V matrix boards are 3.x-only; serial does not support native_posix's
  UART configuration. Native_posix/3.7.0 remains excluded: the current
  picolibc C build passes posix_cheats.h as an extra input and GCC rejects
  -o with multiple files, before Rust runs. The old missing-std explanation
  predates build-std and is no longer the observed failure.

## Add Zephyr 2.3.0 test execution to CI

Add a separate required Actions job using the pinned 2.3.0 image, preserving
the current sanitycheck command, board scope, and exit status. The script is
an outer Docker launcher; inside an Actions container invoke the Zephyr
runner directly rather than nesting Docker:

```sh
ZEPHYR_TOOLCHAIN_VARIANT=zephyr "$ZEPHYR_BASE/scripts/sanitycheck" \
  -N -O /tmp/sanity-out -c -p qemu_x86 -p qemu_cortex_m3 -T tests
```

Validate that all seven configurations execute, an intentional assertion
failure fails the job, and runner logs are retained. Do not describe this
job as later-version test coverage or broaden its scope in the same change.

## Execute later-version tests with twister

Inspect each pinned 2.7.3/3.7.0 runner's help and testcase schema before
implementing dispatch. Preserve 2.3.0 sanitycheck unchanged; let twister own
completion recognition, emulator lifecycle, timeouts, and cleanup.

Start with one test on qemu_x86 and qemu_cortex_m3 per version, then expand
according to testcase.yaml and validated platform support. Verify pass,
build failure, assertion failure, timeout, and startup failure all propagate
correctly and leave no QEMU/native descendants. Add separate Actions jobs
only after those checks pass; keep test execution separate from sample runs.

## Revisit excluded platforms without weakening validation

- Fix and validate the 3.7.0 native_posix/picolibc C build before removing
  the matrix exclusion. Build-std alone does not establish platform support.
- The 2.3.0 sanitycheck runner hardcodes -Werror and --fatal-warnings.
  Native_posix's kernel noinit attribute conflict and cortex_r5's DT_TEXTREL
  warning become fatal only under that runner; plain matrix builds pass.
  Broaden execution only after fixing the causes or deliberately evaluating
  runner-specific handling, without masking genuine application warnings.
- RISC-V is not a supported 2.3.0 matrix target. Do not add it to the older
  runner merely because it appears in a test's cross-version whitelist.
