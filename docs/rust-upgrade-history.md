# Rust upgrade history

Running log of zephyr-rust Rust version upgrades: every important decision
and conflict, per `docs/rust-upgrade.md`. Newest first.

## 1.84.0 → 1.85.0 (2026-10-06)

**Result**: 16 port commits rebased from `1.84.0` onto `1.85.0`, branch
`zephyr-1.85.0`, rebased tip `f9102398680c918ea3ff4aff9872bea15f07edf2`.
The later library-preparation commits advance this branch as noted below.
Rust 1.85.0 requires libc `0.2.169`; rebased the six-commit libc port
onto that tag, branch `zephyr-0.2.169`, final tip
`3f4a9a93606ff0ae0880e2ee583a89165a9c4cf1`. Neither old port branch
was rewritten.

**Validation checkpoint**: the Rust 1.84.0 baseline and upgraded Rust
1.85.0 default sample built and ran on `qemu_x86` / Zephyr 3.7.0. Both
reached `Next call will crash if userspace is working.`, the expected
user-thread access violation and CPU exception, and run status 1. TLS
isolation and mutex contention checks passed. After autosquashing, a
second fresh build with the repository mounted read-only also built and
ran successfully. Every run used the process-group-safe
`ci/run-sample.sh` runner. A separate stable Rust compile check confirmed
const `HashMap::with_hasher` and `HashSet::with_hasher` initialization
without feature gates. Shell syntax and wrapper argument-routing checks
also passed.

Initially stopped before the full matrix for user review, as requested.
No broader validation was claimed at that checkpoint. After review the
user requested library preparation for build-std, keeping the manual
sysroot build functional, then the full matrix. That validation passed:

- Full 113-job matrix with RUN=1 on Zephyr 2.3.0/2.7.3/3.7.0, including
  six expected-crash sample runs and all 107 build-only combinations.
- Zephyr 2.3.0 sanitycheck: all seven configurations executed and passed
  on qemu_x86 and qemu_cortex_m3, with no failures or skips.
- Strict Clippy on Zephyr 3.7.0: host crates, libraries, and all eight
  app/test crates passed with --locked and -D warnings. The sysroot
  lint-root limitation documented in CLIPPY_SYSROOT_DEBT.md remains.
- Every preparation commit passed the default smoke test. Shared bindings
  and shared mutex bookkeeping additionally passed smoke runs on all
  three Zephyr versions. The final autosquash preserved the validated
  tree hash, followed by a fresh successful smoke build/run.

The full matrix exposed two Rust 1.85 target-validation requirements:
ARM targets must specify llvm-floatabi (soft/hard, matching their upstream
none/eabi counterparts), and the soft-float Cortex-R5 feature string had
an invalid leading comma. Fixed the seven ARM target JSONs and the feature
join in update-targets.sh; folded these fixes into the upgrade commit.
All twelve target JSONs pass rustc's parser. The affected/incomplete matrix
results were reset before resuming, retaining only completed, unaffected
jobs; all rust-app combinations were rerun after moving the nine-slot pool
test setting to the qemu_x86 board config.

Library separation is detailed in BUILD_STD_INVESTIGATION.md. Two appended
std-port commits expose primitive Instant ticks and make core/sys private
backend dependencies; the branch now ends at
`61e773a1c7e` (the earlier rebased port history was not rewritten).
The production build still uses rust/build.sh and its compiler wrapper.
No production build-std switch was made, and nothing was pushed.

### Conflicts and adaptations

1. **`rust: remove submodules not required to build zephyr-rust`**:
   upstream updated seven intentionally deleted pointers (five
   documentation trees, LLVM, and Cargo). Kept their deletions and
   resolved `.gitmodules` to retain only stdarch and backtrace.
   Upstream itself removed the rustc-dev-guide submodule, so there is
   no longer a corresponding port deletion. No new submodules needed
   removal.
2. **`zephyr: stub sys impl` / `c_char`**: upstream replaced the OS
   whitelist with architecture-based signedness selection. Retained
   upstream's selection and documentation for other targets, but
   preserved Zephyr's existing unsigned aarch64/riscv64 and signed
   other-architecture types, matching the libc port. Otherwise the
   rebase would silently change Zephyr arm/riscv32 types. Applied the
   signedness preservation as a fixup of the original stub commit.
3. **`zephyr: panicking: remove get/set hook rwlock`**: retained
   upstream's new `#[derive(Default)]` and `#[default]` enum variant,
   dropping the obsolete manual `Default` implementation. Cfg-gated
   the enum and hook static as before; Zephyr still calls the default
   hook directly and does not support custom hooks.
4. **Libc registration**: retained upstream's `crate::`-qualified
   re-exports and new `prelude!()` call for Xous. Registered Zephyr
   with the same prelude pattern rather than restoring the old layout.
   Range-diff confirms the other five libc patches are unchanged.
5. **Review and autosquash**: autosquashed the c_char fixup after the
   build and smoke run passed, without conflicts. Verified that the
   final tree hash exactly matched the validated pre-autosquash tree.
   Range-diff retains all 16 Rust commits; patch changes are limited
   to the submodule, c_char, and panic-hook adaptations above. Deltas
   from the release tags contain only the ports and intentional
   submodule deletions, with no conflict markers or new lint allows.

### Dependencies and build integration

- Updated nested worktrees to Rust 1.85.0's upstream pointers:
  backtrace `4d7906bb24ae`, stdarch `684de0d6fef7`. Recursive submodule
  update completed successfully; no nested port changes.
- The first new-version build failed to update the sysroot lockfile
  on the read-only mount. Used `WRITABLE=1` only for Cargo updates,
  with `RUSTC_BOOTSTRAP=1` for std's public-dependency manifest feature.
  Updated compiler_builtins to Rust 1.85's exact `0.1.140` requirement
  and libc to `0.2.169`; other existing resolutions were retained
  except hashbrown as described next.
- Rust 1.85 stabilizes const collection constructors. The old
  hashbrown `0.15.0` caused errors that `with_hasher` cannot be
  indirectly exposed to stable. Updated to upstream's locked `0.15.2`,
  which has the required `#[rustc_const_stable_indirect]` annotations,
  but the error persisted because this custom Cargo build lacked
  Rust bootstrap's `-Zforce-unstable-if-unmarked` stability metadata.
- Added `rust/sysroot-rustc.sh`, invoked only during std's Cargo build,
  to supply that flag to hashbrown and its std consumer. Applying it
  to the entire sysroot fixed std but incorrectly marked the public
  Zephyr crates as `rustc_private`; scoping it to hashbrown alone
  required that feature in std. The final wrapper covers std and
  hashbrown only, preserving stable application access to Zephyr APIs
  without changing upstream std or weakening const-stability checks.
  The build integration is committed separately from the version bump.
- Changing global sysroot flags during investigation also left duplicate
  core/compiler_builtins artifacts in the experimental build directory.
  Discarded that directory from validation and used fresh directories
  for both successful upgraded builds. No stale ELF was used.

### Process notes

- Pulled the baseline `3.7.0-1.84.0` and all target images
  (`2.3.0-1.85.0`, `2.7.3-1.85.0`, `3.7.0-1.85.0`) from
  `ghcr.io/tylerwhall/zephyr-rust`. The initial combined pull command
  hit its tool deadline during the last download; explicitly pulling
  that image again completed successfully. No image pull failed and
  no local images were built. Forced the ghcr prefix and explicit
  Rust/Zephyr versions on every container invocation.
- Updated active pins, workflow tags/default, README, AGENTS.md, and
  pending build-matrix examples. Historical references are unchanged.
- Existing sysroot/Zephyr warnings remain, including unused PAL items
  and unsupported dylib crate type. Broader validation subsequently passed
  as described above; 2.7.3/3.7.0 test execution remains separate twister work.
- Logs and timestamped build volumes remain local under `.upgrade-logs/`,
  excluded from commits. Final clean build/run logs are
  `final-1.85-build.log` and `final-1.85-run.log`; the final volume's
  timestamp is recorded in `1.85-final-stamp`. Push is deferred to the
  user, in submodule-before-parent order. Subsequent preparation validation
  logs are prep-matrix.log, prep-sanity.log, and prep-clippy.log, with matrix
  results under ci/log/build/run-1. Per-commit smoke logs are in prep-*/.

## 1.83.0 → 1.84.0 (2026-09-30)

**Result**: 16 port commits rebased from `1.83.0` onto `1.84.0`, branch
`zephyr-1.84.0`, final tip `82c6a017a4580809c65f1ddedb360d7397b7baa7`.
The old `zephyr-1.83.0` branch was not rewritten. Rust 1.84.0 requires
libc `0.2.162`; rebased the six-commit port onto that tag without
conflicts, branch `zephyr-0.2.162`, tip
`62ee882c8cb07d640651ffd8940d2e2afaa25606`. The old libc branch was
not rewritten; range-diff confirms all six patches are unchanged.

**Validation checkpoint**: the Rust 1.83.0 baseline and upgraded Rust
1.84.0 default sample built and ran on `qemu_x86` / Zephyr 3.7.0. Both
reached `Next call will crash if userspace is working.`, the expected
user-thread access violation and CPU exception, and run status 1. TLS
isolation and mutex contention checks passed. After autosquashing, a
second, fresh build with the repository mounted read-only also built and
ran successfully. Every run used the process-group-safe
`ci/run-sample.sh` runner. Stopped there for user review, continuing
the constraints of the preceding upgrade.

After user review, the remaining CI-parity stages all passed:
- Full 113-job matrix (`ci/build-all.sh`, `RUN=1`) across Zephyr
  2.3.0/2.7.3/3.7.0: all 107 build-only jobs green; the six sample
  runs (rust-app and no_std on qemu_x86, one per Zephyr version)
  each reached `Next call will crash if userspace is working.`, the
  expected user-mode page fault, and exited 1.
- Strict Clippy (`ci/clippy.sh`, `-D warnings`, `--locked`, Zephyr
  3.7.0): host crates, library crates, and all eight apps/tests
  clean.
- Zephyr 2.3.0 sanitycheck: 7 of 7 test configurations passed on
  qemu_x86 and qemu_cortex_m3.

Nothing was pushed.

### Conflicts and adaptations

1. **`rust: remove submodules not required to build zephyr-rust`**:
   upstream updated eight intentionally deleted submodule pointers
   (six documentation trees, LLVM, and Cargo). Kept the deletions and
   resolved `.gitmodules`' changed LLVM branch by removing its entry,
   retaining only stdarch and backtrace. No new submodules needed removal.
2. **Mutex const-stability compile error**: Rust 1.84 enables the check
   that `#[rustc_const_stable]` applies only to stable functions
   (`d066dfdb835`). The internal Zephyr `Mutex::new` had an obsolete
   annotation inherited from the original port. Removed it, following
   upstream's identical Xous-backend adaptation (`59944c9c9f9`), rather
   than adding a public stability declaration or suppressing the check.
   The constructor remains `const`; lazy allocation, atomic publication,
   and runtime locking behavior are unchanged. Committed as a fixup of
   `zephyr: implement mutex`; the next build and smoke run passed.
3. **Autosquash and review**: autosquashed the fixup after successful
   build/run validation, without conflicts. Git tracked the mutex-file
   relocation through the later sys::sync commit. Verified the final
   tree hash exactly matched the validated pre-autosquash tree. The
   final range-diff retains all 16 commits; changes to the port patches
   are limited to the submodule conflict, upstream context shifts, and
   removal of the obsolete annotation. The delta from `1.84.0` contains
   only port changes and intentional submodule deletions, with no
   conflict markers or new lint allows.
4. **RISC-V target ICE in the full matrix**: Rust 1.84 removed the
   empty-string default from the RISC-V `llvm-abiname` match
   (upstream `abb05c0fd50`), so `qemu_riscv32` builds ICE'd in
   `rustc_codegen_ssa`'s `create_object_file` ("unknown RISC-V ABI
   name") because the custom target specs never set the field. Set
   `"llvm-abiname": "ilp32"` in the three rv32 target JSONs and
   `"lp64"` in `riscv64imac`, matching their soft-float feature sets
   (`+m,+a[,+c]`) and the built-in none-elf targets. Parent-repo
   change, committed separately; the riscv32 job and then the full
   matrix passed.

### Dependencies and process notes

- Updated stdarch to Rust 1.84.0's upstream pointer `e5e00aab0a8c`;
  backtrace remains at `230570f2dac8`. Recursive submodule update
  completed successfully; no nested port changes.
- The first new-version build failed to update the sysroot lockfile on
  the read-only mount. Used `WRITABLE=1` only for the Cargo lock update,
  with `RUSTC_BOOTSTRAP=1` for std's public-dependency manifest feature.
  Updated compiler_builtins to `0.1.138`, matching Rust 1.84's exact
  requirement and upstream library lockfile. The lockfile also updates
  libc to `0.2.162` and removes memchr `2.5.0`, whose direct std
  dependency was removed upstream. Hashbrown remains at the compatible
  `0.15.0`; other resolutions and lockfile format 4 are unchanged.
- Pulled the baseline `3.7.0-1.83.0` and all target images
  (`2.3.0-1.84.0`, `2.7.3-1.84.0`, `3.7.0-1.84.0`) from
  `ghcr.io/tylerwhall/zephyr-rust`. Forced the ghcr prefix and explicit
  Rust/Zephyr versions on every container invocation. No local images
  were built; stopping on any pull failure remained a user requirement.
- Updated active pins, workflow tags/default, README, AGENTS.md, and
  pending build-matrix command examples. Historical records are unchanged.
- Builds still emit sysroot/Zephyr warnings, including unused std PAL
  imports/functions and the unsupported dylib crate type. Clippy and
  the full build matrix passed as described above; 2.7.3/3.7.0 test
  execution (twister) remains tracked in
  `docs/BUILD_MATRIX_TODO.md`.
- Matrix process notes: `ci/log/build` must be a real directory, not a
  symlink (GNU parallel's results-directory `mkpath` fails on an
  existing symlink), and `--resume` skips any job whose result files
  exist even if it was killed mid-run. The first failed matrix run's
  halted 3.7.0 sample-run jobs had to be deleted from the results
  tree before the resume re-ran them.
- Logs and separate timestamped Docker build volumes remain local under
  `.upgrade-logs/`, excluded from commits. Final clean build/run logs:
  `final-1.84-build-20260930-195500.log` and
  `final-1.84-run-20260930-195500.log`; full validation logs:
  `matrix-1.84-resume2.log`, `clippy-3.7.0-1.84.log`, and
  `sanitycheck-2.3.0-1.84.log`. Push is deferred to the user.

## 1.82.0 → 1.83.0 (2026-09-30)

**Result**: 16 port commits rebased from `1.82.0` onto `1.83.0`, branch
`zephyr-1.83.0`, final tip `0aa4f0475411d633776909438bb3fdaf2824c51c`.
The old `zephyr-1.82.0` branch was not rewritten. Rust 1.83.0 requires
libc `0.2.161`; rebased the existing six-commit port onto that tag without
conflicts, branch `zephyr-0.2.161`, tip
`79f074e38df3f46ffd241f58d20135f9b9f1f50c`. The old libc port was not
rewritten, and range-diff confirms all six patches are unchanged.

**Validation checkpoint**: the Rust 1.82.0 baseline and upgraded Rust
1.83.0 default sample built and ran on `qemu_x86` / Zephyr 3.7.0. Both
reached `Next call will crash if userspace is working.`, the expected
user-thread access violation and CPU exception, and run status 1. TLS
isolation and mutex contention checks passed. After autosquashing, a
second, fresh build with the repository mounted read-only also built and
ran successfully. Every run used the process-group-safe
`ci/run-sample.sh` runner. Stopped before the full matrix for user review,
as requested; no matrix, repository test suite, or Clippy pass was
started. Other boards and Zephyr versions remain unvalidated. Nothing
was pushed.

### Conflicts and adaptations

1. **`rust: remove submodules not required to build zephyr-rust`**:
   upstream updated eight intentionally deleted documentation/Cargo
   submodule pointers and added `src/gcc` and `src/tools/enzyme`. Kept
   the deletions, removed the two new compiler-only submodules, and
   resolved `.gitmodules` to retain only stdarch and backtrace.
2. **`zephyr: ThreadId: don't use uninitialized mutex`**: retained
   upstream's import ordering and the new `pub(crate)` visibility of
   `ThreadId::new`, while preserving the port's 32-bit atomic counter.
   Rust 1.83 introduces `ThreadId::from_u64` for persistent current-thread
   IDs stored through OS-TLS pointers. Adapted it with checked `u32`
   conversion and `NonZeroU32`, rejecting zero and out-of-range values
   rather than truncating them. Retained upstream's new current-thread
   implementation unchanged.
3. **Random backend compile errors (`E0432`, `E0425`)**: Rust 1.83 moves
   random generation and HashMap seeding out of PAL common into
   `sys::random`. Removed the obsolete `common::hashmap_random_keys`
   export, then registered Zephyr with upstream's unsupported backend
   in both the selection and default-HashMap-seeding cfgs. Each fix was
   committed separately as a fixup of `zephyr: stub sys impl`, with a
   build after each. Random generation remains unsupported (the new
   unstable API panics); HashMap seeds now use upstream's allocation
   address fallback instead of the former fixed `(1, 2)`. This is not
   cryptographic entropy; no Zephyr RNG API or Kconfig dependency was
   introduced.
4. **Autosquash and review**: autosquashed both fixups after the build
   and smoke run succeeded, without conflicts. Verified the resulting
   tree hash exactly matched the validated pre-autosquash tree. The
   final range-diff retains all 16 commits; semantic adaptations are
   limited to the removals, ThreadId, and random backend above. The
   delta from `1.83.0` contains only port changes and intentional
   submodule deletions, with no conflict markers or new lint allows.

### Dependencies and process notes

- Updated the stdarch worktree to Rust 1.83.0's upstream pointer
  `c881fe3231b3`; backtrace remains at `230570f2dac8`. Recursive submodule
  update completed successfully; no nested port changes.
- The first new-version build failed to update the sysroot lockfile on
  the read-only mount. Used `WRITABLE=1` only for Cargo lock updates,
  with `RUSTC_BOOTSTRAP=1` for std's public-dependency manifest feature.
  Resolved compiler_builtins explicitly to `0.1.133`, matching Rust
  1.83's library lockfile and minimum requirement, rather than selecting
  a newer release incompatible with this compiler/Cargo.
- Cargo initially selected hashbrown `0.15.5`, which failed with missing
  compiler_builtins, unknown `strict_provenance_lints`, and unavailable
  `rustc_const_stable_indirect` attributes. Resolved explicitly to
  upstream Rust 1.83's locked `0.15.0`; the next build passed that crate.
  The lockfile also updates libc to `0.2.161`, allocator-api2 to
  `0.2.21`, adds std's pinned memchr `2.5.0`, and uses Cargo's lockfile
  format 4. Existing rustc-demangle and other resolutions are retained.
- Pulled the baseline `3.7.0-1.82.0` and all target images
  (`2.3.0-1.83.0`, `2.7.3-1.83.0`, `3.7.0-1.83.0`) from
  `ghcr.io/tylerwhall/zephyr-rust`. Forced the ghcr prefix and explicit
  Rust/Zephyr versions on every container invocation. No local images
  were built; the user required stopping if any image could not be
  pulled.
- Updated active pins, workflow tags/default, README, AGENTS.md, and
  pending build-matrix command examples. Historical records are unchanged.
- Builds still emit sysroot/Zephyr warnings, including unused std PAL
  imports/functions and the unsupported dylib crate type. Clippy and
  broader compatibility coverage remain pending, not claimed as passed.
- Logs and separate timestamped Docker build volumes remain local under
  `.upgrade-logs/`, excluded from commits. Final clean build/run logs:
  `final-1.83-build-20260930-193739.log` and
  `final-1.83-run-20260930-193739.log`. Push is deferred to the user.

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
`ci/run-sample.sh` runner. Initially stopped before the full matrix for
user review, as requested. After authorization to continue, strict Clippy
passed for host crates, libraries, and all eight apps/tests on Zephyr
2.3.0, 2.7.3, and 3.7.0 / `qemu_x86`, with `--locked`, `-D warnings`,
and no skips. The full 113-job matrix passed with `RUN=1`, including all
six verified sample runs (expected crash marker and exit status 1).
All six run logs contain CPU exceptions; Zephyr 2.3.0 omits the literal
access-violation line printed on 2.7.3 and 3.7.0, as in the prior upgrade.
Zephyr 2.3.0 sanitycheck executed and passed all seven configurations on
`qemu_x86` and `qemu_cortex_m3`, with zero failures, skips, or warnings.
Later-version test execution remains outside the current runner's scope.
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
  Strict Clippy passed without code changes. Sysroot-layer crates still
  receive only rustc lint coverage, not true Clippy coverage; see
  `docs/CLIPPY_SYSROOT_DEBT.md`.
- Logs and separate timestamped Docker build volumes remain local under
  `.upgrade-logs/`, excluded from commits. The final clean build/run logs
  are named `final-1.82-build-<timestamp>.log` and
  `final-1.82-run-<timestamp>.log`.
- Broader validation used fresh, version-keyed Clippy build directories,
  two app workers, and a host/library pass before each full pass. Audited
  all app exit files and build logs to rule out stale ELF reuse. Archived
  the previous matrix result link and linked `ci/log/build` to a fresh
  1.82 directory, so `--resume` could not skip older-version jobs.
  Independently confirmed 113 unique completed build logs and six run
  logs. Validation artifacts use timestamp `20260930-190511`.
- Sanitycheck used the exact runner command and flags from
  `ci/sanitycheck.sh` through `ci/build-cmd.sh`, with a fresh volume at
  `.upgrade-logs/sanity-1.82-20260930-190511`. This avoids deleting or
  reusing the root-owned `ci/sanity-out` from the previous upgrade.
  It ran in the pulled Zephyr 2.3.0 image with
  `ZEPHYR_TOOLCHAIN_VARIANT=zephyr`. Push remains deferred to the user.

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
