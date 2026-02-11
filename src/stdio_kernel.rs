//! Kernel-delegating vprintf (feature stdio-kernel): validate format, format to buffer, write(1).
//! Supports %% and %s only in this minimal version.

use core::ffi::c_void;

use libc::c_char;
use sc::nr::WRITE;
use sc::syscall;

const OUT_MAX: usize = 4096;
const FORMAT_MAX: usize = 4096;

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
        let c = unsafe { *fmt.add(n) };
        if c == 0 {
            return true;
        }
        if c == b'%' as i8 {
            n += 1;
            if n >= FORMAT_MAX {
                return false;
            }
            let c2 = unsafe { *fmt.add(n) };
            if c2 == 0 {
                return false;
            }
            if c2 == b'n' as i8 {
                return false; /* reject %n */
            }
            if c2 != b'%' as i8 {
                n += 1; /* skip width/precision */
                continue;
            }
        }
        n += 1;
    }
    false
}

#[no_mangle]
pub unsafe extern "C" fn vprintf(format: *const c_char, mut ap: libc::va_list) -> libc::c_int {
    if format.is_null() || !validate_format(format) {
        return -1;
    }
    let mut buf = [0u8; OUT_MAX];
    let mut i = 0usize;
    let mut n = 0usize;
    while n < FORMAT_MAX && i < OUT_MAX.saturating_sub(1) {
        let c = *format.add(n);
        if c == 0 {
            break;
        }
        if c == b'%' as i8 {
            n += 1;
            if n >= FORMAT_MAX {
                break;
            }
            let c2 = *format.add(n);
            if c2 == b'%' as i8 {
                buf[i] = b'%';
                i += 1;
                n += 1;
                continue;
            }
            if c2 == b's' as i8 {
                let s = stdio_va_arg_s(&mut ap as *mut libc::va_list as *mut c_void);
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
        buf[i] = c as u8;
        i += 1;
        n += 1;
    }
    stdio_va_end(&mut ap as *mut libc::va_list as *mut c_void);
    if i == 0 {
        return 0;
    }
    let ret = syscall!(WRITE, 1usize, buf.as_ptr() as usize, i) as isize;
    if ret < 0 {
        -1
    } else {
        ret as libc::c_int
    }
}
