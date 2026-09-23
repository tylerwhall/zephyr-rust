# Upgrading zephyr-rust to a new Rust version

A repo revision supports exactly one Rust version: the `rust/rust`
submodule (the std port) must match the compiler exactly. A port is a
coordinated change across `rust/rust`, `rust/libc`, the version pins, and
the CI images. Per-upgrade decisions and conflicts are recorded in
`docs/rust-upgrade-history.md`.

## Overview

1. [Baseline](#1-baseline) — current tree builds and runs
2. [Rebase the rust/rust port](#2-rebase-the-rustrust-port)
3. [Update zephyr-rust](#3-update-zephyr-rust) — pins, workflows, submodule pointer
4. [Build and validate](#4-build-and-validate)
5. [Record the upgrade](#5-record-the-upgrade)
6. [Push](#6-push) — print the commands for the user

## 1. Baseline

1. Submodules in sync: `git submodule status --recursive`,
   `git diff --submodule`. The base must be the submodule rev.
2. CI image for the current `ZEPHYR_VERSION`-`RUST_VERSION` exists locally
   or is pullable. `ci/env.sh` resolves `RUST_VERSION` from
   `rust-toolchain.toml`.
3. Build + run the default sample on the default board, persisting the
   build dir. Do not stream complete build output into the agent context:
   redirect verbose commands to a temporary log and inspect only the exit
   status plus a concise tail or targeted error excerpts.

   ```sh
   cd ci
   DOCKER_ARGS="-v /tmp/zr-smoke:/tmp/build" \
       ./build-cmd.sh west build -d /tmp/build -p auto -b qemu_x86 samples/rust-app \
       > /tmp/zr-baseline.log 2>&1
   rc=$?; tail -100 /tmp/zr-baseline.log; exit $rc
   DOCKER_ARGS="-v /tmp/zr-smoke:/tmp/build" \
       ./build-cmd.sh ninja -C /tmp/build run \
       > /tmp/zr-baseline-run.log 2>&1
   rc=$?; tail -100 /tmp/zr-baseline-run.log; exit $rc
   ```

4. **Pass criteria** (used for all later validation): clean build, console
   output through `Next call will crash if userspace is working.`, then the
   *intentional* page fault (`ZEPHYR FATAL ERROR 0: CPU exception`,
   "Access violation: user thread not allowed to read"). The non-zero exit
   is by design. Anything else is a failure — stop and fix the baseline.
5. If the new containers are already on ghcr.io, start pulling them in the
   background now so the download overlaps with the rebase:

   ```sh
   for v in <zephyr versions>; do
       docker pull ghcr.io/<registry>/zephyr-rust:zephyr-rust-$v-<new> &
   done
   ```

   Pull the ghcr-prefixed names: [Phase 4](#4-build-and-validate) forces
   `CONTAINER_IMAGE_PREFIX` to the ghcr prefix, so the images must exist under
   those names.

## 2. Rebase the rust/rust port

The submodule carries the port as upstream-quality commits on top of a
release tag. When a commit conflicts with upstream changes, adapt the port
to the new upstream code; no hacks.

1. Confirm base and port commits:

   ```sh
   git log --oneline <old-tag>..HEAD        # the port commits
   git tag | grep -E '^1\.[0-9]+\.0'        # confirm the required tags exist locally
   ```

2. Rebase onto the new tag. Branch names are `zephyr-<rust_version>`:

   ```sh
   git checkout -b zephyr-<new>            # from the current submodule rev
   git rebase --onto <new-tag> <old-tag>
   ```

3. Resolve conflicts with `git rebase --continue`. Preserve upstream
   changes while retaining the Zephyr port's behavior. For modify/delete or
   file-location conflicts, keep intentional port deletions and adapt the
   port to the new upstream layout rather than restoring obsolete structure.
   Audit relative paths, module registration, cfg conditions, and generated
   metadata after structural changes. Record non-obvious decisions in the
   history document.

4. Sanity-check: `git diff <new-tag> --stat` should show only zephyr-port
   changes; no leftover conflict markers.

5. Check `rust/libc`: the required version is in `library/std/Cargo.toml`
   (`libc = { version = ... }`); compare with `git describe --tags` in
   `rust/libc`. If the port is based on an older libc, rebase it the same
   way (its commits are also upstream-quality).

## 3. Update zephyr-rust

In the parent repo, in the same commit as the submodule pointer bump:

- `rust-toolchain.toml`: `channel = "<new>"`
- `rust/build.sh`: the `VERSION=` rustc-version assertion
- `README.md`: version references ("exactly ...", "stable-...",
  `rustup toolchain install ...`)
- `.github/workflows/*.yml`: container image tags and the
  `container-build.yml` default input
- `rust/rust` submodule pointer to the new `zephyr-<new>` tip

Sweep for stragglers before committing:
`grep -rn <old-version> --exclude-dir=.git --exclude-dir=rust/rust
--exclude-dir=rust/libc --exclude-dir=build .` (ignore historical
mentions in docs).

## 4. Build and validate

The default smoke test is required during the upgrade. The full matrix and
repository tests remain required before final submission, according to the
change type described in `AGENTS.md`.

1. If Phase 1 started background pulls, confirm they finished
   (`docker image inspect ghcr.io/<registry>/zephyr-rust:zephyr-rust-<v>-<new>`)
   before building. A still-running pull is not a correctness problem —
   docker blocks on it — but the pre-pull avoids that wait.
2. If the image is not on ghcr.io, build it locally. **Gotcha**: `env.sh` resolves
   `RUST_VERSION` from the host's `rustc --version`, which triggers a
   rustup auto-install of the newly pinned toolchain. If the host can't
   write to `~/.rustup` (e.g. sandbox), `RUST_VERSION` comes out empty and
   the build fails deep in rustup with "a value is required for
   '--default-toolchain'". Pass the version explicitly to skip the probe:

   ```sh
   cd ci && RUST_VERSION=<new> ZEPHYR_VERSION=<ver> ./container-build.sh
   ```

3. Build + run the default sample in the new image, using a **new build
   directory** (an old one caches the previous sysroot). Prefer a new uniquely
   named temporary directory rather than deleting an old Docker-created
   directory, which may contain root-owned files. Redirect verbose commands to
   a temporary log and report only the exit status and a concise tail or
   targeted error excerpts. Pass criteria: as in [Baseline](#1-baseline).

   ```sh
   cd ci
   export RUST_VERSION=<new>
   export CONTAINER_IMAGE_PREFIX=ghcr.io/<registry>/zephyr-rust:zephyr-rust-
   DOCKER_ARGS="-v /tmp/zr-smoke-new:/tmp/build" \
       ./build-cmd.sh west build -d /tmp/build -p auto -b qemu_x86 samples/rust-app \
       > /tmp/zr-new.log 2>&1
   rc=$?; tail -100 /tmp/zr-new.log; exit $rc
   DOCKER_ARGS="-v /tmp/zr-smoke-new:/tmp/build" \
       ./build-cmd.sh ninja -C /tmp/build run \
       > /tmp/zr-new-run.log 2>&1
   rc=$?; tail -100 /tmp/zr-new-run.log; exit $rc
   ```

   `CONTAINER_IMAGE_PREFIX` forces the ghcr image names (matching the
   Phase 1 pulls) instead of letting `env.sh` prefer a stale local image.
   Omit this override when validating against the local image produced by
   `container-build.sh`.

   **Gotcha**: the first build may fail with
   `error: failed to write .../rust/sysroot-stage1/Cargo.lock` — the repo
   is mounted read-only and std's dependency graph changed. Rerun with
   `WRITABLE=1`; the lockfile diff is a real change, committed with the
   port. Check that updated dependencies are compatible with the pinned Cargo;
   if resolution selects a dependency requiring a newer Cargo, resolve it to
   a compatible version explicitly and record the decision.

4. **Compile errors in the port** (upstream std/core API churn): identify the
   original port commit that introduced the affected code. Fix one error at a
   time in a separate commit. Use `git commit --fixup=<original-rev>` when the
   fix belongs to an earlier port commit; use a normal commit with a proper
   message when it is independent. Continue building after each fix, but stop
   and ask when the correct adaptation or intended behavior is unclear. Do not
   autosquash until the build succeeds. Then autosquash the fixups, resolve
   any autosquash conflicts in favor of the final validated state, and verify
   the resulting port history.

## 5. Record the upgrade

Add a section to `docs/rust-upgrade-history.md` (one per upgrade, e.g.
`## 1.75.0 → 1.76.0`) recording every important decision and conflict,
the libc decision, compile errors and their fixes, and deviations from this
process. Write it after autosquashing, once the final submodule tip is known;
commit it separately from the port.

## 6. Push

The agent cannot push to the github `origin` remotes. End the process by
printing the commands for the user to run, in dependency order (submodule
branches before the parent repo), covering whatever changed: `rust/rust`
(`zephyr-<new>`, and `zephyr-<old>` if it was rewritten), `rust/libc` if
touched, and the parent repo. Mention that container image(s) need to be built
in CI if they were not pulled from github.
