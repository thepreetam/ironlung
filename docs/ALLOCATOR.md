# IronLung Allocator Design

## Thread Safety

The allocator is **thread-safe** via `Talck<spin::Mutex<()>, MmapOom>`:

- All `malloc`, `free`, `realloc`, `calloc` calls acquire a global mutex before touching the talc heap.
- The `spin::Mutex` provides mutual exclusion; no two threads allocate/deallocate concurrently.
- OOM handling (mmap via syscall) runs under the same lock.

## Contention

Under high concurrency, lock contention may limit throughput. Future improvements:

- **Per-thread cache**: Each thread maintains a small free-list; fast path serves from cache; slow path acquires global lock and refills.
- **Alternative allocators**: mimalloc/snmalloc patterns for higher scalability (evaluate if needed).

## Current Architecture

```
malloc(n) → ensure_init() → GLOBAL.lock() → talc.alloc(layout) → return ptr+header
free(p)   → GLOBAL.lock() → talc.dealloc(header_ptr, layout)
```

Bootstrap heap: 64 pages from static buffer. OOM: mmap 16 pages, claim into talc.

## Async-Signal-Safety

**malloc/free/realloc/calloc are NOT async-signal-safe.** They take locks. Do not call from signal handlers. See [SIGNAL_SAFETY.md](SIGNAL_SAFETY.md).
