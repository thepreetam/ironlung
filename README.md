# IronLung

A no_std Rust shared object (.so) acting as a partial libc replacement via LD_PRELOAD.

> "Trust No Pointer, Verify Every Byte, Delegate to the Kernel."

**Status:** Experimental. Use at your own risk. Not a full libc replacement—focuses on a critical subset (allocator, string ops, stdio, networking) and hardens complex functions via process sandboxing. See [Implemented](#implemented) for coverage. No guarantee of ABI completeness, support, or compatibility with all programs. Architecture (delegation vs reimplementation rule): [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

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
- **printf** / **vprintf** / **fprintf** — format validation, delegates to libc
- **fopen** / **fclose** / **fread** / **fwrite** / **fgets**
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
