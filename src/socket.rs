//! Basic sockets: socket, bind, listen, accept, connect, send, recv.
//! send/recv use direct syscalls; others delegate via dlsym.

use core::ffi::c_void;
use core::sync::atomic::AtomicPtr;

use sc::nr::{RECVFROM, SENDTO};
use sc::syscall;

use crate::cache;
use crate::errno;

type SocketFn = unsafe extern "C" fn(libc::c_int, libc::c_int, libc::c_int) -> libc::c_int;
type BindFn = unsafe extern "C" fn(libc::c_int, *const libc::sockaddr, libc::socklen_t) -> libc::c_int;
type ListenFn = unsafe extern "C" fn(libc::c_int, libc::c_int) -> libc::c_int;
type AcceptFn = unsafe extern "C" fn(libc::c_int, *mut libc::sockaddr, *mut libc::socklen_t) -> libc::c_int;
type ConnectFn = unsafe extern "C" fn(libc::c_int, *const libc::sockaddr, libc::socklen_t) -> libc::c_int;

static SOCKET: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static BIND: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static LISTEN: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static ACCEPT: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static CONNECT: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());

const MAX_MSG: libc::size_t = 1_000_000_000;
const EINTR: libc::c_int = 4;

#[no_mangle]
pub unsafe extern "C" fn socket(domain: libc::c_int, type_: libc::c_int, protocol: libc::c_int) -> libc::c_int {
    let f = cache::resolve(b"socket\0", &SOCKET);
    if f.is_null() {
        return -1;
    }
    let f: SocketFn = core::mem::transmute(f);
    f(domain, type_, protocol)
}

#[no_mangle]
pub unsafe extern "C" fn bind(
    sockfd: libc::c_int,
    addr: *const libc::sockaddr,
    addrlen: libc::socklen_t,
) -> libc::c_int {
    if addr.is_null() {
        return -1;
    }
    let f = cache::resolve(b"bind\0", &BIND);
    if f.is_null() {
        return -1;
    }
    let f: BindFn = core::mem::transmute(f);
    f(sockfd, addr, addrlen)
}

#[no_mangle]
pub unsafe extern "C" fn listen(sockfd: libc::c_int, backlog: libc::c_int) -> libc::c_int {
    let f = cache::resolve(b"listen\0", &LISTEN);
    if f.is_null() {
        return -1;
    }
    let f: ListenFn = core::mem::transmute(f);
    f(sockfd, backlog)
}

#[no_mangle]
pub unsafe extern "C" fn accept(
    sockfd: libc::c_int,
    addr: *mut libc::sockaddr,
    addrlen: *mut libc::socklen_t,
) -> libc::c_int {
    let f = cache::resolve(b"accept\0", &ACCEPT);
    if f.is_null() {
        return -1;
    }
    let f: AcceptFn = core::mem::transmute(f);
    f(sockfd, addr, addrlen)
}

#[no_mangle]
pub unsafe extern "C" fn connect(
    sockfd: libc::c_int,
    addr: *const libc::sockaddr,
    addrlen: libc::socklen_t,
) -> libc::c_int {
    if addr.is_null() {
        return -1;
    }
    let f = cache::resolve(b"connect\0", &CONNECT);
    if f.is_null() {
        return -1;
    }
    let f: ConnectFn = core::mem::transmute(f);
    f(sockfd, addr, addrlen)
}

#[no_mangle]
pub unsafe extern "C" fn send(sockfd: libc::c_int, buf: *const c_void, len: libc::size_t, flags: libc::c_int) -> libc::ssize_t {
    if buf.is_null() || len > MAX_MSG {
        return -1;
    }
    // sendto(fd, buf, len, flags, NULL, 0) is equivalent to send
    loop {
        let ret = syscall!(SENDTO, sockfd as usize, buf as usize, len, flags as usize, 0usize, 0usize) as isize;
        if ret >= 0 {
            return ret as libc::ssize_t;
        }
        if ret != -(EINTR as isize) {
            errno::set_errno_from_syscall(ret);
            return -1;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn recv(sockfd: libc::c_int, buf: *mut c_void, len: libc::size_t, flags: libc::c_int) -> libc::ssize_t {
    if buf.is_null() || len > MAX_MSG {
        return -1;
    }
    // recvfrom(fd, buf, len, flags, NULL, NULL)
    loop {
        let ret = syscall!(RECVFROM, sockfd as usize, buf as usize, len, flags as usize, 0usize, 0usize) as isize;
        if ret >= 0 {
            return ret as libc::ssize_t;
        }
        if ret != -(EINTR as isize) {
            errno::set_errno_from_syscall(ret);
            return -1;
        }
    }
}
