# IronLung Architecture

## Delegation vs. Reimplementation Rule

IronLung follows a clear rule for each intercepted function:

### Kernel-delegating (validate then syscall)

No libc in the hot path. Validate inputs/sizes, then perform a raw syscall.

| Function | Notes |
|----------|--------|
| `read`, `write` | Direct syscalls; EINTR retry; errno set from kernel return |
| `send`, `recv` | Wrapped as sendto/recvfrom with null address |
| `puts` | Validates string, single `write(2)` to fd 1 with `[IronLung]` prefix |

### Validate-then-delegate to libc

Validate inputs/sizes, then call the real implementation via `dlsym(RTLD_NEXT, ...)`.

| Function | Notes |
|----------|--------|
| `fopen`, `fclose`, `fread`, `fwrite`, `fgets` | Size caps; null checks |
| `pthread_*` | Full delegation (mutex, cond, create, join) |
| `open`, `close`, `fork`, `execve` | Delegate after validation |
| `socket`, `bind`, `listen`, `accept`, `connect` | Delegate |
| `iconv_open`, `iconv`, `iconv_close` | Delegate |
| `getpwnam`, `crypt` | Delegate with length limits |
| `printf`, `vprintf`, `fprintf` | C shim: validate format (reject %n, length limit), delegate to libc |

### Contained (sandbox)

Out-of-process with seccomp; no direct libc call in process.

| Function | Notes |
|----------|--------|
| `getaddrinfo`, `freeaddrinfo` | With `sandbox` feature: request/response via SHM, helper process with seccomp; without feature: delegate to libc |

### Memory and strings (no delegation)

Implemented in Rust; no kernel syscall for the operation itself (allocator uses mmap on OOM).

| Function | Notes |
|----------|--------|
| `malloc`, `free`, `realloc`, `calloc` | Talc allocator + spin mutex; mmap OOM |
| `memcpy`, `memmove` | Bounds and overlap checks |
| `strcpy` | Look-ahead safe copy with heuristic limit |

---

## Future Work

- **Kernel-delegating printf:** When feature `stdio-kernel` is on, `vprintf` is implemented in Rust (subset: `%%`, `%s`; others output `?`) and writes via `write(1, buf, len)`. See [docs/STDIO_KERNEL.md](STDIO_KERNEL.md). Full specifier set (e.g. `%d`, `%x`, `%p`) can be added later.
- **Per-thread allocator cache:** Reduce contention; fast path from thread-local free-list, slow path from global talc.

---

## errno

Error paths set `errno` so callers see glibc-compatible values. Syscall paths set it from the kernel return; delegated paths leave it to libc. See [docs/ERRNO.md](ERRNO.md) for the contract and tests.

## ABI and Types

ABI compatibility relies on the `libc` crate and system headers at build time. Critical types (e.g. `FILE*`, `pthread_mutex_t`, `addrinfo`) come from the `libc` crate. Layout changes across distros would require a bindgen pipeline or vendored headers; not currently done. To detect layout drift, add a compile-time or runtime assertion (e.g. `core::mem::size_of::<libc::pthread_mutex_t>() == expected`) for the default target (x86_64-unknown-linux-gnu) if desired.
