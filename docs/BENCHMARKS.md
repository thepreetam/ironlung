# IronLung Benchmarks

## I/O Throughput (read/write on pipe)

Run with and without LD_PRELOAD to compare overhead.

```bash
gcc -O2 -o io_bench tests/io_bench.c

# Baseline (glibc)
./io_bench 100000 4096

# IronLung
LD_PRELOAD=./target/release/libironlung.so ./io_bench 100000 4096
```

**Target:** IronLung overhead ≤ 1% vs baseline. **Achieved:** I/O throughput with IronLung is within 1% of baseline (typically comparable or slightly faster due to direct syscalls).

Example (100k iterations, 4K buffer):
- Baseline: ~1.6–1.8M read+write ops/s
- IronLung: ~1.7–1.9M read+write ops/s

## Allocator Throughput (malloc/free)

```bash
gcc -O2 -lpthread -o alloc_bench tests/alloc_bench.c

# Baseline (glibc malloc)
./alloc_bench 100000 4

# IronLung default (talc with per-thread cache)
LD_PRELOAD=./target/release/libironlung.so ./alloc_bench 100000 4

# IronLung with mimalloc backend
LD_PRELOAD=./target/release/libironlung.so ./alloc_bench 100000 4

# Concurrent stress test
gcc -O2 -lpthread -o concurrent_alloc tests/concurrent_alloc.c
LD_PRELOAD=./target/release/libironlung.so ./concurrent_alloc 16 10000 1024
```

IronLung offers multiple allocator backends:
- **Default**: talc with per‑thread cache (32‑256 byte size classes)
- **mimalloc**: High‑scalability backend (`--features alloc-mimalloc`)
- **Quarantine**: UAF mitigation with per‑thread ring buffer (`--features alloc-quarantine`)

Performance characteristics:
- **Default**: Good single‑thread performance, moderate scalability
- **mimalloc**: Excellent scalability for high‑concurrency workloads
- **Quarantine**: ~10‑20% overhead for UAF protection

Re-run after allocator changes to check for regressions.

## Hot Paths Using Direct Syscalls

- `read`, `write` — raw syscalls (no dlsym)
- `send`, `recv` — raw syscalls (sendto/recvfrom with NULL addr)
- `puts` — raw syscall (WRITE)

Cold paths (open, close, socket, etc.) use pluggable cache (`src/cache.rs`) with Relaxed load on fast path.
