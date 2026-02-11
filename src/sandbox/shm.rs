//! Shared memory helpers for sandbox IPC. Uses memfd_create and mmap.

use core::arch::asm;
use core::ffi::c_void;

use sc::nr::MMAP;
use sc::syscall;

// memfd_create syscall numbers (Linux)
#[cfg(target_arch = "x86_64")]
const MEMFD_CREATE_NR: usize = 319;
#[cfg(target_arch = "aarch64")]
const MEMFD_CREATE_NR: usize = 385;

const MFD_CLOEXEC: i32 = 0x0001;
const PROT_READ: i32 = 0x1;
const PROT_WRITE: i32 = 0x2;
const MAP_SHARED: i32 = 0x01;

pub const SHM_SIZE: usize = 0x10000; // 64 KB

/// Create anonymous shared memory via memfd_create. Returns fd or -1.
#[cfg(target_os = "linux")]
pub unsafe fn memfd_create(name: &[u8], flags: i32) -> libc::c_int {
    let name_c = {
        let mut buf = [0i8; 32];
        let len = name.len().min(buf.len() - 1);
        for (i, &b) in name[..len].iter().enumerate() {
            buf[i] = b as i8;
        }
        buf[len] = 0;
        buf.as_ptr()
    };
    let ret: isize;
    #[cfg(target_arch = "x86_64")]
    {
        asm!(
            "syscall",
            in("rax") MEMFD_CREATE_NR,
            in("rdi") name_c,
            in("rsi") flags as usize,
            out("rcx") _,
            out("r11") _,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
    }
    #[cfg(target_arch = "aarch64")]
    {
        asm!(
            "svc 0",
            in("x8") MEMFD_CREATE_NR,
            in("x0") name_c,
            in("x1") flags as usize,
            lateout("x0") ret,
            options(nostack, preserves_flags)
        );
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        let _ = (name_c, flags);
        return -1;
    }
    if ret < 0 {
        -1
    } else {
        ret as libc::c_int
    }
}

/// Create and size memfd for sandbox IPC. Returns fd or -1.
#[cfg(target_os = "linux")]
pub fn shm_create() -> libc::c_int {
    let fd = unsafe { memfd_create(b"ironlung-getaddrinfo\0", MFD_CLOEXEC) };
    if fd < 0 {
        return -1;
    }
    if unsafe { libc::ftruncate(fd, SHM_SIZE as libc::off_t) } != 0 {
        unsafe { libc::close(fd) };
        return -1;
    }
    fd
}

/// Same as shm_create but without MFD_CLOEXEC so fd stays open across execve.
/// Child sandbox process inherits the fd.
#[cfg(target_os = "linux")]
pub fn shm_create_for_sandbox() -> libc::c_int {
    let fd = unsafe { memfd_create(b"ironlung-getaddrinfo\0", 0) };
    if fd < 0 {
        return -1;
    }
    if unsafe { libc::ftruncate(fd, SHM_SIZE as libc::off_t) } != 0 {
        unsafe { libc::close(fd) };
        return -1;
    }
    fd
}

/// Map memfd into process address space. Returns ptr or null.
#[cfg(target_os = "linux")]
pub unsafe fn shm_map(fd: libc::c_int) -> *mut c_void {
    let ret = syscall!(
        MMAP,
        0usize,
        SHM_SIZE,
        (PROT_READ | PROT_WRITE) as usize,
        MAP_SHARED as usize,
        fd as usize,
        0usize
    );
    if ret as isize <= 0 {
        core::ptr::null_mut()
    } else {
        ret as *mut c_void
    }
}
