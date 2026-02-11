//! Sandbox helper binary for getaddrinfo. Runs under seccomp, reads request from
//! shared memory, calls real getaddrinfo via dlsym, serializes result, exits.
//! Linux-only: requires libseccomp and runs under seccomp filter.

#[cfg(not(target_os = "linux"))]
fn main() -> std::process::ExitCode {
    eprintln!("ironlung-sandbox: Linux only");
    std::process::ExitCode::from(1)
}

#[cfg(target_os = "linux")]
fn main() -> std::process::ExitCode {
    sandbox_main()
}

#[cfg(target_os = "linux")]
fn sandbox_main() -> ! {
    use std::env;
    use std::ffi::c_void;

    use ironlung_protocol::{
        serialize_addrinfo_list, DATA_OFFSET, GetAddrInfoRequest, GetAddrInfoResponse,
        RESPONSE_OFFSET,
    };
    use libseccomp::{ScmpAction, ScmpFilterContext, ScmpSyscall};

    const SHM_SIZE: usize = 0x10000; // 64 KB
    const PROT_READ: i32 = 0x1;
    const PROT_WRITE: i32 = 0x2;
    const MAP_SHARED: i32 = 0x01;

    fn apply_seccomp_filter() -> Result<(), ()> {
        let mut filter = ScmpFilterContext::new(ScmpAction::KillProcess).map_err(|_| ())?;

        for name in [
            "read", "write", "mmap", "munmap", "mremap", "brk",
            "getrandom", "clock_gettime", "rt_sigreturn", "exit_group",
            "arch_prctl", "set_robust_list", "futex", "gettid", "sigaltstack",
        ] {
            if let Ok(syscall) = ScmpSyscall::from_name(name) {
                let _ = filter.add_rule(ScmpAction::Allow, syscall);
            }
        }

        filter.load().map_err(|_| ())
    }

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        unsafe { libc::_exit(1) };
    }

    let fd: i32 = match args[1].parse() {
        Ok(f) => f,
        Err(_) => unsafe { libc::_exit(1) },
    };

    if apply_seccomp_filter().is_err() {
        unsafe { libc::_exit(1) };
    }

    let ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            SHM_SIZE,
            PROT_READ | PROT_WRITE,
            MAP_SHARED,
            fd,
            0,
        )
    };
    if ptr == libc::MAP_FAILED {
        unsafe { libc::_exit(1) };
    }

    let req = unsafe { &*(ptr as *const GetAddrInfoRequest) };

    let node_ptr = if req.node[0] != 0 {
        req.node.as_ptr() as *const libc::c_char
    } else {
        std::ptr::null()
    };
    let service_ptr = if req.service[0] != 0 {
        req.service.as_ptr() as *const libc::c_char
    } else {
        std::ptr::null()
    };

    let hints = libc::addrinfo {
        ai_flags: req.hints.ai_flags,
        ai_family: req.hints.ai_family,
        ai_socktype: req.hints.ai_socktype,
        ai_protocol: req.hints.ai_protocol,
        ai_addrlen: 0,
        ai_addr: std::ptr::null_mut(),
        ai_canonname: std::ptr::null_mut(),
        ai_next: std::ptr::null_mut(),
    };

    const RTLD_NEXT: *mut c_void = -1_isize as *mut c_void;
    let getaddrinfo_fn = unsafe {
        libc::dlsym(RTLD_NEXT, b"getaddrinfo\0".as_ptr() as *const libc::c_char)
    };
    let freeaddrinfo_fn = unsafe {
        libc::dlsym(RTLD_NEXT, b"freeaddrinfo\0".as_ptr() as *const libc::c_char)
    };
    if getaddrinfo_fn.is_null() || freeaddrinfo_fn.is_null() {
        unsafe { libc::_exit(1) };
    }

    type GetaddrinfoFn = unsafe extern "C" fn(
        *const libc::c_char,
        *const libc::c_char,
        *const libc::addrinfo,
        *mut *mut libc::addrinfo,
    ) -> libc::c_int;
    type FreeaddrinfoFn = unsafe extern "C" fn(*mut libc::addrinfo);

    let getaddrinfo: GetaddrinfoFn = unsafe { std::mem::transmute(getaddrinfo_fn) };
    let freeaddrinfo: FreeaddrinfoFn = unsafe { std::mem::transmute(freeaddrinfo_fn) };

    let mut res: *mut libc::addrinfo = std::ptr::null_mut();
    let ret = unsafe { getaddrinfo(node_ptr, service_ptr, &hints, &mut res) };

    let base = ptr as *mut u8;
    let resp_ptr = unsafe { base.add(RESPONSE_OFFSET) as *mut GetAddrInfoResponse };
    let capacity = SHM_SIZE.saturating_sub(DATA_OFFSET);
    let data_base = unsafe { base.add(DATA_OFFSET) };

    if ret != 0 {
        let err = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        unsafe {
            std::ptr::write(
                resp_ptr,
                GetAddrInfoResponse {
                    result: ret,
                    errno: err,
                    addrinfo_data_len: 0,
                    addrinfo_data_offset: 0,
                },
            );
        }
        unsafe { libc::_exit(0) };
    }

    let len = unsafe { serialize_addrinfo_list(res, data_base, capacity) };
    unsafe { freeaddrinfo(res) };

    if len == 0 {
        unsafe {
            std::ptr::write(
                resp_ptr,
                GetAddrInfoResponse {
                    result: -1,
                    errno: libc::ENOMEM,
                    addrinfo_data_len: 0,
                    addrinfo_data_offset: 0,
                },
            );
        }
        unsafe { libc::_exit(0) };
    }

    unsafe {
        std::ptr::write(
            resp_ptr,
            GetAddrInfoResponse {
                result: 0,
                errno: 0,
                addrinfo_data_len: len,
                addrinfo_data_offset: DATA_OFFSET,
            },
        );
    }

    unsafe { libc::_exit(0) };
}
