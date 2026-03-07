# IronLung: Rust-based Memory Hardener

A no_std Rust shared object (.so) providing memory safety hardening for C/C++ applications via LD_PRELOAD.

> "Memory Safety Through Quarantine, Performance Through Delegation."

**Status:** v1.1 — Strategic pivot to memory hardening focus. UAF protection via quarantine, performance through thread-local caching, and safe delegation to system libc. Experimental; use at your own risk. Not a libc replacement—embraces delegation to system libc for complex operations. See [Implemented](#implemented) for coverage and [Gaps and limitations](#gaps-and-limitations) for what is out of scope. Architecture: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). ABI and CI: [docs/CI_ABI.md](docs/CI_ABI.md).

## Target

x86_64 Linux

## Performance

I/O hot paths (`read`, `write`, `send`, `recv`) use direct syscalls with ≤1% overhead vs baseline glibc.

## When to Use IronLung

Use IronLung if you need memory safety hardening for existing Linux binaries, particularly UAF (Use-After-Free) protection through quarantine. IronLung is a **memory hardener**, not a libc replacement—it delegates complex operations to system libc while adding safety checks and hardening.

**Key Features:**
- **UAF Protection**: Quarantine allocator delays reuse of freed memory
- **Thread-Local Caching**: Slab allocator cache per thread reduces lock contention
- **Memory Safety**: Bounds checking on string/memory operations
- **Performance**: Direct syscalls for hot paths, vDSO timekeeping
- **Delegation**: Complex operations (DNS, etc.) safely delegated to system libc

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

| Feature | Effect | Performance Impact | Security Benefit |
|--------|--------|-------------------|------------------|
| `sandbox` | getaddrinfo/freeaddrinfo run in a seccomp-contained helper process instead of delegating to libc in-process. | High (fork+exec per call) | High (DNS isolation) |
| `alloc-cache` | Per-thread free-list for 32‑256 byte allocations to reduce allocator contention. **Default on.** | Low (cache hit) → High (avoid global lock) | None |
| `alloc-quarantine` | Per‑thread ring‑buffer quarantine of freed blocks to delay reuse (UAF mitigation). **Opt‑in.** Configure with `IRONLUNG_QUARANTINE_SIZE`. | Low (no global lock) → Medium (memory bloat) | High (UAF mitigation) |
| `alloc-mimalloc` | Use mimalloc as backend allocator for high scalability. **Opt‑in.** | Very High (scalable) | None |
| `stdio-kernel` | Kernel‑delegating `vprintf` with 4 KB buffering (subset of specifiers). **Opt‑in.** | Medium (buffered syscalls) | Medium (format validation) |
| `stdio-kernel-fread-fwrite` | `fread`/`fwrite` use `fileno(stream)` + `read`/`write` syscalls (Linux). **Opt‑in.** | High (syscall per I/O) | Low |
| `stdio-libc` | Force printf/fread/fwrite to delegate to libc (turns off kernel path when set). **Default on.** | Low (libc buffering) | None |
| `hosted-test` | Enable std for hosted testing (development only). | N/A | N/A |

Default build enables `alloc‑cache` and `stdio‑libc` for best performance. Add `alloc‑quarantine` only when UAF hardening is required. Use `alloc‑mimalloc` for high‑concurrency workloads.

## Test

```bash
gcc victim.c -o victim

# Normal run (glibc)
./victim

# IronLung run (intercepted)
LD_PRELOAD=./target/release/libironlung.so ./victim
```

With IronLung, you should see `[IronLung]` prefixed on `puts` output.

**Unit tests:** The main crate is no_std with `panic = "abort"`. Running `cargo test` hits duplicate-`core` / lang-item issues with `-Z build-std`. Use `cargo test --features hosted-test` for hosted functional tests. Validation is **build + victim + LD_PRELOAD** (above), **concurrent stress tests**, and **doppelganger fuzz** (see Phase 4).

## Implemented

### Memory Hardening Core
- **malloc** / **free** / **realloc** / **calloc** — Rust-managed heap with UAF quarantine
- **Thread-Local Slab Cache** — Per-thread caching for 8 size classes (16B to 2048B)
- **Bootstrap Buffer** — Safe dlsym initialization without recursion
- **memcpy** / **memmove** — bounds and overlap checks with size limits
- **strcpy** — look-ahead safe copy with heuristic limit

### Performance Optimizations
- **clock_gettime** / **gettimeofday** — Direct syscalls with vDSO optimization
- **read** / **write** / **send** / **recv** — Direct syscalls for hot paths
- **puts** — syscall-based write with `[IronLung]` tag

### Safe Delegation (validate-then-delegate)
- **memset** / **strcmp** / **strncpy** — size-bounded, delegate to system libc
- **snprintf** — format validation (reject %n), delegate to system libc
- **getenv** — name length limit, delegate to system libc
- **pthread_*** — delegates to system libc for thread creation and synchronization
- **getaddrinfo** / **freeaddrinfo** — input validation, delegate to system libc
- **open** / **close** / **fork** / **execve** — delegate to system libc
- **socket** / **bind** / **listen** / **accept** / **connect** — delegate to system libc

### Features
- **alloc-tls-cache** — Thread-local slab cache (default)
- **alloc-quarantine** — UAF protection via delayed memory reuse
- **alloc-mimalloc** — High-performance mimalloc backend
- **hosted-test** — Enable std for testing

## Gaps and limitations

- **Critical subset only** — Not a full libc; many symbols are not implemented. Rely on [symbols.baseline](crates/ironlung-abi-check/symbols.baseline) and [docs/CI_ABI.md](docs/CI_ABI.md) for the export contract.
- **Glibc conformance** — Full glibc test suite runs on release (workflow fails if it fails), weekly schedule, and manual trigger; push CI does not gate on it.
- **Kernel stdio default** — On Linux, kernel-path printf and fread/fwrite are the default; use feature `stdio-libc` to force delegate to libc.
- **Wide char** — Minimal wchar delegation (`wcslen`, `wcscpy`, `wcsncpy`, `wcscmp`); full locale out of scope.
- **Quarantine / UAF hardening** — v1.0 provides **memory safety** (allocator internal consistency) and optional **exploit mitigation** via **quarantine** (delayed reuse of freed memory). Quarantine is **opt‑in** via the `alloc‑quarantine` feature. Runtime size can be configured with `IRONLUNG_QUARANTINE_SIZE` (0 to disable, defaults to 512 KiB). See [docs/ALLOCATOR_QUARANTINE.md](docs/ALLOCATOR_QUARANTINE.md).
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
