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
| `pthread_*` | With feature `pthread-native` (Linux x86_64): create/join/mutex/cond via clone3+futex; no libc. Otherwise full delegation. |
| `open`, `close`, `fork`, `execve` | Delegate after validation |
| `socket`, `bind`, `listen`, `accept`, `connect` | Delegate |
| `iconv_open`, `iconv`, `iconv_close` | Delegate |
| `getpwnam`, `crypt` | Delegate with length limits |
| `printf`, `vprintf`, `fprintf`, `snprintf` | C shim: validate format (reject %n, length limit), delegate to libc |
| `memset`, `strcmp`, `strncpy` | Size/length limits, delegate to libc |
| `getenv` | Name length limit, delegate to libc |
| `wcslen`, `wcscpy`, `wcsncpy`, `wcscmp` | Minimal wchar; bounded, delegate to libc; full locale out of scope |

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

- **Kernel-delegating printf:** When feature `stdio-kernel` is on (default on Linux), `vprintf` is implemented in Rust (subset: `%%`, `%s`, `%d`, `%x`, `%p`, etc.) and writes via `write(1, buf, len)`. See [STDIO_KERNEL.md](STDIO_KERNEL.md). Full specifier set can be extended as needed.
- **Kernel-delegating fread/fwrite:** When feature `stdio-kernel-fread-fwrite` is on (default on Linux), `fread` and `fwrite` obtain the fd via delegated `fileno(stream)` and perform `read`/`write` syscalls with size validation and EINTR handling. See [STDIO_KERNEL.md](STDIO_KERNEL.md).
- **Per-thread allocator cache:** Reduce contention; fast path from thread-local free-list, slow path from global talc.
- **Allocator hardening (quarantine / UAF mitigation):** Planned for v0.2; see [ALLOCATOR_QUARANTINE.md](ALLOCATOR_QUARANTINE.md).

---

## errno

Error paths set `errno` so callers see glibc-compatible values. Syscall paths set it from the kernel return; delegated paths leave it to libc. See [docs/ERRNO.md](ERRNO.md) for the contract and tests.

## ABI and Types

ABI compatibility relies on the `libc` crate and system headers at build time. Critical types include `FILE*`, `pthread_mutex_t`, `pthread_cond_t`, `addrinfo`, and other structs used by delegated or kernel-delegating code. Layout changes across distros would require a bindgen pipeline or vendored headers; not currently done. A bindgen/header-scraping pipeline could be added later for stricter per-distro layout checks.

To detect layout drift on the default target (x86_64-unknown-linux-gnu), run the ABI check with the optional layout assertion: build `ironlung-abi-check` with the `layout-check` feature and set `IRONLUNG_LAYOUT_CHECK=1` when running it; it will assert `size_of::<libc::pthread_mutex_t>()` against a pinned value and exit with an error if the layout has drifted. See [docs/CI_ABI.md](CI_ABI.md) for the cdylib export contract.
