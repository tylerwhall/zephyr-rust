# Known clippy debt: sysroot-layer crates

The sysroot-layer crates (`zephyr-sys`, `zephyr-core`, `time-convert`) are
built as part of the Rust std port (see `rust/rust/library/std/Cargo.toml`
and `rust/build.sh`), not as standalone crate workspaces. This means
`ci/clippy.sh` cannot actually run clippy on them, only rustc. Their known
clippy warnings are therefore invisible to the normal clippy/report flow and
are tracked here until a structural fix or toolchain upgrade makes them
lintable for real.

## Why plain clippy cannot see them

`cargo clippy` injects `clippy-driver` via `RUSTC_WORKSPACE_WRAPPER`, which
cargo applies **only to workspace members**. `ci/clippy.sh` lints the
sysroot-layer crates with `-p` against the `rust/sysroot-stage1` workspace
manifest, where they are non-member path dependencies of `std`. Cargo
compiles non-member packages with plain `rustc`, so only rustc lints
(obeying `--cap-lints`) can surface; clippy lints never run. Verified in
`cargo -vv` logs: the `zephyr_core` compile is `/bin/rustc`, not
`clippy-driver`.

For reference, the only mechanism found that does run real clippy on them is
`cargo clippy --fix` (the fix/diagnostics-server path). It is not used in
the tooling on purpose: it applies fixes without a preceding clippy report
(the report under-counts while `--fix` over-applies), and some of its
machine-applicable suggestions are unsafe for this project (see below).

## Reproducing the debt (how to list the warnings)

`--fix` is the only way to surface them today. Run it in a throwaway or
writable checkout, then inspect the diff and revert:

```
cd ci
WRITABLE=1 ./build-cmd.sh sh -c '
  . /tmp/zephyr-rust-clippy/rust-app/rust-env.sh
  CARGO_TARGET_DIR=/tmp/zephyr-rust-clippy/cargo-target \
  RUSTFLAGS="--sysroot $SYSROOT" \
  cargo clippy --manifest-path rust/sysroot-stage1/Cargo.toml \
    -p zephyr-core --target "$RUST_TARGET_SPEC" \
    --fix --allow-dirty --allow-staged --lib'
git diff rust/zephyr-core/   # applied fixes (machine-applicable)
git checkout rust/zephyr-core/   # discard, or keep per-item after review
```

The run prints "Fixed <file> (N fixes)" for what clippy can fix itself, and
the remaining (non-machine-applicable) warnings above the summary. Run the
same `-p zephyr-sys` and `-p time-convert` for those crates (debt unknown,
expected smaller: generated bindings and a small conversion crate).

## Debt inventory (zephyr-core, Zephyr 3.7.0 / Rust 1.75.0, from the
reproduction above)

Machine-fixable (what `--fix` would change; already reviewed as semantically
sound for this code, except where noted):

- `rust/zephyr-core/src/kobj.rs`: `use crate::kobj::*;` -> `use $crate::kobj::*;`
  inside the `make_static_wrapper!` macro (macro-hygiene fix).
- `rust/zephyr-core/src/semaphore.rs`: drop `&self` reborrow in
  `k_sem_init` (`needless_borrow`); drop `.into()` on `K_FOREVER`,
  `K_NO_WAIT`, and `k_sem_count_get` (`useless_conversion`). **Caution:**
  these conversions are in Zephyr-version-sensitive code (`zephyr250` cfg
  branches exist); verify they still compile on every supported Zephyr
  version (build matrix) before committing.
- `rust/zephyr-core/src/mutex.rs`: `pub fn lock<'a, C>(&'a self) ->
  MutexGuard<'a, T, C>` -> `pub fn lock<C>(&self) -> MutexGuard<'_, T, C>`
  (`needless_lifetimes`); drop `&*` in the `Clone` impl's
  `MutexData::new(&*self.data.0.get())` (`needless_borrow`, same value).

Remaining, not machine-fixable:

- `clippy::missing_safety_doc` (22): unsafe fns and an unsafe trait across
  `kobj.rs`, `semaphore.rs`, `poll_signal.rs`, `poll.rs`, `mutex.rs`,
  `mutex_alloc.rs`, `memdomain.rs`, `lib.rs`. Add `# Safety` sections
  (see the docs added to `rust/zephyr/src/eeprom.rs` for the style).
  Representative locations: `mutex.rs:21,22,26,33-36,76,83`;
  `kobj.rs:11,33,79`; `semaphore.rs:18,65`; `poll_signal.rs:15,67`;
  `poll.rs:16`; `mutex_alloc.rs:32`; `memdomain.rs:15`; `lib.rs:61`.
- `clippy::unused_parens` (1): `rust/zephyr-core/src/semaphore.rs:99`,
  `(zephyr_sys::raw::K_NO_WAIT)` -> `zephyr_sys::raw::K_NO_WAIT`.
- `clippy::blocks_in_if_conditions` (1):
  `rust/zephyr-core/src/mutex_alloc.rs:79` — hoist the block into a `let`
  binding.

## Structural fix / version exploration (picked up later, no other context)

Goal: make `ci/clippy.sh` genuinely run clippy on the sysroot-layer crates.
Options investigated (all blocked as of Rust 1.75.0 / cargo in this repo):

1. **Workspace members.** Adding `[workspace] members` for them under
   `rust/sysroot-stage1/Cargo.toml` fails: cargo requires members to be
   hierarchically below the workspace root, and the natural root (`rust/`)
   would absorb every other crate under `rust/` (zephyr, zephyr-logger,
   zephyr-futures, zephyr-macros, zephyr-uart-buffered) into one workspace,
   breaking the per-crate committed `Cargo.lock`s the clippy pass depends on
   (`--locked` invariant). Each crate could opt out with an empty
   `[workspace]` section in its manifest, but that is an invasive,
   cross-cutting change.
2. **Standalone linting** from each crate's own manifest. Blocked by:
   - their `Cargo.lock` is intentionally gitignored (they are built from the
     sysroot workspace lock), so the committed-lock / `--locked` invariant
     cannot extend to them;
   - a fresh build-script compile with `RUSTFLAGS="--sysroot <custom>"` fails
     (`E0463: can't find crate for std` — the custom sysroot has no host
     std; the `-p` pass only survives by reusing build-script artifacts
     cached from the `west build`);
   - standalone resolution picks `compiler_builtins 0.1.160`, whose
     edition-2024 manifest cargo 1.75 cannot parse (pinning to the sysroot's
     0.1.103 fixes that, but not the build-script problem).
3. **`RUSTC_WRAPPER=clippy-driver`** to wrap non-members: incompatible —
   cargo passes the real rustc path as an argument, which clippy-driver
   rejects ("multiple input filenames").

Revisit when: a newer cargo/clippy changes the `-p`/member wrapping behavior
or adds a way to lint path deps; the sysroot layout is restructured (e.g.
the crates get their own workspaces/locks); or `cargo clippy --fix` becomes
acceptable as a discovery tool with a manual review gate. When the fix
lands, refresh the inventory above, fix it per warning type (one commit per
lint, `# Safety` docs for `missing_safety_doc`), and run the full build
matrix to validate the version-sensitive `.into()`/conversion changes.