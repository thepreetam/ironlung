//! DNS and getaddrinfo: delegates to system libc via dlsym(RTLD_NEXT).
//! With sandbox feature, routes getaddrinfo through a contained helper process.

use core::ffi::c_void;
use core::sync::atomic::AtomicPtr;

use crate::cache;

#[cfg(all(target_os = "linux", feature = "sandbox"))]
use crate::errno;
#[cfg(all(target_os = "linux", feature = "sandbox"))]
use crate::sandbox::protocol;
#[cfg(all(target_os = "linux", feature = "sandbox"))]
use crate::sandbox::protocol::{GetAddrInfoRequest, GetAddrInfoResponse, RESPONSE_OFFSET};
#[cfg(all(target_os = "linux", feature = "sandbox"))]
use crate::sandbox::shm;

#[cfg(all(target_os = "linux", feature = "sandbox"))]
extern "C" {
    static environ: *mut *mut libc::c_char;
}

type GetaddrinfoFn = unsafe extern "C" fn(
    *const libc::c_char,
    *const libc::c_char,
    *const libc::addrinfo,
    *mut *mut libc::addrinfo,
) -> libc::c_int;
type FreeaddrinfoFn = unsafe extern "C" fn(*mut libc::addrinfo);

static GETADDRINFO: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static FREEADDRINFO: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());

const HOST_MAX: usize = 256;
#[cfg(feature = "sandbox")]
const SERV_MAX: usize = 256;
#[cfg(not(feature = "sandbox"))]
const SERV_MAX: usize = 32;

/// For Week 2: allow any hints (we copy scalar fields only; pointers ignored).
#[cfg(all(target_os = "linux", feature = "sandbox"))]
fn hints_ok(_hints: *const libc::addrinfo) -> bool {
    true
}

/// Returns true if env string starts with "LD_PRELOAD=".
#[cfg(all(target_os = "linux", feature = "sandbox"))]
unsafe fn is_ld_preload(s: *const libc::c_char) -> bool {
    const PREFIX: &[u8] = b"LD_PRELOAD=";
    let mut i = 0usize;
    while i < PREFIX.len() {
        let c = *s.add(i);
        if c == 0 {
            return false;
        }
        if c as u8 != PREFIX[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Build envp for execve without LD_PRELOAD. Sandbox must run without ironlung preloaded.
#[cfg(all(target_os = "linux", feature = "sandbox"))]
unsafe fn build_env_without_ld_preload() -> *const *const libc::c_char {
    let mut count = 0usize;
    let mut p = environ;
    while !p.is_null() && !(*p).is_null() {
        if !is_ld_preload(*p) {
            count += 1;
        }
        p = p.add(1);
    }

    extern "C" {
        fn malloc(size: libc::size_t) -> *mut core::ffi::c_void;
    }
    let size = (count + 1) * core::mem::size_of::<*const libc::c_char>();
    let arr = malloc(size) as *mut *const libc::c_char;
    if arr.is_null() {
        return environ as *const *const libc::c_char;
    }
    core::ptr::write_bytes(arr, 0, count + 1);

    let mut dst = 0usize;
    let mut src = environ;
    while !src.is_null() && !(*src).is_null() {
        if !is_ld_preload(*src) {
            *arr.add(dst) = *src;
            dst += 1;
        }
        src = src.add(1);
    }
    arr as *const *const libc::c_char
}

/// Format fd as null-terminated decimal string. Returns ptr into buf.
#[cfg(all(target_os = "linux", feature = "sandbox"))]
fn format_fd(fd: libc::c_int, buf: &mut [u8; 12]) -> *const libc::c_char {
    if fd < 0 {
        buf[0] = b'0';
        buf[1] = 0;
        return buf.as_ptr() as *const libc::c_char;
    }
    let mut n = fd as u32;
    let mut i = 0usize;
    if n == 0 {
        buf[0] = b'0';
        buf[1] = 0;
        return buf.as_ptr() as *const libc::c_char;
    }
    let mut digits = [0u8; 10];
    let mut nd = 0usize;
    while n > 0 {
        digits[nd] = b'0' + (n % 10) as u8;
        n /= 10;
        nd += 1;
    }
    for j in (0..nd).rev() {
        buf[i] = digits[j];
        i += 1;
    }
    buf[i] = 0;
    buf.as_ptr() as *const libc::c_char
}

#[cfg(all(target_os = "linux", feature = "sandbox"))]
unsafe fn getaddrinfo_sandboxed(
    node: *const libc::c_char,
    service: *const libc::c_char,
    hints: *const libc::addrinfo,
    res: *mut *mut libc::addrinfo,
) -> libc::c_int {
    let fd = shm::shm_create_for_sandbox();
    if fd < 0 {
        return libc::EAI_MEMORY;
    }

    let ptr = shm::shm_map(fd);
    if ptr.is_null() {
        libc::close(fd);
        return libc::EAI_MEMORY;
    }

    protocol::build_request(ptr as *mut GetAddrInfoRequest, node, service, hints);

    let sandbox_path = libc::getenv(b"IRONLUNG_SANDBOX_PATH\0".as_ptr() as *const libc::c_char);
    let path_ptr = if !sandbox_path.is_null() && *sandbox_path != 0 {
        sandbox_path
    } else {
        b"/tmp/ironlung-sandbox\0".as_ptr() as *const libc::c_char
    };

    let mut fd_buf = [0u8; 12];
    let fd_str = format_fd(fd, &mut fd_buf);

    let argv: [*const libc::c_char; 3] = [path_ptr, fd_str, core::ptr::null()];

    let pid = crate::posix::fork();
    if pid == -1 {
        libc::munmap(ptr, shm::SHM_SIZE);
        libc::close(fd);
        return libc::EAI_SYSTEM;
    }

    if pid == 0 {
        // Strip LD_PRELOAD so the sandbox runs without loading ironlung (which would break it).
        let envp = build_env_without_ld_preload();
        crate::posix::execve(path_ptr, argv.as_ptr(), envp);
        libc::_exit(127);
    }

    let mut status: libc::c_int = 0;
    let _ = libc::waitpid(pid, &mut status, 0);

    if !libc::WIFEXITED(status) || libc::WEXITSTATUS(status) != 0 {
        libc::munmap(ptr, shm::SHM_SIZE);
        libc::close(fd);
        return libc::EAI_SYSTEM;
    }

    let resp = ptr.add(RESPONSE_OFFSET) as *const GetAddrInfoResponse;
    let r = &*resp;

    if r.result != 0 {
        errno::set_errno(r.errno);
        libc::munmap(ptr, shm::SHM_SIZE);
        libc::close(fd);
        return libc::EAI_SYSTEM;
    }

    let data_base = ptr.add(r.addrinfo_data_offset) as *const u8;
    let head = protocol::deserialize_addrinfo_list(data_base, r.addrinfo_data_len);

    libc::munmap(ptr, shm::SHM_SIZE);
    libc::close(fd);

    if head.is_null() {
        return libc::EAI_MEMORY;
    }

    *res = head;
    0
}

unsafe fn getaddrinfo_legacy(
    node: *const libc::c_char,
    service: *const libc::c_char,
    hints: *const libc::addrinfo,
    res: *mut *mut libc::addrinfo,
) -> libc::c_int {
    let f = cache::resolve(b"getaddrinfo\0", &GETADDRINFO);
    if f.is_null() {
        return libc::EAI_SYSTEM;
    }
    let f: GetaddrinfoFn = core::mem::transmute(f);
    f(node, service, hints, res)
}

#[no_mangle]
pub unsafe extern "C" fn getaddrinfo(
    node: *const libc::c_char,
    service: *const libc::c_char,
    hints: *const libc::addrinfo,
    res: *mut *mut libc::addrinfo,
) -> libc::c_int {
    if res.is_null() {
        return libc::EAI_FAIL;
    }
    if !node.is_null() {
        let mut n = 0usize;
        while n < HOST_MAX {
            if *node.add(n) == 0 {
                break;
            }
            n += 1;
        }
        if n >= HOST_MAX {
            return libc::EAI_FAIL;
        }
    }
    if !service.is_null() {
        let mut n = 0usize;
        while n < SERV_MAX {
            if *service.add(n) == 0 {
                break;
            }
            n += 1;
        }
        if n >= SERV_MAX {
            return libc::EAI_FAIL;
        }
    }

    #[cfg(all(target_os = "linux", feature = "sandbox"))]
    {
        if hints_ok(hints) {
            return getaddrinfo_sandboxed(node, service, hints, res);
        }
    }

    getaddrinfo_legacy(node, service, hints, res)
}

#[no_mangle]
pub unsafe extern "C" fn freeaddrinfo(res: *mut libc::addrinfo) {
    if res.is_null() {
        return;
    }
    let f = cache::resolve(b"freeaddrinfo\0", &FREEADDRINFO);
    if f.is_null() {
        return;
    }
    let f: FreeaddrinfoFn = core::mem::transmute(f);
    f(res)
}
