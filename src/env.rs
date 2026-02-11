//! getenv: validate-then-delegate to libc.

use core::ffi::c_void;
use core::sync::atomic::AtomicPtr;

use crate::cache;

type GetenvFn = unsafe extern "C" fn(*const libc::c_char) -> *mut libc::c_char;

static GETENV: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());

const NAME_MAX: usize = 4096;

#[no_mangle]
pub unsafe extern "C" fn getenv(name: *const libc::c_char) -> *mut libc::c_char {
    if name.is_null() {
        return core::ptr::null_mut();
    }
    let mut len = 0usize;
    while *name.add(len) != 0 && len < NAME_MAX {
        len += 1;
    }
    if len >= NAME_MAX || *name.add(len) != 0 {
        return core::ptr::null_mut();
    }
    let f = cache::resolve(b"getenv\0", &GETENV);
    if f.is_null() {
        return core::ptr::null_mut();
    }
    let f: GetenvFn = core::mem::transmute(f);
    f(name)
}
