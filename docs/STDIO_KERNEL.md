# Kernel-delegating printf (feature `stdio-kernel`)

## Design

Two-pass, no libc in the hot path:

1. **Parse format string:** Reject `%n` (format-string attack). Limit format length (e.g. 4096). Compute upper bound of output length or use a fixed output buffer.
2. **Format and write:** Use a fixed stack buffer (e.g. 4 KiB). Format supported specifiers into the buffer. Single `write(1, buf, len)` (or `write(2, ...)` for stderr in fprintf).

Must be `no_std`-friendly: no libc `FILE*`, no `vprintf`.

## Supported specifiers (initial subset)

- `%%` — literal `%`
- `%s` — string (null-terminated)
- `%d`, `%i` — signed int
- `%u` — unsigned int
- `%x`, `%X` — hex
- `%p` — pointer

Width/precision (e.g. `%10s`) can be added later. Unsupported specifiers can be delegated to libc or output as `?`.

## Integration

When feature `stdio-kernel` is enabled and `stdio-libc` is not set, `vprintf` is implemented in Rust and delegates to the kernel (`write` syscall). `printf` remains a small C wrapper that does `va_start`; `vprintf(fmt, ap)`; `va_end`. `fprintf` continues to validate and delegate to libc (or a future kernel path for fd 1/2). **By default** the crate enables `stdio-kernel` and `stdio-kernel-fread-fwrite`; use the `stdio-libc` feature to force the libc delegate path.

## Buffer limits

Output is capped at 4096 bytes per call to avoid stack overflow and unbounded allocation. Longer output is truncated (or split across multiple writes in a future revision).

---

## Kernel-delegating fread/fwrite (feature `stdio-kernel-fread-fwrite`)

When this feature is enabled (and `target_os = "linux"`), `fread` and `fwrite` obtain the file descriptor from `FILE*` via delegated `fileno(stream)` and then perform `read`/`write` syscalls with the same size validation as the delegate path. Buffering is not reimplemented in IronLung; we only validate size/nmemb, get fd, and loop read/write with EINTR handling. If `fileno` fails (e.g. invalid stream), the implementation falls back to delegated fread/fwrite. This aligns with macro Phase 2: "delegate fread/fwrite directly to read/write syscalls."
