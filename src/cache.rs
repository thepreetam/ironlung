//! Pluggable function cache: dlsym(RTLD_NEXT) with optimized fast path.
//! Fast path: single Relaxed load; cold path: Acquire/Release for dlsym.

use core::ffi::c_void;
use core::sync::atomic::{AtomicPtr, Ordering};

const RTLD_NEXT: *mut c_void = -1isize as *mut c_void;

extern "C" {
    fn dlsym(handle: *mut c_void, symbol: *const libc::c_char) -> *mut c_void;
}

/// Resolve symbol via dlsym(RTLD_NEXT), caching result. Fast path uses Relaxed.
#[inline(always)]
pub fn resolve(name: &[u8], holder: &AtomicPtr<c_void>) -> *mut c_void {
    let ptr = holder.load(Ordering::Relaxed);
    if !ptr.is_null() {
        return ptr;
    }
    resolve_slow(name, holder)
}

#[cold]
fn resolve_slow(name: &[u8], holder: &AtomicPtr<c_void>) -> *mut c_void {
    let ptr = holder.load(Ordering::Acquire);
    if !ptr.is_null() {
        return ptr;
    }
    let name_c = {
        let mut buf = [0i8; 64];
        let len = name.len().min(buf.len() - 1);
        for (i, &b) in name[..len].iter().enumerate() {
            buf[i] = b as i8;
        }
        buf[len] = 0;
        buf.as_ptr() as *const libc::c_char
    };
    let fp = unsafe { dlsym(RTLD_NEXT, name_c) };
    if fp.is_null() {
        return core::ptr::null_mut();
    }
    let fp_void = fp as *mut c_void;
    holder.store(fp_void, Ordering::Release);
    
    // Mark bootstrap phase as complete after first successful dlsym
    // This ensures normal allocator is used for subsequent allocations
    crate::bootstrap_finish();
    
    fp_void
}
