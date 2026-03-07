//! Pthread compatibility layer: delegates to system libc via dlsym(RTLD_NEXT).
//! IronLung provides memory/string; pthread stays with the system.

use core::ffi::c_void;
use core::sync::atomic::AtomicPtr;

use crate::cache;

type PthreadCreate = unsafe extern "C" fn(
    *mut libc::pthread_t,
    *const libc::pthread_attr_t,
    Option<extern "C" fn(*mut c_void) -> *mut c_void>,
    *mut c_void,
) -> libc::c_int;
type PthreadJoin = unsafe extern "C" fn(libc::pthread_t, *mut *mut c_void) -> libc::c_int;
type PthreadMutexInit = unsafe extern "C" fn(*mut libc::pthread_mutex_t, *const libc::pthread_mutexattr_t) -> libc::c_int;
type PthreadMutexLock = unsafe extern "C" fn(*mut libc::pthread_mutex_t) -> libc::c_int;
type PthreadMutexUnlock = unsafe extern "C" fn(*mut libc::pthread_mutex_t) -> libc::c_int;
type PthreadMutexDestroy = unsafe extern "C" fn(*mut libc::pthread_mutex_t) -> libc::c_int;
type PthreadCondInit = unsafe extern "C" fn(*mut libc::pthread_cond_t, *const libc::pthread_condattr_t) -> libc::c_int;
type PthreadCondWait = unsafe extern "C" fn(*mut libc::pthread_cond_t, *mut libc::pthread_mutex_t) -> libc::c_int;
type PthreadCondSignal = unsafe extern "C" fn(*mut libc::pthread_cond_t) -> libc::c_int;
type PthreadCondDestroy = unsafe extern "C" fn(*mut libc::pthread_cond_t) -> libc::c_int;

static PTHREAD_CREATE: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static PTHREAD_JOIN: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static PTHREAD_MUTEX_INIT: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static PTHREAD_MUTEX_LOCK: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static PTHREAD_MUTEX_UNLOCK: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static PTHREAD_MUTEX_DESTROY: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static PTHREAD_COND_INIT: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static PTHREAD_COND_WAIT: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static PTHREAD_COND_SIGNAL: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static PTHREAD_COND_DESTROY: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());

#[no_mangle]
pub unsafe extern "C" fn pthread_create(
    thread: *mut libc::pthread_t,
    attr: *const libc::pthread_attr_t,
    start_routine: Option<extern "C" fn(*mut c_void) -> *mut c_void>,
    arg: *mut c_void,
) -> libc::c_int {
    let f = cache::resolve(b"pthread_create\0", &PTHREAD_CREATE);
    if f.is_null() {
        return libc::ENOSYS;
    }
    let f: PthreadCreate = core::mem::transmute(f);
    f(thread, attr, start_routine, arg)
}

#[no_mangle]
pub unsafe extern "C" fn pthread_join(thread: libc::pthread_t, retval: *mut *mut c_void) -> libc::c_int {
    let f = cache::resolve(b"pthread_join\0", &PTHREAD_JOIN);
    if f.is_null() {
        return libc::ENOSYS;
    }
    let f: PthreadJoin = core::mem::transmute(f);
    f(thread, retval)
}

#[no_mangle]
pub unsafe extern "C" fn pthread_mutex_init(
    mutex: *mut libc::pthread_mutex_t,
    attr: *const libc::pthread_mutexattr_t,
) -> libc::c_int {
    let f = cache::resolve(b"pthread_mutex_init\0", &PTHREAD_MUTEX_INIT);
    if f.is_null() {
        return libc::ENOSYS;
    }
    let f: PthreadMutexInit = core::mem::transmute(f);
    f(mutex, attr)
}

#[no_mangle]
pub unsafe extern "C" fn pthread_mutex_lock(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    let f = cache::resolve(b"pthread_mutex_lock\0", &PTHREAD_MUTEX_LOCK);
    if f.is_null() {
        return libc::ENOSYS;
    }
    let f: PthreadMutexLock = core::mem::transmute(f);
    f(mutex)
}

#[no_mangle]
pub unsafe extern "C" fn pthread_mutex_unlock(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    let f = cache::resolve(b"pthread_mutex_unlock\0", &PTHREAD_MUTEX_UNLOCK);
    if f.is_null() {
        return libc::ENOSYS;
    }
    let f: PthreadMutexUnlock = core::mem::transmute(f);
    f(mutex)
}

#[no_mangle]
pub unsafe extern "C" fn pthread_mutex_destroy(mutex: *mut libc::pthread_mutex_t) -> libc::c_int {
    let f = cache::resolve(b"pthread_mutex_destroy\0", &PTHREAD_MUTEX_DESTROY);
    if f.is_null() {
        return libc::ENOSYS;
    }
    let f: PthreadMutexDestroy = core::mem::transmute(f);
    f(mutex)
}

#[no_mangle]
pub unsafe extern "C" fn pthread_cond_init(
    cond: *mut libc::pthread_cond_t,
    attr: *const libc::pthread_condattr_t,
) -> libc::c_int {
    let f = cache::resolve(b"pthread_cond_init\0", &PTHREAD_COND_INIT);
    if f.is_null() {
        return libc::ENOSYS;
    }
    let f: PthreadCondInit = core::mem::transmute(f);
    f(cond, attr)
}

#[no_mangle]
pub unsafe extern "C" fn pthread_cond_wait(
    cond: *mut libc::pthread_cond_t,
    mutex: *mut libc::pthread_mutex_t,
) -> libc::c_int {
    let f = cache::resolve(b"pthread_cond_wait\0", &PTHREAD_COND_WAIT);
    if f.is_null() {
        return libc::ENOSYS;
    }
    let f: PthreadCondWait = core::mem::transmute(f);
    f(cond, mutex)
}

#[no_mangle]
pub unsafe extern "C" fn pthread_cond_signal(cond: *mut libc::pthread_cond_t) -> libc::c_int {
    let f = cache::resolve(b"pthread_cond_signal\0", &PTHREAD_COND_SIGNAL);
    if f.is_null() {
        return libc::ENOSYS;
    }
    let f: PthreadCondSignal = core::mem::transmute(f);
    f(cond)
}

#[no_mangle]
pub unsafe extern "C" fn pthread_cond_destroy(cond: *mut libc::pthread_cond_t) -> libc::c_int {
    let f = cache::resolve(b"pthread_cond_destroy\0", &PTHREAD_COND_DESTROY);
    if f.is_null() {
        return libc::ENOSYS;
    }
    let f: PthreadCondDestroy = core::mem::transmute(f);
    f(cond)
}