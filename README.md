# IronLung

A no_std Rust shared object (.so) acting as a partial libc replacement via LD_PRELOAD.

> "Trust No Pointer, Verify Every Byte, Delegate to the Kernel."

**Status:** Experimental. Use at your own risk. Not a full libc replacement—focuses on a critical subset (allocator, string ops, stdio, networking) and hardens complex functions via process sandboxing. See [Implemented](#implemented) for coverage and [Gaps and limitations](#gaps-and-limitations) for what is out of scope or optional. No guarantee of ABI completeness, support, or compatibility with all programs. Architecture: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). ABI and CI: [docs/CI_ABI.md](docs/CI_ABI.md).

## Target

x86_64 Linux

## Performance

I/O hot paths (`read`, `write`, `send`, `recv`) use direct syscalls with ≤1% overhead vs baseline glibc.

## Build

```bash
cargo build --release
```

From macOS (or non-Linux host), use Docker:

```bash
docker run --rm -v "$(pwd)":/app -w /app rust:latest cargo build --release
```

Output: `target/release/libironlung.so`

### Sandbox (getaddrinfo containment)

Build the sandbox helper (Linux only; requires libseccomp-dev):

```bash
cargo build --release --features sandbox -p ironlung
cargo build --release -p ironlung-sandbox
```

Output: `target/release/libironlung.so` and `target/release/ironlung-sandbox`

Run with sandboxed getaddrinfo:

```bash
LD_PRELOAD=./target/release/libironlung.so IRONLUNG_SANDBOX_PATH=./target/release/ironlung-sandbox ./victim
```

**CI:** The sandbox test step in CI is best-effort (`continue-on-error: true`) because it can hang in containerized runners. Run `sh scripts/test_sandbox.sh` locally for getaddrinfo validation.

### Optional features

| Feature | Effect |
|--------|--------|
| `sandbox` | getaddrinfo/freeaddrinfo run in a seccomp-contained helper process instead of delegating to libc in-process. |
| `alloc-cache` | Per-thread free-list for one size class to reduce allocator contention (see [docs/ALLOCATOR.md](docs/ALLOCATOR.md)). |
| `stdio-kernel` | Kernel-delegating `vprintf` (subset of specifiers, writes via `write(1, …)`); see [docs/STDIO_KERNEL.md](docs/STDIO_KERNEL.md). |
| `stdio-kernel-fread-fwrite` | `fread`/`fwrite` use `fileno(stream)` + `read`/`write` syscalls (Linux). |

Default build uses none of these; CI builds with `sandbox` only.

## Test

```bash
gcc victim.c -o victim

# Normal run (glibc)
./victim

# IronLung run (intercepted)
LD_PRELOAD=./target/release/libironlung.so ./victim
```

With IronLung, you should see `[IronLung]` prefixed on `puts` output.

## Implemented

### Core (kernel-delegating)
- **malloc** / **free** / **realloc** / **calloc** — Rust-managed heap via talc + mmap
- **memcpy** / **memmove** — bounds and overlap checks
- **strcpy** — look-ahead safe copy with heuristic limit
- **puts** — syscall-based write with `[IronLung]` tag

### Phase 1: Concurrency
- **pthread_*** — delegates to system libc via dlsym(RTLD_NEXT)
- Allocator documentation and benchmarks (`docs/ALLOCATOR.md`, `tests/alloc_bench.c`)
- Async-signal-safety audit (`docs/SIGNAL_SAFETY.md`)

### Phase 2: I/O and System
- **printf** / **vprintf** / **fprintf** — format validation, delegate to libc (optional kernel path: `stdio-kernel`)
- **fopen** / **fclose** / **fread** / **fwrite** / **fgets** — delegate to libc (optional kernel path: `stdio-kernel-fread-fwrite`)
- **open** / **read** / **write** / **close** / **fork** / **execve**
- **socket** / **bind** / **listen** / **accept** / **connect** / **send** / **recv**

### Phase 3: Complex Services
- **getaddrinfo** / **freeaddrinfo**
- **iconv_open** / **iconv** / **iconv_close**
- **getpwnam** / **crypt**

### Phase 4: Validation
- **CI (smoke):** Distro matrix (build, smoke test, ABI check, app matrix), plus doppelgänger fuzz job (best-effort).
- **Full validation (manual):** Run `scripts/run_glibc_tests.sh [glibc_build_dir]` with a built glibc tree for conformance; run `scripts/doppelganger_fuzz.py` locally for more iterations. Optionally trigger the **Glibc validation** workflow from the Actions tab (workflow_dispatch) to run the glibc test suite in CI (best-effort, continue-on-error).
- Scripts: `scripts/run_glibc_tests.sh`, `scripts/run_app_matrix.sh`, `scripts/doppelganger_fuzz.py`.

## Gaps and limitations

- **Critical subset only** — Not a full libc; many symbols are not implemented. Rely on [symbols.baseline](crates/ironlung-abi-check/symbols.baseline) and [docs/CI_ABI.md](docs/CI_ABI.md) for the export contract.
- **No full glibc conformance guarantee** — Glibc test suite and heavy fuzz are optional/manual; CI does not gate on them.
- **Optional kernel-delegating stdio** — Kernel-path printf and fread/fwrite are behind features (`stdio-kernel`, `stdio-kernel-fread-fwrite`); default is validate-then-delegate to libc.
- **No wide char / full locale** — Beyond current iconv delegation; out of scope.
- **No allocator quarantine or shadow memory** — Out of scope for current plan.
- **Sandbox and glibc CI** — Sandbox test and Glibc validation workflow are best-effort in CI (continue-on-error); run locally or manually when needed.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for future work and delegation rules.

## Documentation

| Doc | Description |
|-----|-------------|
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | Delegation vs reimplementation, errno, ABI and types, future work |
| [CI_ABI.md](docs/CI_ABI.md) | Cdylib export contract, printf export mechanism, optional layout check |
| [ALLOCATOR.md](docs/ALLOCATOR.md) | Talc allocator, per-thread cache (optional), benchmarks |
| [SIGNAL_SAFETY.md](docs/SIGNAL_SAFETY.md) | Async-signal-safe set and tests |
| [ERRNO.md](docs/ERRNO.md) | errno contract and tests |
| [STDIO_KERNEL.md](docs/STDIO_KERNEL.md) | Kernel-delegating printf and fread/fwrite (optional features) |
