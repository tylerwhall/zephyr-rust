# Moving the Zephyr port to Cargo build-std

Investigation against Rust/Cargo 1.85.0, using the pulled
`ghcr.io/tylerwhall/zephyr-rust:zephyr-rust-3.7.0-1.85.0` image.
The initial experiments used copies under
`.upgrade-logs/build-std-investigation/`, without changing production crates.
Both the library preparation and production build-std migration are now
implemented. The later sections preserve the original investigation and
preparation checkpoint; the production description below supersedes them.

## Production build-std

CMake invokes `rust/cargo.sh build` once for std and the generated application
staticlib. `CONFIG_RUST_STD=y` (default) selects `std,panic_abort`; disabling
it selects `core,alloc` and marks the generated image root no_std. The std
feature list is empty, preserving the absence of optional backtrace/unwind
features. Both generated dev and release profiles use panic=abort.

### Source discovery without an installed-toolchain change

Cargo 1.85 discovers sources from its host compiler sysroot, not RUST_LIB_SRC.
Each image gets `modules/zephyr-rust/toolchain`: copied rustc/clippy drivers
and librustc_driver establish a private default sysroot, while other installed
host resources are symlinked. Real Cargo is invoked directly, avoiding the
rustup proxy's original-toolchain dynamic-library path. This is toolchain
relocation, not a compiler-argument wrapper.

The source hierarchy under `lib/rustlib/src` mirrors the port's relative paths
with symlinks. Only the library workspace manifest and Cargo.lock are copied;
no full source tree or compiled std artifacts are copied. The staged workspace
patches libc to the pinned fork. The installed toolchain and upstream rust-src,
if present, are untouched.

Std's reviewed resolution moved unchanged from sysroot-stage1 to
`rust/Cargo.lock`. Cargo's incomplete std-workspace --locked enforcement is
handled by comparing the staged lock after every invocation, including failed
commands. Unexpected mutation fails the build and reports the diff for review.
App/standalone Clippy locks retain their separate --locked policy.

### Removed machinery and genuine no_std coverage

- Removed `rust/build.sh`, `rust/sysroot-rustc.sh`, and the custom sysroot
  manifest: no two-stage Cargo build, artifact publication, or app-directory
  invalidation based on copied rlibs remains.
- Cross-Clippy uses the same Cargo entry point; host tools use ordinary Cargo.
  Its build checks use west's exit status, not a possibly stale ELF.
- Object macros expand against zephyr-core, not the std-facing zephyr crate.
  Macro consumers declare that direct dependency.
- `samples/no_std` now really excludes target std, supplies a panic handler,
  and exercises kernel allocation plus kernel/user mutexes and syscalls.
  Removed its optional std examples; rust-app already covers those APIs.

### Migration validation

- Default smoke passed at each atomic change; the baseline used the old build.
- Fresh 113-job matrix passed (35/35/43 across 2.3.0/2.7.3/3.7.0), including
  all six expected-fault runs. Both std and genuine no_std run on all versions.
- All seven Zephyr 2.3.0 sanitycheck configurations executed and passed.
- Strict Clippy passed on 3.7.0 for host crates, libraries, and all eight apps.
  Core/sys remain non-member selections, not real Clippy roots; see
  CLIPPY_SYSROOT_DEBT.md. No lint suppression was added.
- Core/alloc-only artifact inspection found no target libstd; generated-root
  check and Clippy also passed in the dev profile.
- Regression checks rejected a mutated std lock and failed west builds with
  stale ELFs.

Artifacts: `.upgrade-logs/build-std-*` and `ci/log/build/run-1`. Validation used
only the previously pulled GHCR images. Later-version test execution still
awaits twister; the matrix does not claim to execute those tests.

An additional userspace Box test exposed the existing dedicated allocator's
privileged arch_irq_lock path (3.7.0, MempoolAlloc::alloc). It was not part of
the old sample's coverage and is not fixed by changing Cargo. The no_std sample
retains kernel allocation and user syscall coverage, not a claim of safe
userspace heap allocation. That runtime issue requires separate work.

## Library preparation checkpoint (before the switch)

- CMake generates one pair of bindings per Zephyr image. Both instances of
  zephyr-sys consume them; unchanged outputs retain their timestamps.
- Core selects rustc-std-workspace-alloc only in its std-private build.
- The generated application root registers the pool allocator once.
- C owns the shared mutex allocation bitmap in rust_std_partition. All Rust
  instances access its bytes through AtomicU8; partial bytes and exhaustion
  are handled correctly. The sample exercises exhaustion, uniqueness, reuse,
  and allocation sharing with std, in kernel and user mode.
- The std port exposes Instant::as_zephyr_ticks; zephyr::time::instant_ticks
  performs the app-side conversion and futures uses that adapter.
- Public app/helper crates have explicit Cargo dependencies and updated
  lockfiles. Core/sys depend directly on the pinned libc port. Std's core/sys
  dependencies are private, and core's Kconfig environment inputs are tracked.

At this checkpoint rust/build.sh still installed a custom sysroot and used
sysroot-rustc.sh. Source provisioning and lockfile enforcement were deferred;
both are now implemented above. The detailed probes below remain historical.

## Preparation validation

Each separate library-preparation commit passed a qemu_x86 / Zephyr 3.7.0
build and process-group-safe smoke run through the intentional userspace
fault (run status 1). Shared binding generation and the mutex bitmap also
passed on Zephyr 2.3.0 and 2.7.3. The final matrix confirms independent
std-private and ordinary app-side crate instances work together:

- 113 matrix jobs passed: 35 on 2.3.0, 35 on 2.7.3, 43 on 3.7.0.
- All six selected sample runs reached the intentional fault; rust-app
  additionally passed its shared-pool checks in kernel and user mode.
- All seven Zephyr 2.3.0 sanitycheck configurations executed and passed.
- Strict Clippy passed on 3.7.0 for host crates, libraries, and all eight
  apps/tests. Core/sys's non-member lint-root limitation remains documented
  in CLIPPY_SYSROOT_DEBT.md; this is not a claim of real Clippy coverage there.
- Fixes were folded into the corresponding commits. The post-autosquash
  tree matched the validated tree exactly, and a fresh smoke build/run passed.

The matrix also required two Rust 1.85 target-spec fixes, folded into the
upgrade: explicit ARM float ABIs and removal of an empty Cortex-R5 target
feature. All target JSONs now pass compiler parsing. No local images were
built; all validation used the pulled GHCR images.

Review initially stopped here, before the production build-std switch. Later-version
test execution still requires the separately tracked twister migration.
Local validation artifacts are in `.upgrade-logs/prep-*`; the complete
matrix result tree is `ci/log/build/run-1`.

## Conclusion

Keep the existing semantic layering:

- `zephyr-sys`: generated low-level types, constants, and FFI/syscall bindings.
- `zephyr-core`: no_std-capable context-aware wrappers usable by std or apps.
- `zephyr`: application-facing APIs, including integrations with std.

However, do **not** require std and applications to share a Rust instance of
`zephyr-core` or `zephyr-sys`. Cargo 1.85 build-std constructs separate std
and application dependency graphs. Treat the std-side builds as private
implementation dependencies and the app-side builds as ordinary Cargo
libraries. Share external Zephyr resources through their C ABI, not through
Rust crate identity. Move singleton resource ownership out of dual-built
library instances, and prevent private Rust types from crossing std's
public API boundary.

This removes the reason for publishing Zephyr crates into a custom sysroot.
Cargo supplies the bootstrap compilation flags itself, so
`rust/sysroot-rustc.sh` has been removed by the migration.

## What the pinned Cargo actually does

Inspected the Cargo source at `d73d2caf9e41a39daf2a8d6ce60ec80bf354d2a7`,
matching the Rust 1.85 release's Cargo submodule and container binary:

- `src/cargo/core/compiler/standard_lib.rs`: resolves the Rust library
  workspace independently of the application workspace.
- `src/cargo/core/compiler/unit_dependencies.rs`: maintains separate feature
  resolutions and marks std dependencies with `is_std`. Standard-library
  roots are subsequently attached to application target units.
- `src/cargo/core/compiler/mod.rs`: supplies
  `-Zforce-unstable-if-unmarked` to std units automatically.

A `-Zunstable-options --unit-graph` experiment confirmed two library units
for each of zephyr-core, zephyr-sys, and time-convert. Enabling identical
`rustc-dep-of-std` features on the application copy did **not** collapse
these into one unit. Feature unification is not a solution.

The `public = true` declarations in std's manifest also do not unify units
or turn private std dependencies into public application libraries.

## Changes by crate

### zephyr-sys: dual-buildable, stateless bindings

Keep the current generated `raw` and context-specific `syscalls` modules.
They contain declarations, not ownership of the underlying kernel objects.
Two Rust representations can therefore address the same Zephyr C resources,
provided they are generated from the same image configuration and no Rust
newtypes or ownership objects are exchanged between the two crate instances.

- Ordinary app dependencies should not enable `rustc-dep-of-std`.
- Retain that feature for std's private instance, including its core and
  compiler_builtins wiring.
- Keep generated ABI sizes, constants, device names, and C types sourced
  from the built Zephyr image. Do not hardcode them to make the split work.
- Prefer generating bindings once per Zephyr build from CMake. Each crate
  instance can include or copy the same `bindings.rs`/`syscalls.rs` inputs.
  Currently `build.rs` invokes zephyr-bindgen independently for each
  instance; that works in the probes but duplicates work.
- Add proper `rerun-if-changed` / `rerun-if-env-changed` tracking for shared
  generated inputs and image configuration. App/board configurations must
  not share binding artifacts accidentally.

Removing libc just to unify the graphs is unnecessary. Bindgen currently
uses `ctypes_prefix("libc")`, and changing to core C aliases would be a
separate ABI-sensitive change, especially given the Zephyr c_char port.

### zephyr-core: normal alloc dependency outside std, private inside std

The unconditional dependency on `../rust/library/alloc` is unsuitable for
an ordinary application crate: it explicitly introduces the library-source
alloc into the app graph rather than using the alloc supplied by build-std.

Replace it with an optional standard-workspace alias:

```toml
[dependencies]
alloc = { version = "1.0.0", optional = true,
          package = "rustc-std-workspace-alloc" }

[features]
rustc-dep-of-std = [
    "alloc", "core", "compiler_builtins/rustc-dep-of-std",
    "zephyr-sys/rustc-dep-of-std", "libc/rustc-dep-of-std",
    "time-convert/rustc-dep-of-std",
]
```

Add `extern crate alloc;` in the library. On the ordinary app side this
selects the injected alloc; inside std, the feature selects the alias
patched by Rust's library workspace. Both are needed: merely removing the
path dependency made the private std-side compilation fail with E0463.
This alias arrangement successfully compiled in the probes.

Keep core's ordinary mode no_std-capable. Its existing `have_std` feature
can host application-only std adapters, but must never be enabled by the
std dependency graph. Alternatively, keep core always no_std and put all
std adapters in `zephyr` as extension traits/functions. The latter keeps
the direction of the dependency layers clearer.

The manifest's `defaults` feature is not Cargo's special `default` feature;
it currently does not implicitly enable `rustc-dep-of-std`. Do not rename
it to `default` and thereby accidentally activate std-internal wiring for
ordinary applications.

### Global allocator: exactly one image-level registration

`zephyr-core/src/lib.rs` automatically calls `global_sys_mem_pool!` when
`mempool` is enabled. Dual-building core produces two `#[global_allocator]`
definitions. A staticlib probe reproduced the compiler error:

```text
the #[global_allocator] in zephyr_core conflicts with global allocator in: zephyr_core
```

Retain allocator implementation types and the opt-in macro in core, but
remove automatic registration from a dual-use dependency. Register once
at the application/staticlib root, possibly through a small dedicated
allocator crate or generated root code. That registration supplies alloc
for std as well as no_std application libraries; std's System allocator
remains the fallback when no pool allocator is configured.

The probe instead gated automatic registration to the std-private core
instance to establish compile feasibility. That is **not** the recommended
final design: it does not cover a core/alloc-only image and leaves allocator
ownership implicit. Source-level `no_std` alone does not imply the current
`samples/no_std` image omits std; its Cargo dependencies still use zephyr.

### Mutex pool: one allocation bitmap, not merely one C mutex array

`mutex_alloc.rs` references the shared C `rust_mutex_pool`, but its `USED`
bitmap is a Rust static. Two core instances have independent bitmaps for
the same array: std and apps could allocate the same mutex simultaneously.
Compiling successfully does not detect this bug.

Centralize pool bookkeeping in the Zephyr runtime linked once by CMake:

- expose pool allocation/free through C ABI shims, or expose one shared
  atomic bitmap with clearly specified alignment and access semantics;
- preserve context-aware syscall behavior, object permissions, and
  userspace-safe access to the bookkeeping;
- derive size from CONFIG_RUST_MUTEX_POOL_SIZE and handle the final partial
  bitmap byte correctly;
- place any C-owned userspace bookkeeping explicitly in the Rust memory
  partition. `rust-smem.ld` currently captures data from librust_app.a,
  not arbitrary C objects. `rust-smem.c` shows the established
  `RUST_STD_SECTION` placement pattern.

The Box-backed non-pool implementation has no equivalent global bitmap;
its allocations still need the single image-wide allocator.

Std's TLS key counter is already private to std and need not be duplicated
into an app crate. The extern C kernel objects in core's macros likewise
should remain externally owned rather than defined independently in each
library instance.

### Time conversion: move the cross-layer adapter

The std port currently implements `From<Instant> for zephyr_core::Ticks`
in `library/std/src/time.rs`. That implements the trait for the **private**
core instance, not the app's core. A probe using ordinary app dependencies
failed with E0277: app-side `Ticks: From<Instant>` is not satisfied.

Remove the private-core type from this public std integration:

1. expose a narrowly scoped Zephyr std accessor for Instant's underlying
   ticks using a primitive representation, with explicit semantics;
2. implement the app-side conversion in core under an app-only std feature,
   or provide an adapter in zephyr;
3. keep tick conversions tied to generated image types/rates and preserve
   overflow and deadline semantics; do not infer Instant layout or replace
   ticks with rounded milliseconds.

The probe used a temporary `Instant::as_zephyr_ticks() -> u64` accessor and
an app-side `have_std` conversion. Both the conversion and a staticlib using
std and core mutexes then compiled without a compiler wrapper. This proves
the boundary approach, not the final API design. `zephyr-futures/src/delay.rs`
currently uses `Ticks::from(Instant)` and must be covered by the migration.

### zephyr and other application crates: explicit Cargo dependencies

`rust/zephyr/Cargo.toml` has no dependencies and its source loads core/sys
from the custom sysroot. Add explicit path dependencies on the ordinary
core/sys builds. Do the same for every direct user in helper and app crates,
including macros that expand references to `::zephyr_core` / `::zephyr_sys`.
A transitive dependency is not automatically available by crate name.
Keep zephyr out of std's dependency graph.

This also makes standalone Cargo/Clippy checks of the public core/sys
packages possible in principle; workspace and committed-lockfile policy
still need deliberate changes rather than claiming the existing Clippy
limitations are automatically solved.

## Build integration and source discovery

Replace the two independent Cargo invocations plus manual sysroot copying
with an application build such as:

```sh
cargo build --release --target "$RUST_TARGET_SPEC" \
    -Zbuild-std=std,alloc,core,panic_abort -Zbuild-std-features=
```

The explicit panic_abort root was needed by the staticlib probe. The empty
feature list preserves the present lack of enabled std backtrace/unwind
features; the final configuration must be checked against build.sh and
all supported configurations rather than taking Cargo's defaults.

For Cargo 1.85 source discovery:

- it looks under the compiler sysroot at `lib/rustlib/src/rust/library`;
- it does **not** consult RUST_LIB_SRC in the inspected implementation;
- `__CARGO_TESTS_ONLY_SRC_ROOT` worked for early probes, but is an internal
  test escape hatch, not a production interface;
- symlinking only `library` into the normal discovery location broke the
  std manifest's `../../../zephyr-core` relative paths;
- symlinking the entire staged `rust/` hierarchy to `lib/rustlib/src`,
  preserving `src/rust/library`, `src/zephyr-core`, `src/zephyr-sys`, and
  `src/libc`, successfully built the probe through normal source discovery,
  with no source-override environment variable or compiler wrapper.

Use a writable source staging area for the library workspace/lockfile and
patch libc in **that workspace**, not only the app manifest. The resolutions
are separate. Cargo 1.85's source explicitly notes incomplete --locked
handling for the std workspace, so app --locked alone is insufficient
assurance. Pin/review the std lockfile and detect unexpected mutation.

A production solution must also cover native development without modifying
a user's shared installed toolchain unexpectedly. Source provisioning,
container setup, CMake environments, and every Cargo/Clippy invocation need
to be migrated together. No build-std flag on host-only bindgen/macro builds
is necessary.

## Historical probe evidence and original migration plan

Local logs and copied sources are under
`.upgrade-logs/build-std-investigation/`:

- `setup-probe.log`, `probe/unit-graph.json`: separate crate units.
- `probe/unit-graph-matching-features.json`: matching features still do not
  unify std and app units.
- `probe-step2.log`: alloc alias works; original Instant conversion fails.
- `probe-step3.log`: duplicate global allocator and missing panic_abort root.
- `probe-step4.log`: staticlib compiles after allocator gating and explicit
  panic_abort selection.
- `probe-step5.log`: primitive Instant bridge and app-side conversion compile.
- `supported-source-path2.log`: successful normal Cargo source discovery.
- `probe-no-std.log`: a truly no_std staticlib using only build-std=core,alloc,
  an app-root pool allocator, and an app panic handler also compiled through
  normal source discovery without a wrapper.

Those initial probes were compile/static-archive checks only: their copied
mutex bitmap remained unsafe and no restructured image was run at that time.
The subsequent production library preparation and runtime/matrix validation
are recorded above; they still do not validate a production build-std switch.

Original suggested implementation order (library steps are now complete):

1. Normalize core's alloc dependency and explicit application dependencies.
2. Centralize allocator registration and mutex-pool ownership; test pool
   exhaustion and concurrent allocations spanning std and app wrappers.
3. Replace the Instant/private-Ticks public boundary and test futures timers.
4. Stage ported sources/lockfiles and replace manual sysroot publication with
   build-std; remove the wrapper only when this path passes a fresh smoke run.
5. Validate std-enabled and truly core/alloc-only images, each affected
   configuration on all supported Zephyr versions, then the full CI matrix,
   tests, and real Clippy coverage of core/sys.
