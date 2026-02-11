//! errno handling for syscall error paths.
//! Uses __errno_location from libc via dlsym (one-time resolve).

use core::ffi::c_void;
use core::sync::atomic::{AtomicPtr, Ordering};

const RTLD_NEXT: *mut c_void = -1isize as *mut c_void;

extern "C" {
    fn dlsym(handle: *mut c_void, symbol: *const libc::c_char) -> *mut c_void;
}

type ErrnoLocationFn = unsafe extern "C" fn() -> *mut libc::c_int;

static ERRNO_LOCATION: core::sync::atomic::AtomicPtr<c_void> =
    core::sync::atomic::AtomicPtr::new(core::ptr::null_mut());

fn get_errno_location() -> *mut libc::c_int {
    let ptr = ERRNO_LOCATION.load(Ordering::Acquire);
    if !ptr.is_null() {
        return ptr as *mut libc::c_int;
    }
    let name = b"__errno_location\0";
    let name_c = name.as_ptr() as *const libc::c_char;
    let fp = unsafe { dlsym(RTLD_NEXT, name_c) };
    if fp.is_null() {
        return core::ptr::null_mut();
    }
    let f: ErrnoLocationFn = unsafe { core::mem::transmute(fp) };
    let loc = unsafe { f() };
    ERRNO_LOCATION.store(loc as *mut c_void, Ordering::Release);
    loc
}

/// Set errno to the given value. Call when a syscall returns a negative error code.
/// Linux returns -errno (e.g. -EINTR), so we negate to get the positive errno.
#[inline(always)]
pub fn set_errno_from_syscall(ret: isize) {
    if ret < 0 {
        let loc = get_errno_location();
        if !loc.is_null() {
            unsafe {
                *loc = (-ret) as libc::c_int;
            }
        }
    }
}

/// Set errno to a direct value (e.g. from sandbox response).
#[inline(always)]
pub fn set_errno(e: libc::c_int) {
    let loc = get_errno_location();
    if !loc.is_null() {
        unsafe {
            *loc = e;
        }
    }
}
