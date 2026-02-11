//! Extra string symbols: memset, strcmp, strncpy. Validate-then-delegate to libc.

use core::ffi::c_void;
use core::sync::atomic::AtomicPtr;

use crate::cache;

type MemsetFn = unsafe extern "C" fn(*mut c_void, libc::c_int, libc::size_t) -> *mut c_void;
type StrcmpFn = unsafe extern "C" fn(*const libc::c_char, *const libc::c_char) -> libc::c_int;
type StrncpyFn = unsafe extern "C" fn(*mut libc::c_char, *const libc::c_char, libc::size_t) -> *mut libc::c_char;

static MEMSET: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static STRCMP: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static STRNCPY: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());

const SIZE_LIMIT: libc::size_t = 1_000_000_000;

#[no_mangle]
pub unsafe extern "C" fn memset(ptr: *mut c_void, c: libc::c_int, n: libc::size_t) -> *mut c_void {
    if ptr.is_null() || n > SIZE_LIMIT {
        return ptr;
    }
    let f = cache::resolve(b"memset\0", &MEMSET);
    if f.is_null() {
        return ptr;
    }
    let f: MemsetFn = core::mem::transmute(f);
    f(ptr, c, n)
}

#[no_mangle]
pub unsafe extern "C" fn strcmp(s1: *const libc::c_char, s2: *const libc::c_char) -> libc::c_int {
    if s1.is_null() || s2.is_null() {
        return 0;
    }
    let f = cache::resolve(b"strcmp\0", &STRCMP);
    if f.is_null() {
        return 0;
    }
    let f: StrcmpFn = core::mem::transmute(f);
    f(s1, s2)
}

#[no_mangle]
pub unsafe extern "C" fn strncpy(
    dest: *mut libc::c_char,
    src: *const libc::c_char,
    n: libc::size_t,
) -> *mut libc::c_char {
    if dest.is_null() || src.is_null() || n > SIZE_LIMIT {
        return dest;
    }
    let f = cache::resolve(b"strncpy\0", &STRNCPY);
    if f.is_null() {
        return dest;
    }
    let f: StrncpyFn = core::mem::transmute(f);
    f(dest, src, n)
}
