# IronLung

A no_std Rust shared object (.so) acting as a partial libc replacement via LD_PRELOAD.

> "Trust No Pointer, Verify Every Byte, Delegate to the Kernel."

**Status:** v1.0 — memory-safe critical subset (allocator, string ops, stdio, networking) with **UAF hardening (quarantine) on by default** for exploit mitigation. Experimental; use at your own risk. Not a full libc replacement. Hardens complex functions via process sandboxing. See [Implemented](#implemented) for coverage and [Gaps and limitations](#gaps-and-limitations) for what is out of scope or optional. No guarantee of ABI completeness, support, or compatibility with all programs. Architecture: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). ABI and CI: [docs/CI_ABI.md](docs/CI_ABI.md).

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
| `alloc-quarantine` | FIFO quarantine of freed blocks to delay reuse (UAF mitigation). **Default on.** Disable with `default-features = false` and do not add `alloc-quarantine`. |
| `stdio-kernel` | Kernel-delegating `vprintf` (subset of specifiers, writes via `write(1, …)`); see [docs/STDIO_KERNEL.md](docs/STDIO_KERNEL.md). **Default on.** |
| `stdio-kernel-fread-fwrite` | `fread`/`fwrite` use `fileno(stream)` + `read`/`write` syscalls (Linux). **Default on.** |
| `stdio-libc` | Force printf/fread/fwrite to delegate to libc (turns off kernel path when set). |
| `pthread-native` | Linux x86_64: pthread_create/join/mutex/cond via clone3+futex (no libc). See [PTHREAD_NATIVE.md](docs/PTHREAD_NATIVE.md). |

Default build enables the kernel stdio path on Linux; CI builds with `sandbox` plus defaults.

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

### String and env (validate-then-delegate)
- **memset** / **strcmp** / **strncpy** — size-bounded, delegate to libc
- **snprintf** — format validation (reject %n), delegate to libc
- **getenv** — name length limit, delegate to libc. See [docs/SUBSET.md](docs/SUBSET.md).

### Phase 1: Concurrency
- **pthread_*** — with feature `pthread-native`: create/join/mutex/cond via clone3+futex (no libc). Otherwise delegates to libc. See [docs/PTHREAD_NATIVE.md](docs/PTHREAD_NATIVE.md).
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
- **wcslen** / **wcscpy** / **wcsncpy** / **wcscmp** — minimal wchar delegate
- **getpwnam** / **crypt**

### Phase 4: Validation
- **CI (smoke):** Distro matrix (build, smoke test, ABI check, app matrix), plus doppelgänger fuzz job (best-effort).
- **Full validation (manual):** Run `scripts/run_glibc_tests.sh [glibc_build_dir]` with a built glibc tree for conformance; run `scripts/doppelganger_fuzz.py` locally for more iterations. Optionally trigger the **Glibc validation** workflow from the Actions tab (workflow_dispatch) to run the glibc test suite in CI (best-effort, continue-on-error).
- **1M fuzz on a VPS (for launch/graph):** On a cheap Linux VPS, build then run: `nohup ./scripts/run_fuzz_vps.sh > fuzz_out.txt 2>&1 &`. Logs `fuzz_log.csv` (iteration, crashes, timestamp) for a “1 Million Fuzz Iterations / 0 Crashes” graph. `python3 scripts/doppelganger_fuzz.py --iterations 1000000 --progress-every 10000 --csv fuzz_log.csv` does the same in the foreground.
- Scripts: `scripts/run_glibc_tests.sh`, `scripts/run_app_matrix.sh`, `scripts/doppelganger_fuzz.py`, `scripts/run_fuzz_vps.sh`.

## Gaps and limitations

- **Critical subset only** — Not a full libc; many symbols are not implemented. Rely on [symbols.baseline](crates/ironlung-abi-check/symbols.baseline) and [docs/CI_ABI.md](docs/CI_ABI.md) for the export contract.
- **Glibc conformance** — Full glibc test suite runs on release (workflow fails if it fails), weekly schedule, and manual trigger; push CI does not gate on it.
- **Kernel stdio default** — On Linux, kernel-path printf and fread/fwrite are the default; use feature `stdio-libc` to force delegate to libc.
- **Wide char** — Minimal wchar delegation (`wcslen`, `wcscpy`, `wcsncpy`, `wcscmp`); full locale out of scope.
- **Quarantine / UAF hardening** — v1.0 provides **memory safety** (allocator internal consistency) and **exploit mitigation** via **quarantine** (delayed reuse of freed memory) **on by default**. Use `default-features = false` and omit `alloc-quarantine` to get memory safety only. See [docs/ALLOCATOR_QUARANTINE.md](docs/ALLOCATOR_QUARANTINE.md).
- **Sandbox and glibc CI** — Sandbox test and Glibc validation workflow are best-effort in CI (continue-on-error); run locally or manually when needed.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for future work and delegation rules.

## Documentation

| Doc | Description |
|-----|-------------|
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | Delegation vs reimplementation, errno, ABI and types, future work |
| [CI_ABI.md](docs/CI_ABI.md) | Cdylib export contract, printf/version script, GLIBC version aliases, optional layout check |
| [ALLOCATOR.md](docs/ALLOCATOR.md) | Talc allocator, per-thread cache (optional), benchmarks |
| [SIGNAL_SAFETY.md](docs/SIGNAL_SAFETY.md) | Async-signal-safe set and tests |
| [ERRNO.md](docs/ERRNO.md) | errno contract and tests |
| [STDIO_KERNEL.md](docs/STDIO_KERNEL.md) | Kernel-delegating printf and fread/fwrite (default on Linux) |
| [SUBSET.md](docs/SUBSET.md) | Curated symbol subset and next-tier process |
| [ALLOCATOR_QUARANTINE.md](docs/ALLOCATOR_QUARANTINE.md) | Quarantine/shadow design; v1.0 UAF mitigation (default on) |
| [PTHREAD_NATIVE.md](docs/PTHREAD_NATIVE.md) | Native pthread create/join/mutex/cond via clone3+futex (optional feature) |
| [DISTRO_MATRIX.md](docs/DISTRO_MATRIX.md) | Target distros and CI integration |
| [BENCHMARKS.md](docs/BENCHMARKS.md) | I/O and allocator benchmarks |
