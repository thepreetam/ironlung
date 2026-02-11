# Allocator Quarantine / Shadow Memory (Design)

This document describes a **design** for optional allocator hardening: quarantine and shadow memory. Scope: use-after-free mitigation and optional bounds metadata.

## Release scope

- **v1.0:** Memory safety (allocator internal consistency) **and** UAF mitigation via **quarantine on by default** (feature `alloc-quarantine`). Build with `default-features = false` and without `alloc-quarantine` for memory safety only. Shadow memory remains optional future work.

## Goals

- **Quarantine:** Delay reuse of freed blocks so that dangling pointers are less likely to hit reallocated memory; optionally randomize or FIFO.
- **Shadow memory:** Optional per-allocation metadata (size, tag) in a separate shadow region to detect overflows or double-free without changing pointer representation.

## Constraints

- Must remain **no_std** and avoid pulling in libc for the allocator core (same as [ALLOCATOR.md](ALLOCATOR.md)).
- Locking must stay with `spin::Mutex` (or equivalent) to avoid pthread in the allocator.
- Performance and memory overhead must be acceptable; likely behind a feature flag.

## Quarantine

- On `free(ptr)`: move the block to a quarantine list (or pool) instead of immediately merging back into the talc heap.
- Quarantine limit: cap total quarantined size (e.g. 2× heap size) or count; when over limit, release oldest (FIFO) or random entries back to the allocator.
- On `malloc`: if no free block in the main heap, consider draining from quarantine (oldest first) and reusing.
- Trade-off: reduces use-after-free “reuse” at the cost of higher peak RSS and latency.

## Shadow Memory

- Separate contiguous region (e.g. mmap) used as a shadow table.
- Key by allocation address (e.g. page index or (ptr >> align_bits) → size, state).
- On alloc: record size (and optionally id) in shadow; on free: clear or mark freed; on alloc reuse: check consistency.
- Enables checks: overflow (past size), double-free (already freed), invalid free (not in table).
- Overhead: extra mmap, pointer-index computation, and lock for shadow updates.

## Implementation status

**Quarantine** is implemented when the feature `alloc-quarantine` is enabled (**default on** in v1.0; disable with `default-features = false` and omit `alloc-quarantine`). Behaviour: FIFO queue, cap by count (256 entries) and total bytes (512 KiB); `free` pushes into quarantine (draining oldest when over cap); `malloc` on OOM drains from quarantine and retries. No ABI or API changes; quarantine is internal to the allocator. See [ALLOCATOR.md](ALLOCATOR.md) for the concrete flow.

Shadow memory is **not** implemented; it remains optional future work.

## References

- [ALLOCATOR.md](ALLOCATOR.md) — current allocator design, thread safety, no libc in allocator.
- README “Gaps and limitations” — points here for quarantine/shadow.
