//! Locale and wide characters: iconv, wchar. Delegates to system libc.

use core::ffi::c_void;
use core::sync::atomic::AtomicPtr;

use crate::cache;

type IconvOpenFn = unsafe extern "C" fn(*const libc::c_char, *const libc::c_char) -> *mut c_void;
type IconvFn = unsafe extern "C" fn(
    *mut c_void,
    *mut *mut libc::c_char,
    *mut libc::size_t,
    *mut *mut libc::c_char,
    *mut libc::size_t,
) -> libc::size_t;
type IconvCloseFn = unsafe extern "C" fn(*mut c_void) -> libc::c_int;

static ICONV_OPEN: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static ICONV: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static ICONV_CLOSE: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());

#[no_mangle]
pub unsafe extern "C" fn iconv_open(tocode: *const libc::c_char, fromcode: *const libc::c_char) -> *mut c_void {
    if tocode.is_null() || fromcode.is_null() {
        return core::ptr::null_mut();
    }
    let f = cache::resolve(b"iconv_open\0", &ICONV_OPEN);
    if f.is_null() {
        return core::ptr::null_mut();
    }
    let f: IconvOpenFn = core::mem::transmute(f);
    f(tocode, fromcode)
}

#[no_mangle]
pub unsafe extern "C" fn iconv(
    cd: *mut c_void,
    inbuf: *mut *mut libc::c_char,
    inbytesleft: *mut libc::size_t,
    outbuf: *mut *mut libc::c_char,
    outbytesleft: *mut libc::size_t,
) -> libc::size_t {
    if cd.is_null() || inbuf.is_null() || inbytesleft.is_null() || outbuf.is_null() || outbytesleft.is_null() {
        return !0usize;
    }
    let f = cache::resolve(b"iconv\0", &ICONV);
    if f.is_null() {
        return !0usize;
    }
    let f: IconvFn = core::mem::transmute(f);
    f(cd, inbuf, inbytesleft, outbuf, outbytesleft)
}

#[no_mangle]
pub unsafe extern "C" fn iconv_close(cd: *mut c_void) -> libc::c_int {
    if cd.is_null() {
        return -1;
    }
    let f = cache::resolve(b"iconv_close\0", &ICONV_CLOSE);
    if f.is_null() {
        return -1;
    }
    let f: IconvCloseFn = core::mem::transmute(f);
    f(cd)
}
