# IronLung Allocator Design

## Thread Safety

The allocator is **thread-safe** via `Talck<spin::Mutex<()>, MmapOom>`:

- All `malloc`, `free`, `realloc`, `calloc` calls acquire a global mutex before touching the talc heap.
- The `spin::Mutex` provides mutual exclusion; no two threads allocate/deallocate concurrently.
- OOM handling (mmap via syscall) runs under the same lock.

## Contention

Under high concurrency, lock contention may limit throughput.

### Per-thread cache (feature `alloc-cache`)

When the `alloc-cache` feature is enabled, a small per-thread free-list is used for a single size class (64 bytes). Fast path: serve from thread-local cache without the global lock. Slow path: acquire global lock, alloc from talc. Free: push to cache if size matches and cache not full; else global dealloc. malloc/free remain **not** async-signal-safe. Benchmark: build with `cargo build --release --features alloc-cache` and run `tests/alloc_bench.c` with multiple threads; compare to build without the feature. See [BENCHMARKS.md](BENCHMARKS.md) for baseline.

### Other options

- **Alternative allocators**: mimalloc/snmalloc patterns for higher scalability (evaluate if needed).

## Current Architecture

```
malloc(n) → ensure_init() → GLOBAL.lock() → talc.alloc(layout) → return ptr+header
free(p)   → GLOBAL.lock() → talc.dealloc(header_ptr, layout)
```

Bootstrap heap: 64 pages from static buffer. OOM: mmap 16 pages, claim into talc.

## Async-Signal-Safety

**malloc/free/realloc/calloc are NOT async-signal-safe.** They take locks. Do not call from signal handlers. See [SIGNAL_SAFETY.md](SIGNAL_SAFETY.md).
