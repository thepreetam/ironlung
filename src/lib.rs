//! IronLung: A no_std Rust shared object acting as a partial libc replacement.
//! "Trust No Pointer, Verify Every Byte, Delegate to the Kernel."

#![cfg_attr(not(feature = "hosted-test"), no_std)]
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
mod env;
#[cfg(target_os = "linux")]
mod locale;
#[cfg(target_os = "linux")]
mod passwd;
#[cfg(target_os = "linux")]
mod string;
#[cfg(target_os = "linux")]
pub mod sandbox;
#[cfg(all(feature = "stdio-kernel", not(feature = "stdio-libc"), target_os = "linux"))]
mod stdio_kernel;

#[cfg(feature = "hosted-test")]
mod tests;

use core::alloc::{GlobalAlloc, Layout};
use core::panic::PanicInfo;

use libc::{c_void, size_t};
use sc::nr::{MMAP, WRITE};
#[cfg(target_os = "linux")]
use sc::nr::EXIT_GROUP;
#[cfg(not(target_os = "linux"))]
use sc::nr::EXIT;
use sc::syscall;

// Linux mmap constants
const MAP_ANONYMOUS: i32 = 0x20;
const MAP_PRIVATE: i32 = 0x02;
const PROT_READ: i32 = 0x1;
const PROT_WRITE: i32 = 0x2;
const PAGE_SIZE: usize = 4096;

/// Bootstrap heap - talc needs initial memory for metadata.
static mut HEAP: [u8; PAGE_SIZE * 64] = [0; PAGE_SIZE * 64];

// Talc-based allocator (default)
#[cfg(not(feature = "alloc-mimalloc"))]
mod alloc_backend {
    use super::*;
    use talc::{Span, Talc, Talck};
    
    /// Syscall-based OOM handler: requests memory via mmap when allocator runs out.
    pub struct MmapOom;
    
    impl talc::OomHandler for MmapOom {
        fn handle_oom(talc: &mut Talc<Self>, _layout: Layout) -> Result<(), ()> {
            let size = PAGE_SIZE * 16;
            let ret = unsafe {
                sc::syscall!(
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
    
    pub fn ensure_allocator_init() {
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
    
    pub unsafe fn alloc(layout: Layout) -> *mut u8 {
        ensure_allocator_init();
        let mut talc = GLOBAL.lock();
        talc.alloc(layout)
    }
    
    pub unsafe fn dealloc(ptr: *mut u8, layout: Layout) {
        ensure_allocator_init();
        let mut talc = GLOBAL.lock();
        talc.dealloc(ptr, layout)
    }
    
    pub unsafe fn realloc(ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ensure_allocator_init();
        let mut talc = GLOBAL.lock();
        talc.realloc(ptr, layout, new_size)
    }
    
    #[global_allocator]
    static GLOBAL: Talck<spin::Mutex<()>, MmapOom> = Talck::new(Talc::new(MmapOom));
}

// Mimalloc-based allocator (high scalability)
#[cfg(feature = "alloc-mimalloc")]
mod alloc_backend {
    use super::*;
    
    pub fn ensure_allocator_init() {
        // Mimalloc initializes automatically
    }
    
    pub unsafe fn alloc(layout: Layout) -> *mut u8 {
        GLOBAL.alloc(layout)
    }
    
    pub unsafe fn dealloc(ptr: *mut u8, layout: Layout) {
        GLOBAL.dealloc(ptr, layout)
    }
    
    pub unsafe fn realloc(ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        GLOBAL.realloc(ptr, layout, new_size)
    }
    
    #[global_allocator]
    static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
}

use alloc_backend::{alloc, dealloc, realloc};

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn rust_eh_personality() {
    // Stub for cdylib; with panic=abort this is never called. Needed on non-Linux when deps reference it.
}

// Force linker to keep C printf shim symbols (printf, fprintf, vprintf) in the cdylib.
#[cfg(all(target_os = "linux", not(test)))]
extern "C" {
    fn ironlung_printf_keep();
}
#[cfg(all(target_os = "linux", not(test)))]
#[used]
static KEEP_PRINTF_SHIM: unsafe extern "C" fn() = ironlung_printf_keep;

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // Flush stdout buffers before panic
    #[cfg(all(feature = "stdio-kernel", not(feature = "stdio-libc"), target_os = "linux"))]
    unsafe {
        crate::stdio_kernel::flush_all_buffers();
    }
    
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

/// Size classes for per-thread cache (feature alloc-cache).
#[cfg(all(feature = "alloc-cache", target_os = "linux"))]
const CACHE_SIZE_CLASSES: [usize; 4] = [32, 64, 128, 256];

#[cfg(all(feature = "alloc-cache", target_os = "linux"))]
fn is_cacheable_size(size: usize) -> bool {
    size <= CACHE_SIZE_CLASSES[3] // Check if size <= largest cacheable size (256)
}

#[cfg(all(feature = "alloc-cache", target_os = "linux"))]
extern "C" {
    fn alloc_cache_pop(out: *mut *mut u8) -> libc::c_int;
    fn alloc_cache_push(p: *mut u8) -> libc::c_int;
    fn alloc_cache_push_with_size(p: *mut u8, size: libc::size_t) -> libc::c_int;
}

// ---------- Quarantine (feature alloc-quarantine): delay reuse for UAF mitigation ----------
#[cfg(feature = "alloc-quarantine")]
mod quarantine;

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

    #[cfg(all(feature = "alloc-cache", target_os = "linux"))]
    if is_cacheable_size(size) {
        let mut out = core::ptr::null_mut::<u8>();
        if alloc_cache_pop(&mut out) != 0 && !out.is_null() {
            core::ptr::write(out as *mut usize, size);
            return out.add(MALLOC_HEADER) as *mut c_void;
        }
    }

    #[allow(unused_mut)]
    let mut ptr = unsafe { alloc(layout) };
    #[cfg(feature = "alloc-quarantine")]
    {
        let mut drained = 0;
        while ptr.is_null() && drained < 8 && crate::quarantine::quarantine_drain_one() {
            drained += 1;
            ptr = unsafe { alloc(layout) };
        }
    }
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

    #[cfg(all(feature = "alloc-cache", target_os = "linux"))]
    if is_cacheable_size(size) && alloc_cache_push_with_size(header_ptr, size) != 0 {
        return;
    }

    #[cfg(feature = "alloc-quarantine")]
    crate::quarantine::quarantine_push(header_ptr, size);

    #[cfg(not(feature = "alloc-quarantine"))]
    {
        let total = size.checked_add(MALLOC_HEADER).unwrap();
        let layout = Layout::from_size_align(total, core::mem::align_of::<usize>()).unwrap();
        unsafe { dealloc(header_ptr, layout) };
    }
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
    let new_ptr = unsafe { realloc(header_ptr, old_layout, new_total) };
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
