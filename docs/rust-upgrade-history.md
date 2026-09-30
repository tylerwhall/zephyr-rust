# Rust upgrade history

Running log of zephyr-rust Rust version upgrades: every important decision
and conflict, per `docs/rust-upgrade.md`. Newest first.

## 1.81.0 → 1.82.0 (2026-09-30)

**Result**: 16 port commits rebased from `1.81.0` onto `1.82.0`, branch
`zephyr-1.82.0`, final tip `1d3975f8558a88be15e30a787f920f6537b37ace`.
The old `zephyr-1.81.0` branch was not rewritten. `rust/libc` remains at
`9c4f7e0888a8fbb1e0bfaa48b1bb566f1ebcaa99`: Rust 1.82.0 still requires
`0.2.153`, matching the existing six-commit port.

**Validation checkpoint**: the Rust 1.81.0 baseline and upgraded Rust
1.82.0 default sample built and ran on `qemu_x86` / Zephyr 3.7.0. Both
reached `Next call will crash if userspace is working.`, the expected
user-thread access violation and CPU exception, and run status 1. The
upgraded sample also passed a second, fresh build with the repository
mounted read-only. Smoke output includes TLS isolation and mutex
contention checks. Every run used the process-group-safe
`ci/run-sample.sh` runner. Stopped before the full matrix for user review,
as requested; broader builds, repository tests, and Clippy remain pending.
Nothing was pushed.

### Conflicts and adaptations

1. **`rust: remove submodules not required to build zephyr-rust`**:
   upstream updated all ten intentionally deleted submodule pointers
   (documentation, LLVM, Cargo, and rustc-perf). Kept the deletions.
   Resolved `.gitmodules`' LLVM branch conflict by removing the entry,
   retaining only stdarch and backtrace. No new submodules needed removal.
2. **`zephyr: stub sys impl`** and **public dependencies**: upstream
   reformatted the hashbrown/std_detect declarations. Retained that
   formatting and added the Zephyr dependencies; the later port commit
   still marks both as public.
3. **`zephyr: panicking: remove get/set hook rwlock`**: upstream reordered
   imports and grouped fmt/intrinsics/process/thread. Kept the new import
   layout and cfg-gated the relocated PoisonError/RwLock import. Zephyr
   still bypasses the hook lock and calls the default hook directly;
   custom panic hooks remain unsupported.
4. **`zephyr: ThreadId: don't use uninitialized mutex`**: retained
   upstream's new ManuallyDrop import and grouped panic/panicking imports,
   alongside the port's NonZeroU32/NonZeroU64 and atomic imports. The
   32-bit counter and numeric conversion are unchanged.
5. **Review**: range-diff confirms all 16 commits are retained; changes
   to the port patches are limited to the above conflict resolutions.
   No port compile errors, fixup commits, autosquash, or new lint allows
   were needed. The delta from `1.82.0` contains only port changes and
   intentional submodule deletions, with no conflict markers.

### Dependencies and process notes

- Updated the nested submodule worktrees to Rust 1.82.0's upstream
  pointers: backtrace `230570f2dac8`, stdarch `d9466edb4c53`. Recursive
  submodule update completed successfully; no nested port changes.
- The first new-version build failed to update the sysroot lockfile on
  the read-only mount. Reran with `WRITABLE=1`. Rust 1.82 raises the
  minimum compiler_builtins version from `0.1.105` to `0.1.123`; Cargo
  selected `0.1.160`, which requires edition-2024 manifest support absent
  in Cargo 1.82. Resolved explicitly to `0.1.123` with `cargo update
  --precise` and `RUSTC_BOOTSTRAP=1` for std's public-dependency feature.
  The lockfile also adds upstream's local windows-targets package.
  Existing rustc-demangle and other resolutions remain unchanged.
- Pulled the baseline `3.7.0-1.81.0` and all target images
  (`2.3.0-1.82.0`, `2.7.3-1.82.0`, `3.7.0-1.82.0`) from
  `ghcr.io/tylerwhall/zephyr-rust`. Forced the ghcr prefix on every
  container invocation. No local images were built; the user required
  stopping if any image could not be pulled.
- Updated active pins, workflow tags/default, README, AGENTS.md, and
  pending build-matrix command examples. Historical records are unchanged.
- Builds still emit sysroot/Zephyr warnings (including libc cfg checks,
  unused std PAL imports/functions, and the unsupported dylib crate type).
  This checkpoint is smoke validation, not strict lint validation.
- Logs and separate timestamped Docker build volumes remain local under
  `.upgrade-logs/`, excluded from commits. The final clean build/run logs
  are named `final-1.82-build-<timestamp>.log` and
  `final-1.82-run-<timestamp>.log`. Full validation and push are deferred
  until review.

## 1.80.0 → 1.81.0 (2026-09-30)

**Result**: 16 port commits rebased from `1.80.0` onto `1.81.0`, branch
`zephyr-1.81.0`, final tip `67a812dc932f1610add9ce819d0eea0dad81ad3e`.
The old `zephyr-1.80.0` branch was not rewritten. `rust/libc` remains at
`9c4f7e0888a8fbb1e0bfaa48b1bb566f1ebcaa99`: Rust 1.81.0 still requires
`0.2.153`, matching the existing six-commit port.

**Validation checkpoint**: the Rust 1.80.0 baseline and upgraded Rust
1.81.0 default sample built and ran on `qemu_x86` / Zephyr 3.7.0. Both
reached `Next call will crash if userspace is working.`, the expected
user-thread access violation and CPU exception, and run status 1. The new
sample also passed a fresh, read-only build and run after autosquashing.
The process-group-safe `ci/run-sample.sh` runner was used for every run.
Initially stopped before broader validation for the requested review.
After the user authorized continuation, strict Clippy passed for host
crates, libraries, and all eight apps/tests on Zephyr 2.3.0, 2.7.3, and
3.7.0 / `qemu_x86`. The full 113-job matrix passed with `RUN=1`, including
all six verified sample runs (expected crash marker and exit status 1).
All six logs contain CPU exceptions; the Zephyr 2.3.0 logs drop some fault
messages and omit the literal access-violation line printed on 2.7.3 and
3.7.0. Zephyr 2.3.0 sanitycheck executed and passed
all seven configurations on `qemu_x86` and `qemu_cortex_m3` (zero failures,
skips, or warnings). Later-version test execution remains outside the
current runner's scope. Nothing was pushed.

### Conflicts and adaptations

1. **`rust: remove submodules not required to build zephyr-rust`**:
   upstream updated the intentionally deleted book, edition-guide,
   embedded-book, reference, rust-by-example, rustc-dev-guide, Cargo, and
   rustc-perf submodule pointers. Kept these deletions and the merged
   `.gitmodules`; no additional submodules needed removal.
2. **`zephyr: panicking: remove get/set hook rwlock`**: Rust 1.81 renamed
   the hook-facing `PanicInfo` to `PanicHookInfo`, changed construction to
   `PanicHookInfo::new`, and moved backtrace support from `sys_common` to
   `sys`. Retained upstream's imports, signatures, lazy payload handling,
   and non-Zephyr `RwLock<Hook>` implementation. Zephyr still bypasses the
   hook lock and calls the default hook directly; custom hook APIs remain
   unsupported. Removed the obsolete allocation comment/return block
   rather than restoring upstream's deleted structure. Cfg-gated unused
   hook machinery and documented the Zephyr stubs instead of carrying
   the old `missing_docs` and `dead_code` allows. No new lint allows.
3. **OS-TLS compile error (`E0432`)**: Rust 1.81 moved OS-TLS keys from
   the PAL / `sys_common` interface into `sys::thread_local::key`. Moved
   the Zephyr implementation to `sys/thread_local/key/zephyr.rs`, removed
   its old PAL registration, and registered it alongside upstream's other
   key backends. Reused upstream's racy `LazyKey`, retaining its sentinel
   handling and the existing Zephyr 32-slot, thread-custom-data-backed
   storage with no destructor support. Made key creation safe to match
   the new interface, made the atomic counter immutable, and added
   explicit unsafe blocks for the new module's unsafe-operation policy.
   Committed this adaptation as a fixup of the original TLS port commit;
   the next build and run passed.
4. **Autosquash**: moving the TLS fixup earlier conflicted with the later
   thread-parking registration in `sys/pal/zephyr/mod.rs`. Removed only
   `thread_local_key` in the TLS commit and retained `thread_parking` in
   its own commit. Verified the final tree hash exactly matched the
   validated pre-autosquash tree, then rebuilt from scratch.
5. **Clippy `missing_const_for_thread_local`**: strict library Clippy
   passed, but the app pass flagged `samples/rust-app`'s already-const
   `RefCell::new(1)` initializer through the OS-TLS macro expansion.
   Like the Rust 1.80 `zephyr-futures` case, const syntax does not change
   OS-TLS's lazy allocation. Used the equivalent non-const
   `RefCell::from(1)` initializer locally, without changing upstream Rust
   or adding lint allows. Committed as `8032795`; targeted strict
   Clippy, the smoke run's TLS isolation assertions, and full strict
   Clippy on all three Zephyr versions passed.

### Dependencies and process notes

- `library/backtrace` and `library/stdarch` stay at the same upstream
  pointers as Rust 1.80.0 (`72265bea2108`, `df3618d9f351`). Recursive
  submodule update completed successfully; no nested port changes.
- The first new-version build failed to update the sysroot lockfile on
  the read-only mount. Reran with `WRITABLE=1`; the only lockfile change
  is `hermit-abi 0.3.9` → `0.4.0`, matching std's new requirement. Existing
  compiler_builtins and rustc-demangle resolutions remain compatible;
  no explicit dependency downgrades were needed.
- Pulled the baseline `3.7.0-1.80.0` and all target images
  (`2.3.0-1.81.0`, `2.7.3-1.81.0`, `3.7.0-1.81.0`) from
  `ghcr.io/tylerwhall/zephyr-rust`. Forced the ghcr prefix on every
  container invocation; no images were built locally.
- Updated active pins, workflow tags, README, AGENTS.md, and pending
  build-matrix command examples. Historical records and comments about
  Rust 1.80 introducing cfg checking remain unchanged.
- Builds still emit existing sysroot/Zephyr warnings; Rust 1.81 also
  reports unregistered upstream `bootstrap` cfgs in `panic_abort` and
  `unwind`. Strict Clippy used `-D warnings`, `--locked`, and default
  strict build handling. Sysroot-layer crates still receive only rustc
  lint coverage, not true Clippy coverage; see CLIPPY_SYSROOT_DEBT.md.
- Clippy used separate fresh, version-keyed build directories, two app
  workers, and a host/library pass before each full pass. Verified the
  common-pass west build logs independently to rule out stale ELF reuse.
  Archived old matrix results and started fresh so `--resume` could not
  skip an older Rust version's jobs.
- Create `.upgrade-logs/` with `mkdir -p .upgrade-logs`; redirect each
  pull/build/run's stdout and stderr to a named log there (`> ... 2>&1`).
  Check its exit status and inspect only a short tail or targeted errors.
  Use separate timestamped build directories there as Docker volumes.
  Keep logs/builds local and out of commits. Broader validation logs are
  also there, with matrix results linked from `ci/log/build`. Archiving
  the existing `ci/sanity-out` hit a host permission denial; the nono
  diagnostic failed to reload its sandbox state. Sanitycheck still
  performed a clean run successfully in its existing output directory,
  where those artifacts remain. Push is deferred to the user.

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
