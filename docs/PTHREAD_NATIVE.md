# Native threading (clone3 / futex) - DEPRECATED

**This feature has been removed in favor of libc delegation.** The custom clone3/futex implementation added complexity and maintenance burden without significant security benefits.

When the `pthread-native` feature was enabled, IronLung implemented `pthread_create`, `pthread_join`, `pthread_mutex_*`, and `pthread_cond_*` using Linux syscalls and atomics only (no delegation to libc).

## Goal

Allow multithreaded applications to run without depending on glibc’s pthread implementation. Thread creation and synchronization use clone3 and futex only, so LD_PRELOAD of IronLung does not require glibc for threading.

## ABI

- **pthread_t:** We use the kernel TID (thread ID) as the value stored in `pthread_t`. Layout is compatible with a single word (pointer or integer); application code that only passes pthread_t to pthread_join is fine.
- **pthread_create / pthread_join:** Implemented in-tree when `pthread-native` is enabled. No new exported symbols; same ABI as the delegate path.
- **pthread_mutex_* / pthread_cond_*:** With `pthread-native`, also implemented in-tree via futex; first 4 bytes of `pthread_mutex_t` / `pthread_cond_t` hold the futex word (layout and size unchanged).

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

### Mutex

- **State:** First 4 bytes of `pthread_mutex_t`: 0 = unlocked, 1 = locked.
- **init/destroy:** Zero the futex word; `attr` ignored (normal mutex only).
- **lock:** Compare-exchange 0→1; on failure, futex_wait then retry.
- **unlock:** Store 0, futex_wake(1).
- Only normal (non-recursive) mutex is supported in the native path.

### Cond

- **State:** First 4 bytes of `pthread_cond_t`: generation counter.
- **init/destroy:** Zero the counter; `attr` ignored.
- **wait:** Read counter, unlock mutex, futex_wait(cond, counter), re-lock mutex.
- **signal:** Increment counter, futex_wake(1).
- `pthread_cond_broadcast` is not in the baseline; not implemented natively in this phase.

## Feature flag

- **Default:** Off. Threading delegates to libc.
- **`pthread-native`:** On Linux x86_64, create/join/mutex/cond via clone3+futex (no libc).

## Constraints

- **no_std:** No libc in the native path; use `sc` or raw syscall and our allocator for stacks.
- **TLS:** If the application or libc expects TLS (e.g. errno) in the new thread, we may need to set up a minimal TLS area via clone_args.tls. Current design assumes the thread only calls user code and our allocator; more TLS may be added later.
- **Stack size:** Default stack size follows a constant (e.g. 2 MiB) or an attribute if we parse pthread_attr_t; minimal implementation uses a fixed size.

## References

- Linux `clone3(2)` and `futex(2)`.
- [ARCHITECTURE.md](ARCHITECTURE.md) — overall delegation rules.
- [README](README.md) — feature list and gaps.
