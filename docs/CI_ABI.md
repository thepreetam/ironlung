# CI and ABI Export Contract

This document describes how the cdylib export and ABI check are maintained so CI stays green on every push.

## Required Exports

The cdylib **must** export at least the symbols listed in [crates/ironlung-abi-check/symbols.baseline](../crates/ironlung-abi-check/symbols.baseline), including:

- `printf`, `fprintf`, `vprintf` (from the C printf shim)
- `rust_eh_personality` (stub for cdylib)
- All other interceptors (malloc, read, write, pthread_*, etc.)

## How printf/fprintf/vprintf Are Exported

Export is achieved by:

1. **Compile** [csrc/printf_shim.c](csrc/printf_shim.c) to `printf_shim.o` (in `OUT_DIR`) with `-fPIC` and `-fvisibility=default`.
2. **Link** that `.o` into the cdylib by passing its absolute path as a link-arg (via `-Wl,<path>`).
3. **Force undefined references** so the linker pulls in the object: `-Wl,-u,printf`, `-Wl,-u,fprintf`, `-Wl,-u,vprintf`, `-Wl,-u,snprintf`.
4. **Force export** into the dynamic symbol table. On **x86_64** only, a linker version script (generated in `OUT_DIR`) assigns all baseline symbols to **GLIBC_2.2.5** and is passed as `-Wl,--version-script=<path>`. On other Linux arches (e.g. arm64), the version script is omitted so the build succeeds (rustc injects its own script for cdylib; combining it with ours triggers "anonymous version tag cannot be combined with other version tags"). Symbols are still exported on all arches; only x86_64 gets GLIBC_2.2.5 versioning.

All of the above are done in [build.rs](../build.rs) when `target_os = "linux"`.

## GLIBC version aliases (Phase 8)

To improve compatibility with binaries built against older glibc (e.g. when using LD_PRELOAD), the build applies a single version script that assigns **GLIBC_2.2.5** to every symbol in [symbols.baseline](../crates/ironlung-abi-check/symbols.baseline). The script is generated in `OUT_DIR` from the baseline so the same set of symbols is exported as `sym@GLIBC_2.2.5` / `sym@@GLIBC_2.2.5`. The ABI checker treats versioned names (any `sym@...`) as matching the baseline name `sym`.

## Do Not Remove

To avoid CI regressions ("missing symbols: fprintf, printf, vprintf"):

- **Do not remove or weaken** the version script, the `printf_shim.o` link step, or the `-Wl,-u,*` flags in [build.rs](../build.rs) without updating the ABI check and baseline.
- **Do not remove** `printf`, `fprintf`, `vprintf`, or `snprintf` from [crates/ironlung-abi-check/symbols.baseline](../crates/ironlung-abi-check/symbols.baseline).
- **Do not change** the ABI checker so that versioned symbol names (e.g. `printf@GLIBC_2.2.5`) no longer match the baseline name `printf`. The checker intentionally treats a baseline symbol as found when the dynamic symbol table has that name or a versioned form (name followed by `@`).

## ABI Check Step in CI

In [.github/workflows/distro-matrix.yml](../.github/workflows/distro-matrix.yml), the ABI check step:

1. Runs `cargo clean -p ironlung` so the `.so` is always built with the current build.rs and version script (no stale artifact).
2. Builds `ironlung` release with sandbox feature.
3. Runs `nm -D -g` on the `.so` and greps for printf/fprintf/vprintf so failures show whether symbols are missing vs. version mismatch.
4. Runs `cargo run -p ironlung-abi-check -- "$SO"` to compare dynamic symbols against the baseline.

Do not run the ABI check on a `.so` built in a previous step without first ensuring a clean rebuild of the ironlung package.

## Optional layout check

To fail when ABI-critical type layouts drift on x86_64-unknown-linux-gnu, build and run the ABI checker with the layout check enabled: `cargo run -p ironlung-abi-check --features layout-check -- "$SO"` with `IRONLUNG_LAYOUT_CHECK=1` set. This asserts `size_of::<libc::pthread_mutex_t>()` against a pinned value. Off by default; not run in CI.
