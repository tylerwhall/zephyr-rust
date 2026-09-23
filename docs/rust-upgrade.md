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
   build dir:

   ```sh
   cd ci
   DOCKER_ARGS="-v /tmp/zr-smoke:/tmp/build" \
       ./build-cmd.sh west build -d /tmp/build -p auto -b qemu_x86 samples/rust-app
   DOCKER_ARGS="-v /tmp/zr-smoke:/tmp/build" \
       ./build-cmd.sh ninja -C /tmp/build run
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
   git tag | grep -E '^1\.7[56]\.0'         # the new tag exists locally
   ```

2. Rebase onto the new tag. Branch names are `zephyr-<rust_version>`:

   ```sh
   git checkout -b zephyr-<new>            # from the current submodule rev
   git rebase --onto <new-tag> <old-tag>
   ```

3. Resolve conflicts with `git rebase --continue`. Known conflict classes
   (with per-version detail in the history doc):

   - **Submodule removal commit**: upstream moves the pointers of submodules
     we deleted → modify/delete conflicts. Always keep our deletions
     (`git rm <paths>`). Extend the deletion list if upstream added
     submodules we don't need.
   - **`library/std/src/sys/mod.rs`**: upstream adds target branches to the
     `cfg_if` chain. Keep the new upstream branches; keep our `zephyr`
     branch immediately before `unsupported`.
   - **`library/std/src/sys_common/mod.rs`**: the net-module condition was
     inverted in 1.76.0 to enumerate platforms *with* their own `net`.
     Zephyr targets are `target_family = "unix"`, so exclude zephyr
     explicitly. Audit similar inverted cfgs on every port.

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

3. Build + run the default sample in the new image, **fresh build dir**
   (an old one caches the previous sysroot). Pass criteria: as in
   [Baseline](#1-baseline).

   ```sh
   cd ci
   export RUST_VERSION=<new>
   export CONTAINER_IMAGE_PREFIX=ghcr.io/<registry>/zephyr-rust:zephyr-rust-
   DOCKER_ARGS="-v /tmp/zr-smoke:/tmp/build" \
       ./build-cmd.sh west build -d /tmp/build -p auto -b qemu_x86 samples/rust-app
   DOCKER_ARGS="-v /tmp/zr-smoke:/tmp/build" \
       ./build-cmd.sh ninja -C /tmp/build run
   ```

   `CONTAINER_IMAGE_PREFIX` forces the ghcr image names (matching the
   Phase 1 pulls) instead of letting `env.sh` prefer a stale local image.

   **Gotcha**: the first build may fail with
   `error: failed to write .../rust/sysroot-stage1/Cargo.lock` — the repo
   is mounted read-only and std's dependency graph changed. Rerun with
   `WRITABLE=1`; the lockfile diff is a real change, committed with the
   port.

4. **Compile errors in the port** (upstream std/core API churn): fix each
   error as a *separate commit on top of the series* — one commit per
   error, upstream-quality, no history rewriting. Then **stop and wait**
   for user instructions on how to fold the fixes back into the series.
   The first occurrence of this is a learning exercise: record it in the
   history doc so the instructions can be refined.

## 5. Record the upgrade

Add a section to `docs/rust-upgrade-history.md` (one per upgrade, e.g.
`## 1.75.0 → 1.76.0`) recording every important decision and conflict:
which commits conflicted and how/why they were resolved, the libc decision,
any compile errors and their fixes, and any deviation from this process.
Commit it separately from the port.

## 6. Push

The agent cannot push to the github `origin` remotes. End the process by
printing the commands for the user to run, in dependency order (submodule
branches before the parent repo), covering whatever changed: `rust/rust`
(`zephyr-<new>`, and `zephyr-<old>` if it was rewritten), `rust/libc` if
touched, and the parent repo. Mention that container image(s) need to be built
in CI if they were not pulled from github.
