# Upgrading zephyr-rust to a new Rust version

This document is a repeatable, step-by-step guide for porting `zephyr-rust`
to a new Rust toolchain. A repo revision supports exactly one Rust version:
the `rust/rust` submodule (the std/no_std port) must match the compiler
exactly, so a port is a coordinated change across `rust/rust`,
`rust/libc`, the pinned toolchain, and the CI images.

## Step 0: Verify the current state is buildable

Before touching anything, confirm the tree builds and runs at the *current*
pinned versions. This is the baseline every later diff is judged against,
and it fails fast on unrelated breakage (stale submodule, environment
drift).

1. Confirm submodules are in sync:

   ```sh
   git submodule status --recursive
   git diff --submodule
   ```

   The base must be the submodule rev (local commits above it are allowed).

2. Confirm the CI container image for the current
   `ZEPHYR_VERSION`-`RUST_VERSION` exists locally (or is pullable), e.g.
   `zephyr-rust:3.7.0-1.75.0`. `ci/env.sh` resolves `RUST_VERSION` from
   `rust-toolchain.toml` and `ZEPHYR_VERSION` from its default (overridable
   via env).

3. Build and run the default sample on the default board in one
   container invocation, persisting the build dir:

   ```sh
   cd ci
   DOCKER_ARGS="-v /tmp/zr-smoke:/tmp/build" \
       ./build-cmd.sh west build -d /tmp/build -p auto -b qemu_x86 samples/rust-app
   DOCKER_ARGS="-v /tmp/zr-smoke:/tmp/build" \
       ./build-cmd.sh ninja -C /tmp/build run
   ```

4. **Pass criteria**: a clean build, console output through
   `Next call will crash if userspace is working.`, followed by the
   *intentional* page fault (`ZEPHYR FATAL ERROR 0: CPU exception`,
   "Access violation: user thread not allowed to read"). The non-zero exit
   is by design; it proves user-mode isolation. Anything before the hello
   output missing, or a different fault, is a failure — stop and fix the
   baseline before starting the port.

## Step 1: Rebase the `rust/rust` port onto the new release tag

The `rust/rust` submodule carries the zephyr std port as a series of
commits on top of a release tag (e.g. `1.75.0`). Porting to the next
version means rebasing those commits onto the next tag (e.g. `1.76.0`).
The commits are written to be upstream-quality (no hacks); when they
conflict with upstream changes, adapt the port to the new upstream code
rather than papering over the conflict.

1. In `rust/rust`, confirm the base and the port commits:

   ```sh
   git log --oneline <old-tag>..HEAD        # the port commits
   git tag | grep -E '^1\.7[56]\.0'         # the new tag exists locally
   ```

2. Rebase onto the new tag. The branch name is of the form
   zephyr-<rust_version>, e.g. "zephyr-1.75.0"

   ```sh
   git checkout -b zephyr-<new> # start from the current submodule rev
   git rebase --onto <new-tag> <old-tag>
   ```

   e.g.
   ```sh
   git checkout -b zephyr-1.76.0 # start from the current submodule rev
   git rebase --onto 1.76.0 1.75.0
   ```

3. Resolve conflicts commit by commit with `git rebase --continue`.
   Conflict classes observed in the 1.75.0 -> 1.76.0 port:

   - **Submodule removal commit** (`rust: remove submodules not required
     to build zephyr-rust`): the doc/tool submodules our commit deletes
     get new upstream commits in the new tag, producing modify/delete
     conflicts. Resolution is always "keep our deletion":
     `git rm <paths>` for each, then continue. (If upstream *added*
     submodules we also don't need, extend this commit's deletion list.)
   - **`library/std/src/sys/mod.rs`**: upstream keeps adding new target
     branches to the `cfg_if` chain between `sgx` and the `unsupported`
     fallback (e.g. `teeos` in 1.76.0). Keep the new upstream branches
     and keep our `zephyr` branch immediately before `unsupported`.
   - **`library/std/src/sys_common/mod.rs`**: in 1.76.0 the net-module
     condition was *inverted* to enumerate platforms that have their own
     `net` (`all(unix, not(l4re)), windows, hermit, solid_asp3`).
     Zephyr targets are `target_family = "unix"`, so they now match the
     first arm and would require a non-existent `sys::zephyr::net`.
     Exclude zephyr explicitly:
     `all(unix, not(target_os = "l4re"), not(target_os = "zephyr"))`.
     Audit the other arms of this file (and any similar inverted cfgs)
     the same way on every port.

4. Sanity-check the result: the diff from the new tag should contain
   only zephyr-port changes:

   ```sh
   git diff <new-tag> --stat
   grep -rn '<<<<<<<' library/std/src/sys/mod.rs library/std/src/sys_common/mod.rs
   ```

5. Check `rust/libc`: the libc version required by the new std is in
   `library/std/Cargo.toml` (`libc = { version = ... }`). Compare with
   `git describe --tags` in `rust/libc`. For 1.76.0 std requires
   `0.2.150` and the existing `rust/libc` port was already based on
   `0.2.150`, so no change was needed. If the port is based on an older
   libc, rebase it the same way (its commits are also upstream-quality)

6. Update the parent repo in the same commit:

   - `rust-toolchain.toml`: `channel ="`
   - `rust/build.sh`: the `VERSION=` rustc-version assertion
   - `README.md`: version references ("exactly 1.76.0",
     "stable-1.76.0", `rustup toolchain install 1.76.0`)
   - the `rust/rust` submodule pointer to the new `zephyr-<new>` tip
   - container versions in `.github/workflows/*.yml`

7. Rebuild the CI container image for the new pair. **Gotcha**: `env.sh`
   resolves `RUST_VERSION` from the host's `rustc --version`, which
   triggers a rustup auto-install of the newly pinned toolchain. If the
   host can't write to `~/.rustup` (e.g. sandbox), `RUST_VERSION` comes
   out empty and the container build fails deep inside rustup with
   "a value is required for '--default-toolchain'". Workaround: pass the
   version explicitly, which skips the host probe:

   ```sh
   cd ci && RUST_VERSION=1.76.0 ZEPHYR_VERSION=3.7.0 ./container-build.sh
   ```

8. Quick validation (fail fast): build + run the default sample in the
   new image, using a *fresh* build dir (the old one caches the 1.75
   sysroot):

   ```sh
   cd ci
   RUST_VERSION=1.76.0 DOCKER_ARGS="-v /tmp/zr-smoke-176:/tmp/build" \
       ./build-cmd.sh west build -d /tmp/build -p auto -b qemu_x86 samples/rust-app
   RUST_VERSION=1.76.0 DOCKER_ARGS="-v /tmp/zr-smoke-176:/tmp/build" \
       ./build-cmd.sh ninja -C /tmp/build run
   ```

   **Gotcha**: the first build fails with
   `error: failed to write .../rust/sysroot-stage1/Cargo.lock` because
   the repo is mounted read-only and std's dependency graph changed
   (1.76.0 added `unwinding`/`gimli`). Rerun with `WRITABLE=1`. The
   resulting `Cargo.lock` diff is a real change and is committed with
   the port.

   Pass criteria: same as Step 0 (full hello output, then the
   intentional page fault).
