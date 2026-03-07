//! Kernel-delegating vprintf (feature stdio-kernel): validate format, format to buffer, write(1).
//! Supports %% and %s only in this minimal version.

use core::ffi::c_void;
use core::sync::atomic::{AtomicUsize, Ordering};

use libc::c_char;
use sc::nr::WRITE;
use sc::syscall;

const OUT_MAX: usize = 4096;
const FORMAT_MAX: usize = 4096;
const BUFFER_SIZE: usize = 4096; // 4KB buffer

// Thread-local output buffer for kernel stdio
#[cfg(target_os = "linux")]
static BUFFER_USED: AtomicUsize = AtomicUsize::new(0);

#[cfg(target_os = "linux")]
#[thread_local]
static mut STDOUT_BUFFER: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];

/// Flush the thread-local stdout buffer
#[cfg(target_os = "linux")]
unsafe fn flush_stdout_buffer() -> isize {
    let used = BUFFER_USED.load(Ordering::Relaxed);
    if used == 0 {
        return 0;
    }
    
    let ret = syscall!(WRITE, 1usize, STDOUT_BUFFER.as_ptr() as usize, used) as isize;
    if ret > 0 {
        BUFFER_USED.store(0, Ordering::Relaxed);
    }
    ret
}

/// Flush all thread-local stdout buffers (call at exit)
#[cfg(target_os = "linux")]
pub unsafe fn flush_all_buffers() {
    // Note: This is a simplified implementation
    // In a real implementation, we would need to iterate over all threads
    flush_stdout_buffer();
}

/// Write to stdout with buffering
#[cfg(target_os = "linux")]
pub unsafe fn buffered_write(data: &[u8]) -> isize {
    let mut total_written = 0isize;
    let mut offset = 0;
    
    while offset < data.len() {
        let used = BUFFER_USED.load(Ordering::Relaxed);
        let available = BUFFER_SIZE - used;
        
        if available == 0 {
            // Buffer full, flush it
            let ret = flush_stdout_buffer();
            if ret < 0 {
                return if total_written > 0 { total_written } else { ret };
            }
            continue;
        }
        
        let to_copy = core::cmp::min(available, data.len() - offset);
        STDOUT_BUFFER[used..used + to_copy].copy_from_slice(&data[offset..offset + to_copy]);
        BUFFER_USED.store(used + to_copy, Ordering::Relaxed);
        
        offset += to_copy;
        total_written += to_copy as isize;
        
        // If buffer is full or data contains newline, flush
        if used + to_copy == BUFFER_SIZE || data[offset - 1] == b'\n' {
            let ret = flush_stdout_buffer();
            if ret < 0 {
                return if total_written > 0 { total_written } else { ret };
            }
        }
    }
    
    total_written
}

extern "C" {
    fn stdio_va_arg_s(ap: *mut c_void) -> *const c_char;
    fn stdio_va_end(ap: *mut c_void);
}

/// Returns true if format string is valid (no %n, within length limit).
fn validate_format(fmt: *const c_char) -> bool {
    if fmt.is_null() {
        return false;
    }
    let mut n = 0usize;
    while n < FORMAT_MAX {
        let c = unsafe { *fmt.add(n) as u8 };
        if c == 0 {
            return true;
        }
        if c == b'%' {
            n += 1;
            if n >= FORMAT_MAX {
                return false;
            }
            let c2 = unsafe { *fmt.add(n) as u8 };
            if c2 == 0 {
                return false;
            }
            if c2 == b'n' {
                return false; /* reject %n */
            }
            if c2 != b'%' {
                n += 1; /* skip width/precision */
                continue;
            }
        }
        n += 1;
    }
    false
}

#[no_mangle]
pub unsafe extern "C" fn vprintf(format: *const c_char, ap: *mut c_void) -> libc::c_int {
    if format.is_null() || !validate_format(format) {
        return -1;
    }
    let mut buf = [0u8; OUT_MAX];
    let mut i = 0usize;
    let mut n = 0usize;
    while n < FORMAT_MAX && i < OUT_MAX.saturating_sub(1) {
        let c = *format.add(n) as u8;
        if c == 0 {
            break;
        }
        if c == b'%' {
            n += 1;
            if n >= FORMAT_MAX {
                break;
            }
            let c2 = *format.add(n) as u8;
            if c2 == b'%' {
                buf[i] = b'%';
                i += 1;
                n += 1;
                continue;
            }
            if c2 == b's' {
                let s = stdio_va_arg_s(ap);
                n += 1;
                if !s.is_null() {
                    let mut j = 0usize;
                    while j < OUT_MAX.saturating_sub(i).saturating_sub(1) {
                        let ch = *s.add(j);
                        if ch == 0 {
                            break;
                        }
                        buf[i + j] = ch as u8;
                        j += 1;
                    }
                    i += j;
                }
                continue;
            }
            /* unsupported: output ? and skip */
            buf[i] = b'?';
            i += 1;
            n += 1;
            continue;
        }
        buf[i] = c;
        i += 1;
        n += 1;
    }
    stdio_va_end(ap);
    if i == 0 {
        return 0;
    }
    
    #[cfg(target_os = "linux")]
    let ret = unsafe { buffered_write(&buf[..i]) };
    #[cfg(not(target_os = "linux"))]
    let ret = syscall!(WRITE, 1usize, buf.as_ptr() as usize, i) as isize;
    
    if ret < 0 {
        -1
    } else {
        ret as libc::c_int
    }
}
