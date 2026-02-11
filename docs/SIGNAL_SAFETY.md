# Async-Signal-Safety in IronLung

Functions that are safe to call from a signal handler must not:
- Acquire locks (malloc, mutex, etc.)
- Call non-async-signal-safe functions
- Use dynamic allocation
- Depend on errno (it can be overwritten)

## IronLung Function Audit

| Function | Async-Signal-Safe | Notes |
|----------|-------------------|-------|
| `malloc` | **NO** | Takes allocator lock |
| `free` | **NO** | Takes allocator lock |
| `realloc` | **NO** | Takes allocator lock |
| `calloc` | **NO** | Takes allocator lock |
| `memcpy` | **YES** | Pure copy, no locks, no allocation |
| `memmove` | **YES** | Pure copy |
| `strcpy` | **PARTIAL** | No locks, but scans unbounded string; use with caution if src length is bounded |
| `puts` | **NO** | Uses syscall (write) - generally safe, but may interleave with other output; avoid for predictability |
| `pthread_*` | **NO** | All take locks or delegate to libc |

## Guaranteed async-signal-safe set (contract)

IronLung guarantees the following are safe to call from a signal handler when used as specified:

| Function | Contract |
|----------|----------|
| `memcpy(dst, src, n)` | `dst`, `src` valid; `n` bounded; no overlap |
| `memmove(dst, src, n)` | `dst`, `src` valid; `n` bounded |

All other IronLung-intercepted functions (including `strcpy`, `puts`, `malloc`, `free`, `pthread_*`) are **not** async-signal-safe. Do not call them from a signal handler.

## Safe Subset for Signal Handlers

From IronLung, only `memcpy` and `memmove` are **guaranteed async-signal-safe** when given valid, bounded pointers and sizes. Do not call `malloc`, `free`, `strcpy`, or `puts` from a signal handler.

## Recommendation

In signal handlers, use only:
- `memcpy` / `memmove` (with validated, pre-allocated buffers)
- Raw `write` syscall to fd 2 for logging (if needed)
- `_exit` to terminate

Avoid any IronLung function that touches the allocator or pthread layer.

## Test

`tests/signal_safe_test.c` exercises the guaranteed subset: a signal handler calls only `memcpy` then `_exit`. Build and run with LD_PRELOAD to verify under IronLung:

```bash
gcc -o signal_safe_test tests/signal_safe_test.c
LD_PRELOAD=./target/release/libironlung.so ./signal_safe_test
```

Exit code 0 means the handler ran and exited safely.
