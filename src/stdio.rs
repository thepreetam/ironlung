//! FILE* and basic stdio: fopen, fclose, fread, fwrite, fgets.
//! Delegates to system libc via dlsym(RTLD_NEXT) with input validation.

use core::ffi::c_void;
use core::sync::atomic::AtomicPtr;

use crate::cache;

type FopenFn = unsafe extern "C" fn(*const libc::c_char, *const libc::c_char) -> *mut libc::c_void;
type FcloseFn = unsafe extern "C" fn(*mut libc::c_void) -> libc::c_int;
type FreadFn = unsafe extern "C" fn(*mut c_void, libc::size_t, libc::size_t, *mut libc::c_void) -> libc::size_t;
type FwriteFn = unsafe extern "C" fn(*const c_void, libc::size_t, libc::size_t, *mut libc::c_void) -> libc::size_t;
type FgetsFn = unsafe extern "C" fn(*mut libc::c_char, libc::c_int, *mut libc::c_void) -> *mut libc::c_char;

static FOPEN: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static FCLOSE: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static FREAD: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static FWRITE: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static FGETS: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());

const MAX_SIZE: usize = 1_000_000_000;

#[no_mangle]
pub unsafe extern "C" fn fopen(filename: *const libc::c_char, mode: *const libc::c_char) -> *mut libc::c_void {
    if filename.is_null() || mode.is_null() {
        return core::ptr::null_mut();
    }
    let f = cache::resolve(b"fopen\0", &FOPEN);
    if f.is_null() {
        return core::ptr::null_mut();
    }
    let f: FopenFn = core::mem::transmute(f);
    f(filename, mode)
}

#[no_mangle]
pub unsafe extern "C" fn fclose(stream: *mut libc::c_void) -> libc::c_int {
    if stream.is_null() {
        return libc::EOF;
    }
    let f = cache::resolve(b"fclose\0", &FCLOSE);
    if f.is_null() {
        return libc::EOF;
    }
    let f: FcloseFn = core::mem::transmute(f);
    f(stream)
}

#[no_mangle]
pub unsafe extern "C" fn fread(
    ptr: *mut c_void,
    size: libc::size_t,
    nmemb: libc::size_t,
    stream: *mut libc::c_void,
) -> libc::size_t {
    if ptr.is_null() || stream.is_null() || size == 0 {
        return 0;
    }
    if size > MAX_SIZE || nmemb > MAX_SIZE || size.checked_mul(nmemb).is_none() {
        return 0;
    }
    let f = cache::resolve(b"fread\0", &FREAD);
    if f.is_null() {
        return 0;
    }
    let f: FreadFn = core::mem::transmute(f);
    f(ptr, size, nmemb, stream)
}

#[no_mangle]
pub unsafe extern "C" fn fwrite(
    ptr: *const c_void,
    size: libc::size_t,
    nmemb: libc::size_t,
    stream: *mut libc::c_void,
) -> libc::size_t {
    if ptr.is_null() || stream.is_null() || size == 0 {
        return 0;
    }
    if size > MAX_SIZE || nmemb > MAX_SIZE || size.checked_mul(nmemb).is_none() {
        return 0;
    }
    let f = cache::resolve(b"fwrite\0", &FWRITE);
    if f.is_null() {
        return 0;
    }
    let f: FwriteFn = core::mem::transmute(f);
    f(ptr, size, nmemb, stream)
}

#[no_mangle]
pub unsafe extern "C" fn fgets(
    s: *mut libc::c_char,
    n: libc::c_int,
    stream: *mut libc::c_void,
) -> *mut libc::c_char {
    if s.is_null() || stream.is_null() || n <= 0 {
        return core::ptr::null_mut();
    }
    let f = cache::resolve(b"fgets\0", &FGETS);
    if f.is_null() {
        return core::ptr::null_mut();
    }
    let f: FgetsFn = core::mem::transmute(f);
    f(s, n, stream)
}
