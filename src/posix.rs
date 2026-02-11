//! POSIX file and process: open, read, write, close, fork, exec.
//! read/write use direct syscalls; others delegate via dlsym.

use core::ffi::c_void;
use core::sync::atomic::AtomicPtr;

use sc::nr::{READ, WRITE};
use sc::syscall;

use crate::cache;
use crate::errno;

type OpenFn = unsafe extern "C" fn(*const libc::c_char, libc::c_int, libc::c_int) -> libc::c_int;
type CloseFn = unsafe extern "C" fn(libc::c_int) -> libc::c_int;
type ForkFn = unsafe extern "C" fn() -> libc::pid_t;
type ExecveFn = unsafe extern "C" fn(*const libc::c_char, *const *const libc::c_char, *const *const libc::c_char) -> libc::c_int;

static OPEN: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static CLOSE: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static FORK: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static EXECVE: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());

const MAX_READ_WRITE: libc::size_t = 1_000_000_000;
const EINTR: libc::c_int = 4;

#[no_mangle]
pub unsafe extern "C" fn open(path: *const libc::c_char, oflag: libc::c_int, mode: libc::c_int) -> libc::c_int {
    if path.is_null() {
        return -1;
    }
    let f = cache::resolve(b"open\0", &OPEN);
    if f.is_null() {
        return -1;
    }
    let f: OpenFn = core::mem::transmute(f);
    f(path, oflag, mode)
}

#[no_mangle]
pub unsafe extern "C" fn read(fd: libc::c_int, buf: *mut c_void, count: libc::size_t) -> libc::ssize_t {
    if buf.is_null() || count > MAX_READ_WRITE {
        return -1;
    }
    loop {
        let ret = syscall!(READ, fd as usize, buf as usize, count) as isize;
        if ret >= 0 {
            return ret as libc::ssize_t;
        }
        if ret != -(EINTR as isize) {
            errno::set_errno_from_syscall(ret);
            return -1;
        }
        // EINTR: retry once
    }
}

#[no_mangle]
pub unsafe extern "C" fn write(fd: libc::c_int, buf: *const c_void, count: libc::size_t) -> libc::ssize_t {
    if buf.is_null() || count > MAX_READ_WRITE {
        return -1;
    }
    loop {
        let ret = syscall!(WRITE, fd as usize, buf as usize, count) as isize;
        if ret >= 0 {
            return ret as libc::ssize_t;
        }
        if ret != -(EINTR as isize) {
            errno::set_errno_from_syscall(ret);
            return -1;
        }
        // EINTR: retry once
    }
}

#[no_mangle]
pub unsafe extern "C" fn close(fd: libc::c_int) -> libc::c_int {
    let f = cache::resolve(b"close\0", &CLOSE);
    if f.is_null() {
        return -1;
    }
    let f: CloseFn = core::mem::transmute(f);
    f(fd)
}

#[no_mangle]
pub unsafe extern "C" fn fork() -> libc::pid_t {
    let f = cache::resolve(b"fork\0", &FORK);
    if f.is_null() {
        return -1;
    }
    let f: ForkFn = core::mem::transmute(f);
    f()
}

#[no_mangle]
pub unsafe extern "C" fn execve(
    pathname: *const libc::c_char,
    argv: *const *const libc::c_char,
    envp: *const *const libc::c_char,
) -> libc::c_int {
    if pathname.is_null() {
        return -1;
    }
    let f = cache::resolve(b"execve\0", &EXECVE);
    if f.is_null() {
        return -1;
    }
    let f: ExecveFn = core::mem::transmute(f);
    f(pathname, argv, envp)
}
