# errno Handling in IronLung

IronLung sets `errno` on error paths so that callers see the same values as with glibc where possible.

## Contract

- **Syscall paths** (`read`, `write`, `send`, `recv`): On negative return, `errno` is set from the kernel return value (we negate the negative errno and write it via `__errno_location`).
- **Delegated paths** (e.g. `open`, `fopen`, `pthread_*`): The real libc is called; it sets `errno`. We do not overwrite it.
- **Sandbox paths** (e.g. `getaddrinfo`): The sandbox response can include an errno; we set it when returning an error.

## Testing

- `tests/errno_test.c`: Triggers a failing `read(-1, buf, 1)` (expect `EBADF`).
- `scripts/run_errno_test.sh`: Runs the same program with and without LD_PRELOAD and compares exit code (errno). Pass if they match.

Run:

```bash
cargo build --release
sh scripts/run_errno_test.sh
```

## Known differences

None currently documented. If you find an error path where IronLung sets a different errno than glibc, please document it here and consider fixing.
