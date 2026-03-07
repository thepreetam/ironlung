//! Per-thread quarantine ring buffer for UAF mitigation.
//! Uses thread-local storage to avoid global lock contention.

use core::alloc::Layout;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::dealloc;
use crate::MALLOC_HEADER;

const QUARANTINE_MAX_ENTRIES: usize = 64; // Smaller per-thread buffer
const QUARANTINE_BYTES_CAP: usize = 1024 * 128; // 128KB per thread

// Thread-local quarantine state
#[thread_local]
static mut QUARANTINE_PTRS: [*mut u8; QUARANTINE_MAX_ENTRIES] = [core::ptr::null_mut(); QUARANTINE_MAX_ENTRIES];
#[thread_local]
static mut QUARANTINE_SIZES: [usize; QUARANTINE_MAX_ENTRIES] = [0; QUARANTINE_MAX_ENTRIES];
#[thread_local]
static mut QUARANTINE_HEAD: usize = 0;
#[thread_local]
static mut QUARANTINE_LEN: usize = 0;
#[thread_local]
static mut QUARANTINE_TOTAL_BYTES: usize = 0;

// Global atomic for total quarantined bytes (for monitoring)
static GLOBAL_QUARANTINE_BYTES: AtomicUsize = AtomicUsize::new(0);

/// Get the quarantine bytes cap from environment variable
fn quarantine_bytes_cap() -> usize {
    use core::ffi::CStr;
    
    static QUARANTINE_CAP: spin::Once<usize> = spin::Once::new();
    QUARANTINE_CAP.call_once(|| {
        unsafe {
            let env_var = libc::getenv(b"IRONLUNG_QUARANTINE_SIZE\0".as_ptr() as *const libc::c_char);
            if !env_var.is_null() {
                let cstr = CStr::from_ptr(env_var);
                if let Ok(s) = cstr.to_str() {
                    if let Ok(val) = s.parse::<usize>() {
                        if val == 0 {
                            return 0; // Disable quarantine
                        }
                        // Ensure minimum of 1 page, maximum of 1GB
                        const PAGE_SIZE: usize = 4096;
                        let capped = val.clamp(PAGE_SIZE, PAGE_SIZE * 262144); // 1GB max
                        return capped;
                    }
                }
            }
        }
        QUARANTINE_BYTES_CAP // Default
    })
    .clone()
}

/// Push a freed block to the thread-local quarantine
pub unsafe fn quarantine_push(header_ptr: *mut u8, size: usize) {
    let bytes_cap = quarantine_bytes_cap();
    
    // If quarantine is disabled, deallocate immediately
    if bytes_cap == 0 {
        let layout = Layout::from_size_align(size + crate::MALLOC_HEADER, core::mem::align_of::<usize>()).unwrap();
        dealloc(header_ptr, layout);
        return;
    }
    
    // Check if we need to drain due to capacity
    if QUARANTINE_LEN >= QUARANTINE_MAX_ENTRIES || QUARANTINE_TOTAL_BYTES + size > bytes_cap {
        // Drain the oldest entry
        if QUARANTINE_LEN > 0 {
            let head = QUARANTINE_HEAD;
            let old_ptr = QUARANTINE_PTRS[head];
            let old_size = QUARANTINE_SIZES[head];
            
            QUARANTINE_PTRS[head] = core::ptr::null_mut();
            QUARANTINE_SIZES[head] = 0;
            QUARANTINE_HEAD = (head + 1) % QUARANTINE_MAX_ENTRIES;
            QUARANTINE_LEN -= 1;
            QUARANTINE_TOTAL_BYTES -= old_size;
            GLOBAL_QUARANTINE_BYTES.fetch_sub(old_size, Ordering::Relaxed);
            
            let layout = Layout::from_size_align(old_size + crate::MALLOC_HEADER, core::mem::align_of::<usize>()).unwrap();
            dealloc(old_ptr, layout);
        }
    }
    
    // Add new entry if there's space
    if QUARANTINE_LEN < QUARANTINE_MAX_ENTRIES && QUARANTINE_TOTAL_BYTES + size <= bytes_cap {
        let idx = (QUARANTINE_HEAD + QUARANTINE_LEN) % QUARANTINE_MAX_ENTRIES;
        QUARANTINE_PTRS[idx] = header_ptr;
        QUARANTINE_SIZES[idx] = size;
        QUARANTINE_LEN += 1;
        QUARANTINE_TOTAL_BYTES += size;
        GLOBAL_QUARANTINE_BYTES.fetch_add(size, Ordering::Relaxed);
    } else {
        // No space, deallocate immediately
        let layout = Layout::from_size_align(size + crate::MALLOC_HEADER, core::mem::align_of::<usize>()).unwrap();
        dealloc(header_ptr, layout);
    }
}

/// Drain one entry from quarantine (used when malloc needs memory)
pub fn quarantine_drain_one() -> bool {
    unsafe {
        if QUARANTINE_LEN == 0 {
            return false;
        }
        
        let head = QUARANTINE_HEAD;
        let old_ptr = QUARANTINE_PTRS[head];
        let old_size = QUARANTINE_SIZES[head];
        
        if old_ptr.is_null() {
            return false;
        }
        
        QUARANTINE_PTRS[head] = core::ptr::null_mut();
        QUARANTINE_SIZES[head] = 0;
        QUARANTINE_HEAD = (head + 1) % QUARANTINE_MAX_ENTRIES;
        QUARANTINE_LEN -= 1;
        QUARANTINE_TOTAL_BYTES -= old_size;
        GLOBAL_QUARANTINE_BYTES.fetch_sub(old_size, Ordering::Relaxed);
        
        let layout = Layout::from_size_align(old_size + crate::MALLOC_HEADER, core::mem::align_of::<usize>()).unwrap();
        dealloc(old_ptr, layout);
        true
    }
}

/// Get total quarantined bytes across all threads
pub fn get_global_quarantine_bytes() -> usize {
    GLOBAL_QUARANTINE_BYTES.load(Ordering::Relaxed)
}