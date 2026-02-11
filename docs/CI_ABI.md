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
3. **Force undefined references** so the linker pulls in the object: `-Wl,-u,printf`, `-Wl,-u,fprintf`, `-Wl,-u,vprintf`.
4. **Force export** into the dynamic symbol table using a linker version script `printf_export.ver` (generated in `OUT_DIR`) with content: `IRONLUNG_1.0 { global: printf; fprintf; vprintf; };`, passed as `-Wl,--version-script=<path>`.

All of the above are done in [build.rs](../build.rs) when `target_os = "linux"`.

## Do Not Remove

To avoid CI regressions ("missing symbols: fprintf, printf, vprintf"):

- **Do not remove or weaken** the version script, the `printf_shim.o` link step, or the `-Wl,-u,*` flags in [build.rs](../build.rs) without updating the ABI check and baseline.
- **Do not remove** `printf`, `fprintf`, or `vprintf` from [crates/ironlung-abi-check/symbols.baseline](../crates/ironlung-abi-check/symbols.baseline).
- **Do not change** the ABI checker so that versioned symbol names (e.g. `printf@IRONLUNG_1.0`) no longer match the baseline name `printf`. The checker intentionally treats a baseline symbol as found when the dynamic symbol table has that name or a versioned form (name followed by `@`).

## ABI Check Step in CI

In [.github/workflows/distro-matrix.yml](../.github/workflows/distro-matrix.yml), the ABI check step:

1. Runs `cargo clean -p ironlung` so the `.so` is always built with the current build.rs and version script (no stale artifact).
2. Builds `ironlung` release with sandbox feature.
3. Runs `nm -D -g` on the `.so` and greps for printf/fprintf/vprintf so failures show whether symbols are missing vs. version mismatch.
4. Runs `cargo run -p ironlung-abi-check -- "$SO"` to compare dynamic symbols against the baseline.

Do not run the ABI check on a `.so` built in a previous step without first ensuring a clean rebuild of the ironlung package.

## Optional layout check

To fail when ABI-critical type layouts drift on x86_64-unknown-linux-gnu, build and run the ABI checker with the layout check enabled: `cargo run -p ironlung-abi-check --features layout-check -- "$SO"` with `IRONLUNG_LAYOUT_CHECK=1` set. This asserts `size_of::<libc::pthread_mutex_t>()` against a pinned value. Off by default; not run in CI.
