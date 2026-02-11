//! IronLung: A no_std Rust shared object acting as a partial libc replacement.
//! "Trust No Pointer, Verify Every Byte, Delegate to the Kernel."

#![no_std]
#![allow(unused_imports)]

#[cfg(target_os = "linux")]
mod cache;
#[cfg(target_os = "linux")]
mod errno;
#[cfg(target_os = "linux")]
mod pthread;
#[cfg(target_os = "linux")]
mod stdio;
#[cfg(target_os = "linux")]
mod posix;
#[cfg(target_os = "linux")]
mod socket;
#[cfg(target_os = "linux")]
mod dns;
#[cfg(target_os = "linux")]
mod locale;
#[cfg(target_os = "linux")]
mod passwd;
#[cfg(target_os = "linux")]
pub mod sandbox;

use core::alloc::{GlobalAlloc, Layout};
use core::panic::PanicInfo;

use libc::{c_void, size_t};
use sc::nr::{MMAP, WRITE};
#[cfg(target_os = "linux")]
use sc::nr::EXIT_GROUP;
#[cfg(not(target_os = "linux"))]
use sc::nr::EXIT;
use sc::syscall;
use talc::{Span, Talc, Talck};

// Linux mmap constants
const MAP_ANONYMOUS: i32 = 0x20;
const MAP_PRIVATE: i32 = 0x02;
const PROT_READ: i32 = 0x1;
const PROT_WRITE: i32 = 0x2;
const PAGE_SIZE: usize = 4096;

/// Bootstrap heap - talc needs initial memory for metadata.
static mut HEAP: [u8; PAGE_SIZE * 64] = [0; PAGE_SIZE * 64];

/// Syscall-based OOM handler: requests memory via mmap when allocator runs out.
struct MmapOom;

impl talc::OomHandler for MmapOom {
    fn handle_oom(talc: &mut Talc<Self>, _layout: Layout) -> Result<(), ()> {
        let size = PAGE_SIZE * 16;
        let ret = unsafe {
            syscall!(
                MMAP,
                0usize,
                size,
                (PROT_READ | PROT_WRITE) as usize,
                (MAP_PRIVATE | MAP_ANONYMOUS) as usize,
                usize::MAX,
                0usize
            )
        };
        if ret as isize <= 0 {
            return Err(());
        }
        let span = Span::from_base_size(ret as *mut u8, size);
        unsafe { talc.claim(span).map(|_| ()).map_err(|_| ()) }
    }
}

static ALLOC_INIT: spin::Once = spin::Once::new();

fn ensure_allocator_init() {
    ALLOC_INIT.call_once(|| {
        let mut talc = GLOBAL.lock();
        let span = Span::from_base_size(
            core::ptr::addr_of_mut!(HEAP) as *mut u8,
            core::mem::size_of::<[u8; PAGE_SIZE * 64]>(),
        );
        unsafe {
            let _ = talc.claim(span);
        }
    });
}

#[global_allocator]
static GLOBAL: Talck<spin::Mutex<()>, MmapOom> = Talck::new(Talc::new(MmapOom));

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe {
        let msg = b"IronLung Panic: Memory Safety Violation Detected. Terminating.\n";
        let _ = syscall!(WRITE, 2i32 as usize, msg.as_ptr() as usize, msg.len());
        #[cfg(target_os = "linux")]
        syscall!(EXIT_GROUP, 1i32 as usize);
        #[cfg(not(target_os = "linux"))]
        syscall!(EXIT, 1i32 as usize);
    }
    loop {}
}

// ============== Memory Allocator Interceptors ==============
const MALLOC_HEADER: usize = core::mem::size_of::<usize>();

#[no_mangle]
pub unsafe extern "C" fn malloc(size: size_t) -> *mut c_void {
    if size == 0 {
        return core::ptr::null_mut();
    }
    let total = size.checked_add(MALLOC_HEADER).unwrap_or(0);
    if total == 0 {
        return core::ptr::null_mut();
    }
    let layout = match Layout::from_size_align(total, core::mem::align_of::<usize>()) {
        Ok(l) => l,
        Err(_) => return core::ptr::null_mut(),
    };
    ensure_allocator_init();
    let ptr = GLOBAL.alloc(layout);
    if ptr.is_null() {
        core::ptr::null_mut()
    } else {
        core::ptr::write(ptr as *mut usize, size);
        ptr.add(MALLOC_HEADER) as *mut c_void
    }
}

#[no_mangle]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let header_ptr = (ptr as *mut u8).sub(MALLOC_HEADER);
    let size = core::ptr::read(header_ptr as *const usize);
    let total = size.checked_add(MALLOC_HEADER).unwrap();
    let layout = Layout::from_size_align(total, core::mem::align_of::<usize>()).unwrap();
    GLOBAL.dealloc(header_ptr, layout);
}

#[no_mangle]
pub unsafe extern "C" fn realloc(ptr: *mut c_void, size: size_t) -> *mut c_void {
    if ptr.is_null() {
        return malloc(size);
    }
    if size == 0 {
        free(ptr);
        return core::ptr::null_mut();
    }
    let header_ptr = (ptr as *mut u8).sub(MALLOC_HEADER);
    let old_size = core::ptr::read(header_ptr as *const usize);
    let old_total = old_size.checked_add(MALLOC_HEADER).unwrap();
    let old_layout = Layout::from_size_align(old_total, core::mem::align_of::<usize>()).unwrap();
    let new_total = size.checked_add(MALLOC_HEADER).unwrap_or(0);
    if new_total == 0 {
        return core::ptr::null_mut();
    }
    let _new_layout = Layout::from_size_align(new_total, core::mem::align_of::<usize>()).unwrap();
    let new_ptr = GLOBAL.realloc(header_ptr, old_layout, new_total);
    if new_ptr.is_null() {
        core::ptr::null_mut()
    } else {
        core::ptr::write(new_ptr as *mut usize, size);
        new_ptr.add(MALLOC_HEADER) as *mut c_void
    }
}

#[no_mangle]
pub unsafe extern "C" fn calloc(nmemb: size_t, size: size_t) -> *mut c_void {
    let total = nmemb.checked_mul(size).unwrap_or(0);
    if total == 0 {
        return core::ptr::null_mut();
    }
    let ptr = malloc(total);
    if !ptr.is_null() {
        core::ptr::write_bytes(ptr as *mut u8, 0, total);
    }
    ptr
}

// ============== String / Memory Operations ==============

const MEMCPY_SIZE_LIMIT: usize = 1_000_000_000;

#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if n > MEMCPY_SIZE_LIMIT {
        panic!("IronLung: memcpy size exceeds safety threshold");
    }
    let d = dest as usize;
    let s = src as usize;
    if (d > s && d < s + n) || (s > d && s < d + n) {
        panic!("IronLung: memcpy overlap detected");
    }
    core::ptr::copy_nonoverlapping(src, dest, n);
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if n > MEMCPY_SIZE_LIMIT {
        panic!("IronLung: memmove size exceeds safety threshold");
    }
    core::ptr::copy(src, dest, n);
    dest
}

const STR_HEURISTIC_LIMIT: usize = 4096;

#[no_mangle]
pub unsafe extern "C" fn strcpy(dest: *mut i8, src: *const i8) -> *mut i8 {
    if dest.is_null() || src.is_null() {
        return core::ptr::null_mut();
    }
    let mut len = 0usize;
    while *src.add(len) != 0 {
        len += 1;
        if len > STR_HEURISTIC_LIMIT {
            panic!("IronLung: strcpy source exceeds heuristic threshold");
        }
    }
    for i in 0..=len {
        *dest.add(i) = *src.add(i);
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn puts(s: *const i8) -> i32 {
    if s.is_null() {
        let _ = syscall!(WRITE, 1usize, "\n".as_ptr() as usize, 1);
        return 1;
    }
    let mut len = 0usize;
    while *s.add(len) != 0 {
        len += 1;
        if len > STR_HEURISTIC_LIMIT {
            return -1;
        }
    }
    let tag = "[IronLung] ";
    let _ = syscall!(WRITE, 1usize, tag.as_ptr() as usize, tag.len());
    let _ = syscall!(WRITE, 1usize, s as usize, len);
    let _ = syscall!(WRITE, 1usize, "\n".as_ptr() as usize, 1);
    1
}
