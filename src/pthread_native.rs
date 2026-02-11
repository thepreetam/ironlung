//! Native pthread_create / pthread_join and pthread_mutex_* / pthread_cond_* via clone3 and futex (Linux).
//! Used when the `pthread-native` feature is enabled. No libc for threading.

use core::ffi::c_void;
use core::sync::atomic::{AtomicPtr, AtomicU32, Ordering};

use crate::errno;

// Linux x86_64 syscall numbers
const SYS_clone3: usize = 435;
const SYS_futex: usize = 202;
const SYS_mmap: usize = 9;
const SYS_munmap: usize = 11;

const FUTEX_WAIT: u32 = 0;
const FUTEX_WAKE: u32 = 1;
const FUTEX_PRIVATE: u32 = 128;

const CLONE_VM: u64 = 0x0000_0100;
const CLONE_FS: u64 = 0x0000_0200;
const CLONE_FILES: u64 = 0x0000_0400;
const CLONE_SIGHAND: u64 = 0x0000_0800;
const CLONE_THREAD: u64 = 0x0001_0000;
const CLONE_SYSVSEM: u64 = 0x0004_0000;
const CLONE_PARENT_SETTID: u64 = 0x0010_0000;

const STACK_SIZE: usize = 2 * 1024 * 1024; // 2 MiB
const MAP_ANONYMOUS: i32 = 0x20;
const MAP_PRIVATE: i32 = 0x02;
const PROT_READ: i32 = 0x1;
const PROT_WRITE: i32 = 0x2;

#[repr(C)]
struct CloneArgs {
    flags: u64,
    pidfd: u64,
    child_tid: u64,
    parent_tid: u64,
    exit_signal: u64,
    stack: u64,
    stack_size: u64,
    tls: u64,
    set_tid: u64,
    set_tid_size: u64,
}

const MAX_THREADS: usize = 64;

struct Ctrl {
    tid: u32,
    status: AtomicU32, // 1 = running, 0 = done
    retval: AtomicPtr<c_void>,
    stack: *mut u8,
    stack_size: usize,
}
// Safe: Ctrl is only accessed under THREAD_TABLE lock; stack is not shared across threads.
unsafe impl Send for Ctrl {}

static THREAD_TABLE: spin::Mutex<[Option<Ctrl>; MAX_THREADS]> = spin::Mutex::new([const { None }; MAX_THREADS]);

#[cfg(target_arch = "x86_64")]
unsafe fn raw_syscall1(nr: usize, a1: usize) -> isize {
    let ret: isize;
    core::arch::asm!(
        "syscall",
        in("rax") nr,
        in("rdi") a1,
        out("rcx") _, out("r11") _,
        lateout("rax") ret,
        options(nostack, preserves_flags)
    );
    ret
}

#[cfg(target_arch = "x86_64")]
unsafe fn raw_syscall6(nr: usize, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize, a6: usize) -> isize {
    let ret: isize;
    core::arch::asm!(
        "syscall",
        in("rax") nr,
        in("rdi") a1, in("rsi") a2, in("rdx") a3, in("r10") a4, in("r8") a5, in("r9") a6,
        out("rcx") _, out("r11") _,
        lateout("rax") ret,
        options(nostack, preserves_flags)
    );
    ret
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn raw_syscall1(_nr: usize, _a1: usize) -> isize {
    -1
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn raw_syscall6(_nr: usize, _a1: usize, _a2: usize, _a3: usize, _a4: usize, _a5: usize, _a6: usize) -> isize {
    -1
}

unsafe fn mmap_stack(size: usize) -> *mut u8 {
    let ret = raw_syscall6(
        SYS_mmap,
        0,
        size,
        (PROT_READ | PROT_WRITE) as usize,
        (MAP_PRIVATE | MAP_ANONYMOUS) as usize,
        usize::MAX,
        0,
    );
    if ret < 0 {
        return core::ptr::null_mut();
    }
    ret as *mut u8
}

unsafe fn munmap_stack(ptr: *mut u8, size: usize) {
    if ptr.is_null() {
        return;
    }
    let _ = raw_syscall6(SYS_munmap, ptr as usize, size, 0, 0, 0, 0);
}

unsafe fn futex_wait(addr: *const u32, val: u32) {
    let _ = raw_syscall6(
        SYS_futex,
        addr as usize,
        (FUTEX_WAIT | FUTEX_PRIVATE) as usize,
        val as usize,
        0,
        0,
        0,
    );
}

unsafe fn futex_wake(addr: *const u32, count: u32) {
    let _ = raw_syscall6(
        SYS_futex,
        addr as usize,
        (FUTEX_WAKE | FUTEX_PRIVATE) as usize,
        count as usize,
        0,
        0,
        0,
    );
}

/// First 4 bytes of pthread_mutex_t / pthread_cond_t are our futex word (layout-compatible with libc size).
fn mutex_state(mutex: *mut libc::pthread_mutex_t) -> *mut AtomicU32 {
    mutex.cast()
}
fn cond_state(cond: *mut libc::pthread_cond_t) -> *mut AtomicU32 {
    cond.cast()
}

// --- Native mutex (normal only; attr ignored) ---

#[cfg(target_arch = "x86_64")]
pub unsafe fn pthread_mutex_init_native(
    mutex: *mut libc::pthread_mutex_t,
    _attr: *const libc::pthread_mutexattr_t,
) -> libc::c_int {
    if mutex.is_null() {
        return libc::EINVAL;
    }
    (*mutex_state(mutex)).store(0, Ordering::Release);
    0
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn pthread_mutex_lock_native(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    if mutex.is_null() {
        return libc::EINVAL;
    }
    let s = mutex_state(mutex);
    loop {
        if (*s).compare_exchange_weak(0, 1, Ordering::Acquire, Ordering::Relaxed).is_ok() {
            return 0;
        }
        futex_wait(s as *const u32, 1);
    }
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn pthread_mutex_unlock_native(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    if mutex.is_null() {
        return libc::EINVAL;
    }
    let s = mutex_state(mutex);
    (*s).store(0, Ordering::Release);
    futex_wake(s as *const u32, 1);
    0
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn pthread_mutex_destroy_native(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    if mutex.is_null() {
        return libc::EINVAL;
    }
    (*mutex_state(mutex)).store(0, Ordering::Release);
    0
}

#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn pthread_mutex_init_native(
    _mutex: *mut libc::pthread_mutex_t,
    _attr: *const libc::pthread_mutexattr_t,
) -> libc::c_int {
    libc::ENOSYS
}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn pthread_mutex_lock_native(_mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    libc::ENOSYS
}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn pthread_mutex_unlock_native(_mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    libc::ENOSYS
}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn pthread_mutex_destroy_native(_mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    libc::ENOSYS
}

// --- Native cond (attr ignored) ---

#[cfg(target_arch = "x86_64")]
pub unsafe fn pthread_cond_init_native(
    cond: *mut libc::pthread_cond_t,
    _attr: *const libc::pthread_condattr_t,
) -> libc::c_int {
    if cond.is_null() {
        return libc::EINVAL;
    }
    (*cond_state(cond)).store(0, Ordering::Release);
    0
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn pthread_cond_wait_native(
    cond: *mut libc::pthread_cond_t,
    mutex: *mut libc::pthread_mutex_t,
) -> libc::c_int {
    if cond.is_null() || mutex.is_null() {
        return libc::EINVAL;
    }
    let c = cond_state(cond);
    let gen = (*c).load(Ordering::Acquire);
    pthread_mutex_unlock_native(mutex);
    futex_wait(c as *const u32, gen);
    pthread_mutex_lock_native(mutex);
    0
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn pthread_cond_signal_native(cond: *mut libc::pthread_cond_t) -> libc::c_int {
    if cond.is_null() {
        return libc::EINVAL;
    }
    let c = cond_state(cond);
    (*c).fetch_add(1, Ordering::Release);
    futex_wake(c as *const u32, 1);
    0
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn pthread_cond_destroy_native(cond: *mut libc::pthread_cond_t) -> libc::c_int {
    if cond.is_null() {
        return libc::EINVAL;
    }
    (*cond_state(cond)).store(0, Ordering::Release);
    0
}

#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn pthread_cond_init_native(
    _cond: *mut libc::pthread_cond_t,
    _attr: *const libc::pthread_condattr_t,
) -> libc::c_int {
    libc::ENOSYS
}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn pthread_cond_wait_native(
    _cond: *mut libc::pthread_cond_t,
    _mutex: *mut libc::pthread_mutex_t,
) -> libc::c_int {
    libc::ENOSYS
}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn pthread_cond_signal_native(_cond: *mut libc::pthread_cond_t) -> libc::c_int {
    libc::ENOSYS
}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn pthread_cond_destroy_native(_cond: *mut libc::pthread_cond_t) -> libc::c_int {
    libc::ENOSYS
}

/// Child trampoline: run start_routine(arg), store result, mark done, wake joiner, then sleep forever.
unsafe fn trampoline(
    ctrl: *mut Ctrl,
    start_routine: Option<extern "C" fn(*mut c_void) -> *mut c_void>,
    arg: *mut c_void,
) {
    let ret = start_routine.map(|f| f(arg)).unwrap_or(core::ptr::null_mut());
    (*ctrl).retval.store(ret, Ordering::Release);
    (*ctrl).status.store(0, Ordering::Release);
    futex_wake(&(*ctrl).status as *const AtomicU32 as *const u32, 1);
    loop {
        futex_wait(&(*ctrl).status as *const AtomicU32 as *const u32, 0);
    }
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn pthread_create_native(
    thread: *mut libc::pthread_t,
    _attr: *const libc::pthread_attr_t,
    start_routine: Option<extern "C" fn(*mut c_void) -> *mut c_void>,
    arg: *mut c_void,
) -> libc::c_int {
    if thread.is_null() || start_routine.is_none() {
        return libc::EINVAL;
    }
    let stack = mmap_stack(STACK_SIZE);
    if stack.is_null() {
        return libc::ENOMEM;
    }
    let mut table = THREAD_TABLE.lock();
    let slot = table.iter_mut().find(|s| s.is_none());
    let Some(slot) = slot else {
        munmap_stack(stack, STACK_SIZE);
        return libc::EAGAIN;
    };
    let ctrl = Ctrl {
        tid: 0,
        status: AtomicU32::new(1),
        retval: AtomicPtr::new(core::ptr::null_mut()),
        stack,
        stack_size: STACK_SIZE,
    };
    *slot = Some(ctrl);
    let ctrl_ref = slot.as_ref().unwrap() as *const Ctrl as *mut Ctrl;
    drop(table);

    let stack_top = stack.add(STACK_SIZE);
    let args = CloneArgs {
        flags: CLONE_VM | CLONE_FS | CLONE_FILES | CLONE_SIGHAND | CLONE_THREAD | CLONE_SYSVSEM | CLONE_PARENT_SETTID,
        pidfd: 0,
        child_tid: 0,
        parent_tid: &(*ctrl_ref).tid as *const u32 as u64,
        exit_signal: 0,
        stack: stack_top as u64,
        stack_size: STACK_SIZE as u64,
        tls: 0,
        set_tid: 0,
        set_tid_size: 0,
    };
    let ret = raw_syscall1(SYS_clone3, &args as *const CloneArgs as usize);
    if ret == 0 {
        trampoline(ctrl_ref, start_routine, arg);
    }
    if ret < 0 {
        errno::set_errno_from_syscall(ret);
        let mut table = THREAD_TABLE.lock();
        for slot in table.iter_mut() {
            if slot.as_ref().map(|c| c as *const Ctrl == ctrl_ref).unwrap_or(false) {
                if let Some(c) = slot.take() {
                    munmap_stack(c.stack, c.stack_size);
                }
                break;
            }
        }
        return -1;
    }
    *thread = (*ctrl_ref).tid as libc::pthread_t;
    0
}

#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn pthread_create_native(
    _thread: *mut libc::pthread_t,
    _attr: *const libc::pthread_attr_t,
    _start_routine: Option<extern "C" fn(*mut c_void) -> *mut c_void>,
    _arg: *mut c_void,
) -> libc::c_int {
    libc::ENOSYS
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn pthread_join_native(thread: libc::pthread_t, retval: *mut *mut c_void) -> libc::c_int {
    let tid = thread as u32;
    let ctrl_ptr = {
        let table = THREAD_TABLE.lock();
        let ctrl_ref = table.iter().find(|s| s.as_ref().map(|c| c.tid == tid).unwrap_or(false));
        match ctrl_ref {
            Some(Some(c)) => c as *const Ctrl as *mut Ctrl,
            _ => return libc::ESRCH,
        }
    };
    while (*ctrl_ptr).status.load(Ordering::Acquire) != 0 {
        futex_wait(&(*ctrl_ptr).status as *const AtomicU32 as *const u32, 1);
    }
    if !retval.is_null() {
        *retval = (*ctrl_ptr).retval.load(Ordering::Acquire);
    }
    let mut table = THREAD_TABLE.lock();
    if let Some(slot) = table.iter_mut().find(|s| s.as_ref().map(|c| c.tid == tid).unwrap_or(false)) {
        if let Some(c) = slot.take() {
            munmap_stack(c.stack, c.stack_size);
        }
    }
    0
}

#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn pthread_join_native(_thread: libc::pthread_t, _retval: *mut *mut c_void) -> libc::c_int {
    libc::ENOSYS
}
