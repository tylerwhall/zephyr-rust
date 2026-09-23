# Rust upgrade history

Running log of zephyr-rust Rust version upgrades: every important decision
and conflict, per `docs/rust-upgrade.md`. Newest first.

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
