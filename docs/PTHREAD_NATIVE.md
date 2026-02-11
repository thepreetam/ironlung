# Native threading (clone3 / futex)

When the `pthread-native` feature is enabled, IronLung implements `pthread_create` and `pthread_join` using Linux syscalls only (no delegation to libc). Mutex and condition variables remain delegated to system libc via dlsym.

## Goal

Allow multithreaded applications to create and join threads without depending on glibc’s pthread implementation. This “cuts the umbilical” for thread creation so that LD_PRELOAD of IronLung does not require glibc for basic threading.

## ABI

- **pthread_t:** We use the kernel TID (thread ID) as the value stored in `pthread_t`. Layout is compatible with a single word (pointer or integer); application code that only passes pthread_t to pthread_join is fine.
- **pthread_create / pthread_join:** Implemented in-tree when `pthread-native` is enabled. No new exported symbols; same ABI as the delegate path.
- **pthread_mutex_* / pthread_cond_*:** Still delegated to libc. A future phase could implement these with futex for full independence.

## Design

### pthread_create

1. Allocate a stack for the new thread (mmap or reuse a pool).
2. Prepare `clone_args`: flags `CLONE_VM | CLONE_FS | CLONE_FILES | CLONE_SIGHAND | CLONE_THREAD | CLONE_SYSVSEM | CLONE_PARENT_SETTID | CLONE_CHILD_CLEARTID`, stack pointer and size, `parent_tid` and `child_tid` pointing to a word we own (for join), `exit_signal` = 0 (thread).
3. Call `clone3` syscall. Child starts at a small trampoline that calls the user `start_routine(arg)` and then exits (kernel clears `child_tid` and wakes waiters).
4. Store the new TID in `*thread` (pthread_t).
5. Return 0 on success.

### pthread_join

1. Look up the thread’s exit state (the word we passed as `child_tid`). When the thread exits, the kernel sets it to 0 and wakes a futex.
2. Loop: if already 0, collect return value (if any) and return 0. Otherwise `futex_wait` on that address until woken (or timeout).
3. Clean up stack and any bookkeeping.

### Mutex / cond

Not implemented natively in this phase. They remain resolved via `cache::resolve("pthread_mutex_lock", …)` etc., so application code that uses mutex/cond still goes through libc for those calls.

## Feature flag

- **Default:** Off. Threading delegates to libc.
- **`pthread-native`:** On Linux, use clone3 + futex for create/join; mutex/cond still delegated.

## Constraints

- **no_std:** No libc in the native path; use `sc` or raw syscall and our allocator for stacks.
- **TLS:** If the application or libc expects TLS (e.g. errno) in the new thread, we may need to set up a minimal TLS area via clone_args.tls. Current design assumes the thread only calls user code and our allocator; more TLS may be added later.
- **Stack size:** Default stack size follows a constant (e.g. 2 MiB) or an attribute if we parse pthread_attr_t; minimal implementation uses a fixed size.

## References

- Linux `clone3(2)` and `futex(2)`.
- [ARCHITECTURE.md](ARCHITECTURE.md) — overall delegation rules.
- [README](README.md) — feature list and gaps.
