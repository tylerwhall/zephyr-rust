# Rust upgrade history

Running log of zephyr-rust Rust version upgrades: every important decision
and conflict, per `docs/rust-upgrade.md`. Newest first.

## 1.79.0 → 1.80.0 (2026-09-30)

**Result**: 16 port commits rebased from `1.79.0` onto `1.80.0`, branch
`zephyr-1.80.0`, final tip `8ab4e3b06aa03faf93d2a2ed9e22ea06fe14d2f5`.
The old `zephyr-1.79.0` branch was not rewritten. Review removed the extra
generic OS-TLS adaptation from the new branch in favor of a local
`zephyr-futures` initializer change.
`rust/libc` remains at `9c4f7e0888a8fbb1e0bfaa48b1bb566f1ebcaa99`:
Rust 1.80.0 still requires `0.2.153`, matching the existing six-commit port.

**Validation checkpoint**: the Rust 1.79.0 baseline and the final Rust
1.80.0 default sample built and ran on `qemu_x86` / Zephyr 3.7.0. Both
reached `Next call will crash if userspace is working.`, the expected
user-thread access violation and CPU exception, and run status 1.
Stopped here for review at the user's request: the full build matrix,
repository tests, full strict clippy, and per-version lint runs remain
pending. No full runs were started. After removing the generic TLS patch,
a fresh sysroot build reproduced the clippy warning; the local fix then
passed targeted `zephyr-futures` clippy with `--locked --lib -D warnings`
and the default sample build and run.

### Conflicts and adaptations

1. **`rust: remove submodules not required to build zephyr-rust`**:
   upstream updated the intentionally deleted documentation, LLVM, and
   Cargo submodule pointers. Kept the deletions and the merged
   `.gitmodules`. Rust 1.80 also introduced `src/tools/rustc-perf`, which
   is not needed to build this port; removed it and its `.gitmodules`
   entry in a fixup, then autosquashed into the original removal commit.
   No other rebase conflicts occurred.
2. **OS-TLS const initialization**: library clippy reported
   `initializer for thread_local value can be made const` for
   `zephyr-futures` even though its initializer already uses `const {}`.
   Rust 1.80's OS-TLS macro routes const initializers through a non-const
   function, which Clippy inspects without recognizing the caller's const
   syntax. Unlike native TLS, this backend still lazily allocates a boxed
   per-thread value; const syntax only checks the initializer at compile
   time, not optimizes its storage or runtime initialization. An initial
   generic macro patch retained a const initializer function (after fixing
   a helper-arm recursion error). Review rejected changing generic upstream
   Rust for a local false positive, so that commit was removed entirely.
   `zephyr-futures` now uses `RefCell::default()`, giving the same empty
   `RefCell<Option<Reactor>>` through the ordinary lazy initializer path.
   Default is not const in Rust 1.80, so Clippy correctly leaves it alone.
   TLS isolation, allocation, and destruction behavior are unchanged. No
   lint allows, alternative TLS backend, or unstable API were needed.
3. **Rust 1.80 cfg checking**: registered all Zephyr version cfg names in
   CMake's exported RUSTFLAGS, including disabled version thresholds.
   Registered zephyr-core's Kconfig cfg names in its build script. These
   declarations enable checking without enabling any additional cfgs.

### Dependencies and process notes

- Updated `library/backtrace` and `library/stdarch` worktrees to the
  upstream 1.80.0 pointers (`72265bea2108`, `df3618d9f351`).
- The first new-version build needed to update the sysroot lockfile and
  failed on the read-only repository mount; reran with `WRITABLE=1`.
  Rust 1.80 raises the minimum `rustc-demangle` version to `0.1.24`.
  Cargo selected `0.1.28`, which drops its compiler_builtins dependency
  and failed with `E0463` in this custom sysroot. Resolved explicitly to
  upstream Rust 1.80.0's locked `0.1.24`, retaining compiler_builtins.
  The update command required `RUSTC_BOOTSTRAP=1` for std's unstable
  `public-dependency` manifest feature.
- Pulled all images from `ghcr.io/tylerwhall/zephyr-rust`: the
  `3.7.0-1.79.0` baseline and `2.3.0-1.80.0`, `2.7.3-1.80.0`,
  `3.7.0-1.80.0` target images. Forced the ghcr prefix on every container
  invocation; no images were built locally.
- The host crates passed strict library-pass clippy. Subsequent library
  passes reported success while their west build logs contained the
  temporary TLS macro recursion error: `ci/clippy.sh` can reuse a stale
  `zephyr.elf` after a failed rebuild. Those successes are not considered
  final validation. The final smoke command independently required the
  west build to succeed before running QEMU. Use fresh build directories
  for the pending full lint pass.
- Updated active version pins, workflow image tags, README, and current
  command examples in AGENTS.md and the build-matrix TODO. Historical
  examples in completed TODO tasks remain unchanged.
- Validation logs are retained locally under `.upgrade-logs/` and are not
  committed. Unlike the end-to-end process, broader validation and push
  are deliberately deferred until review.

## 1.78.0 → 1.79.0 (2026-09-24)

**Result**: 15 port commits rebased from `1.78.0` onto `1.79.0` plus one
1.79-adaptation commit, branch `zephyr-1.79.0`, final tip `2816eb3f5dc`.
`rust/libc` was unchanged: std
1.79.0 still requires `0.2.153` and the existing `0.2.153-6` port was kept.
The default sample built and ran on `qemu_x86` / Zephyr 3.7.0 in the pulled
`ghcr.io/tylerwhall/zephyr-rust:zephyr-rust-3.7.0-1.79.0` image, reaching the
intentional userspace page fault. The full 113-job build matrix
(`ci/build-all.sh`, including the six executed `RUN=1` samples on all three
Zephyr versions), strict clippy (`-D warnings`), and the Zephyr 2.3.0
sanitycheck all passed. No sysroot `Cargo.lock` changes were needed.

### Conflicts and compile fixes

1. **`rust: remove submodules not required to build zephyr-rust`**: upstream
   moved the deleted submodule pointers again and also rewrote `.gitmodules`
   (limiting the `url = .` remote-submodule overlay to the `gcc-go` entry and
   reordering). Resolved upstream's `.gitmodules` plus our deletions of the
   documentation, `src/llvm-project`, and `src/tools/cargo` submodules.
2. **`zephyr: panicking: remove get/set hook rwlock`**: upstream applied the
   same removal (`RwLock<Hook>` → `StaticRwLock`/`static mut`), producing
   identical additions on both sides; the only work was collapsing diff3
   markers that duplicated `use crate::sys_common::thread_info;` and carrying
   the `#[cfg(not(target_os = "zephyr"))]` gate on `use crate::thread;`
   through. No semantic deviation from either side.
3. **`zephyr: ThreadId: don't use uninitialized mutex`**: between 1.78 and
   1.79 upstream only changed the import style of its 64-bit CAS loop
   (`Ordering::Relaxed` fully qualified instead of a shorthand import), which
   made the whole `new()` body conflict. Resolved the same way as in 1.78:
   the port's wholesale 32-bit `AtomicU32` counter replaces the upstream
   body, `ThreadId(NonZeroU32)` is kept, and `as_u64` casts `self.0.into()`
   to the upstream `NonZeroU64` return type (the upstream generic `NonZero`
   import is retained).
4. **One compile error** in `panicking.rs` (`default_hook`), fixed inside the
   port series as `panicking: default_hook: name threads via
   thread::try_current`: Rust 1.79 deleted `sys_common::thread_info` and
   moved current-thread tracking into `crate::thread` (a `thread_local!`
   OnceCell seeded by `rt::init`). The first fix attempt restored
   `sys_common::thread_info` as a zephyr-port-local module and cfg-gated the
   lookup; after review it was simplified — `thread::try_current()` works
   directly for the zephyr port (the TLS port implements
   `thread_local_key`), and on the threadless zephyr build it degrades to
   `None`, yielding `<unnamed>` exactly as the old `thread_info` path did.
   The final port delta is just the deletion of the removed-module import;
   no `sys_common::thread_info` restore is needed.

### Decisions and gotchas

- **`library/backtrace` and `library/stdarch` submodule pointers moved** to
  the upstream 1.79.0 revisions (`e15130618237`, `c0257c1660e`). The
  rebase's recorded submodule worktrees were stale; `git submodule update
  --init` re-synced them. No zephyr port changes were required in either.
- The first 1.79 build failed with `E0432` (`thread_info`) and `E0433`
  (`thread`) in `panicking.rs`'s `default_hook`; see fix 4 above. No other
  port code needed changes; the entire sysroot then compiled unmodified.
- The ghcr.io images for all three Zephyr versions with the 1.79 tag existed
  upstream and were pulled (`zephyr-rust-3.7.0-1.79.0`, `-2.7.3-1.79.0`,
  `-2.3.0-1.79.0`), so no local container build was needed.
- Hardcoded `RUST_VERSION=1.78.0` examples in `AGENTS.md` and
  `docs/BUILD_MATRIX_TODO.md` were updated to 1.79.0 so the commands stay
  valid for this single-version-per-revision tree.

## 1.77.0 → 1.78.0 (2026-09-23)

**Result**: 15 port commits rebased from `1.77.0` onto `1.78.0`, branch
`zephyr-1.78.0`, final tip `788de1f8937`. The libc port was also rebased from
`0.2.150` to `0.2.153`, branch `zephyr-0.2.153`, tip
`9c4f7e0888a8fbb1e0bfaa48b1bb566f1ebcaa99`. The default sample built and ran
on `qemu_x86` / Zephyr 3.7.0, reaching the intentional userspace page fault.

### Conflicts and compile fixes

1. **`rust: remove submodules not required to build zephyr-rust`**: upstream
   moved the deleted submodule pointers again. Kept the intentional port
   deletions and retained only the required submodules.
2. **`build.rs: mark std for zephyr as stable`**: retained Rust 1.78's
   target-architecture-based platform checks and added `target_os == "zephyr"`.
3. Rust 1.78's LLVM target layout requires `i128:128` for the i686 Zephyr
   target; added it to `rust/targets/i686-unknown-zephyr.json`.
4. Rust 1.78 moved or removed several PAL support files. Updated the Zephyr
   PAL paths for `cmath`, condvar, and rwlock, and removed stale memchr, once,
   and path module declarations. These changes were autosquashed into the
   corresponding port commits.
5. The `ThreadId` port fix was adapted to retain the upstream generic `NonZero`
   import used by `available_parallelism` while using Zephyr's 32-bit atomic
   counter.

### Decisions and gotchas

- **`rust/libc` rebased**: Rust 1.78.0 requires libc `0.2.153`; the existing
  six-commit Zephyr port was rebased onto that tag without conflicts.
- **`rust/sysroot-stage1/Cargo.lock`**: updated the libc entry from `0.2.150`
  to `0.2.153` after the required writable-container build.
- The initial read-only new-version build failed only when Cargo attempted to
  regenerate the lockfile; rerunning with `WRITABLE=1` produced the real
  lockfile change.
- The ghcr.io images pulled successfully from
  `ghcr.io/tylerwhall/zephyr-rust` for both the 1.77.0 baseline and 1.78.0.

## 1.76.0 → 1.77.0 (2026-09-23)

**Result**: 15 port commits rebased from `1.76.0` onto `1.77.0`, branch
`zephyr-1.77.0`, tip `430089a9428`. The default sample built and ran on
`qemu_x86` / Zephyr 3.7.0, reaching the intentional userspace page fault. Port
fixups were committed separately with `--fixup` and autosquashed after the
build succeeded.

### Conflicts

1. **`rust: remove submodules not required to build zephyr-rust`**: upstream
   moved the deleted submodule pointers again. Resolved by keeping all of our
   deletions, including the documentation submodules, `src/llvm-project`, and
   `src/tools/cargo`.
2. **`zephyr: stub sys impl`**: Rust 1.77 moved the platform abstraction
   implementation from `library/std/src/sys` into `library/std/src/sys/pal`.
   Kept Rust's new `sys/mod.rs` structure, added the Zephyr branch to
   `sys/pal/mod.rs`, and moved the Zephyr PAL files to `sys/pal/zephyr`.
3. **`zephyr: implement thread parking for Rust 1.71`**: the new PAL layout
   caused a file-location conflict. Kept the implementation under
   `sys/pal/zephyr`.

### Decisions

- **`rust/libc` unchanged**: std 1.77.0 still requires libc `0.2.150`; the
  existing port remains based on `0.2.150` (`0.2.150-6`).
- **`rust/sysroot-stage1/Cargo.lock`**: updated `compiler_builtins` from
  `0.1.103` to `0.1.105`. A normal lock update selected `0.1.160`, which
  requires Cargo's unstable `edition2024` feature and cannot be parsed by
  Cargo 1.77. The lock was therefore explicitly resolved to `0.1.105`.
- **PAL path fixes**: the Zephyr implementation's `cmath` and `os_str`
  includes were updated for Rust 1.77's `sys` layout (`../../cmath/mod.rs`
  and `../../os_str/mod.rs`). These were fixups for `zephyr: stub sys impl`
  and autosquashed into it. A temporary attempt to change the existing
  `super::zephyr::k_str_out_raw` call to `super::k_str_out_raw` caused a
  compile error; the original call was restored in another fixup.

### Gotchas hit

- The first read-only build failed to write `rust/sysroot-stage1/Cargo.lock`;
  rerunning with `WRITABLE=1` allowed the real lockfile change to be made.
- Full build output was redirected to a temporary log during the run and only
  the relevant tail was inspected.

## 1.75.0 → 1.76.0 (2026-09-17)

**Result**: 15 port commits rebased from `1.75.0` onto `1.76.0`, branch
`zephyr-1.76.0`, tip `45f4023249b`. No port compile errors; the build
passed on the first try (after the lockfile fix below). Smoke test on
`qemu_x86` / Zephyr 3.7.0 passed.

### Conflicts

1. **`rust: remove submodules not required to build zephyr-rust`**
   (first commit of the series): modify/delete conflicts on
   `src/doc/{book,edition-guide,embedded-book,nomicon,reference,rust-by-example,rustc-dev-guide}`,
   `src/llvm-project`, `src/tools/cargo` — upstream moved these submodule
   pointers between 1.75.0 and 1.76.0. Resolved by keeping our deletions
   (`git rm` each path). Upstream added no new submodules we also needed
   to drop.

2. **`zephyr: stub sys impl`**, two files:
   - `library/std/src/sys/mod.rs`: upstream inserted a `teeos` target
     branch in the `cfg_if` chain between `sgx` and the `unsupported`
     fallback. Kept `teeos`; kept our `zephyr` branch immediately before
     `unsupported`.
   - `library/std/src/sys_common/mod.rs`: upstream *inverted* the
     net-module condition to enumerate platforms with their own `net`
     (`all(unix, not(l4re)), windows, hermit, solid_asp3`). Zephyr targets
     are `target_family = "unix"`, so they now match the first arm and
     would require a non-existent `sys::zephyr::net`. Excluded zephyr
     explicitly: `all(unix, not(target_os = "l4re"), not(target_os =
     "zephyr"))`.

### Decisions

- **`rust/libc` unchanged**: std 1.76.0 requires libc `0.2.150`; the
  existing port was already based on `0.2.150` (`0.2.150-6`).
- **`rust/sysroot-stage1/Cargo.lock`**: real change, committed with the
  port — std 1.76.0 added `unwinding 0.2.10` and `gimli 0.34.0`.
- **Workflow files**: `.github/workflows/{container-build,main}.yml` still
  pinned `1.75.0` after the port commit (3 references: the
  `container-build.yml` default input and two container image tags in
  `main.yml`). Missed on the first pass; fixed in a follow-up and squashed
  into the port commit. The file list in `rust-upgrade.md` now includes
  the workflows.

### Gotchas hit

- **`env.sh` host rustup probe**: with `rust-toolchain.toml` bumped to
  1.76.0, the host `rustc --version` in `env.sh` triggered a rustup
  auto-install, which failed (`Permission denied` writing
  `~/.rustup/tmp` in the sandbox). `RUST_VERSION` resolved empty and the
  container build failed with the cryptic "a value is required for
  '--default-toolchain'". Worked around by passing
  `RUST_VERSION=1.76.0` explicitly to `container-build.sh` /
  `build-cmd.sh`.
- **Read-only repo vs. Cargo.lock**: first build in the new image failed
  with `error: failed to write .../rust/sysroot-stage1/Cargo.lock`.
  Rerun with `WRITABLE=1`.
