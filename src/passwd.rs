//! User/group and cryptography: getpwnam, crypt. Delegates to system libc.

use core::ffi::c_void;
use core::sync::atomic::AtomicPtr;

use crate::cache;

type GetpwnamFn = unsafe extern "C" fn(*const libc::c_char) -> *mut libc::passwd;
type CryptFn = unsafe extern "C" fn(*const libc::c_char, *const libc::c_char) -> *mut libc::c_char;

static GETPWNAM: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static CRYPT: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());

const NAME_MAX: usize = 256;
const SALT_MAX: usize = 128;

#[no_mangle]
pub unsafe extern "C" fn getpwnam(name: *const libc::c_char) -> *mut libc::passwd {
    if name.is_null() {
        return core::ptr::null_mut();
    }
    let mut n = 0usize;
    while n < NAME_MAX {
        if *name.add(n) == 0 {
            break;
        }
        n += 1;
    }
    if n >= NAME_MAX {
        return core::ptr::null_mut();
    }
    let f = cache::resolve(b"getpwnam\0", &GETPWNAM);
    if f.is_null() {
        return core::ptr::null_mut();
    }
    let f: GetpwnamFn = core::mem::transmute(f);
    f(name)
}

#[no_mangle]
pub unsafe extern "C" fn crypt(key: *const libc::c_char, salt: *const libc::c_char) -> *mut libc::c_char {
    if key.is_null() || salt.is_null() {
        return core::ptr::null_mut();
    }
    let mut n = 0usize;
    while n < SALT_MAX {
        if *salt.add(n) == 0 {
            break;
        }
        n += 1;
    }
    if n >= SALT_MAX {
        return core::ptr::null_mut();
    }
    let f = cache::resolve(b"crypt\0", &CRYPT);
    if f.is_null() {
        return core::ptr::null_mut();
    }
    let f: CryptFn = core::mem::transmute(f);
    f(key, salt)
}
