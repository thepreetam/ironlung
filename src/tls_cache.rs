//! Thread-local slab cache for allocator performance.
//! Reduces global lock contention by caching freed blocks per thread.

use core::cell::UnsafeCell;
use core::ptr;

/// Maximum number of size classes supported by the cache.
const NUM_SIZE_CLASSES: usize = 8;

/// Size classes for slab allocation (powers of two from 16 to 2048 bytes).
const SIZE_CLASSES: [usize; NUM_SIZE_CLASSES] = [16, 32, 64, 128, 256, 512, 1024, 2048];

/// Maximum number of cached blocks per size class per thread.
const MAX_CACHED_PER_CLASS: usize = 32;

/// Thread-local cache bucket for a specific size class.
struct CacheBucket {
    /// Array of cached pointers (stack-based LIFO).
    slots: [*mut u8; MAX_CACHED_PER_CLASS],
    /// Number of currently cached pointers.
    len: usize,
}

impl CacheBucket {
    const fn new() -> Self {
        Self {
            slots: [ptr::null_mut(); MAX_CACHED_PER_CLASS],
            len: 0,
        }
    }

    /// Push a pointer to the cache. Returns true if successful.
    fn push(&mut self, ptr: *mut u8) -> bool {
        if self.len >= MAX_CACHED_PER_CLASS {
            return false;
        }
        self.slots[self.len] = ptr;
        self.len += 1;
        true
    }

    /// Pop a pointer from the cache. Returns null if empty.
    fn pop(&mut self) -> *mut u8 {
        if self.len == 0 {
            return ptr::null_mut();
        }
        self.len -= 1;
        self.slots[self.len]
    }

    /// Check if cache is empty.
    fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Check if cache is full.
    fn is_full(&self) -> bool {
        self.len >= MAX_CACHED_PER_CLASS
    }
}

/// Thread-local storage for cache buckets.
/// Uses `#[thread_local]` for zero-cost TLS access.
#[thread_local]
static mut TLS_CACHE: UnsafeCell<[CacheBucket; NUM_SIZE_CLASSES]> =
    UnsafeCell::new([CacheBucket::new(); NUM_SIZE_CLASSES]);

/// Find the appropriate size class index for a given allocation size.
fn size_to_class(size: usize) -> Option<usize> {
    for (i, &class_size) in SIZE_CLASSES.iter().enumerate() {
        if size <= class_size {
            return Some(i);
        }
    }
    None
}

/// Get mutable reference to thread-local cache (unsafe but safe in thread-local context).
unsafe fn get_cache() -> &'static mut [CacheBucket; NUM_SIZE_CLASSES] {
    &mut *TLS_CACHE.get()
}

/// Try to allocate from thread-local cache.
/// Returns pointer if successful, null otherwise.
pub unsafe fn tls_alloc(size: usize) -> *mut u8 {
    let class_idx = match size_to_class(size) {
        Some(idx) => idx,
        None => return ptr::null_mut(),
    };

    let cache = get_cache();
    let bucket = &mut cache[class_idx];
    
    if !bucket.is_empty() {
        return bucket.pop();
    }
    
    ptr::null_mut()
}

/// Try to free to thread-local cache.
/// Returns true if cached, false if cache is full or size not cacheable.
pub unsafe fn tls_free(ptr: *mut u8, size: usize) -> bool {
    let class_idx = match size_to_class(size) {
        Some(idx) => idx,
        None => return false,
    };

    let cache = get_cache();
    let bucket = &mut cache[class_idx];
    
    if bucket.is_full() {
        return false;
    }
    
    bucket.push(ptr)
}

/// Drain all cached blocks from thread-local cache.
/// Calls `dealloc_func` for each cached block.
pub unsafe fn tls_drain_all<F>(mut dealloc_func: F)
where
    F: FnMut(*mut u8, usize),
{
    let cache = get_cache();
    
    for (class_idx, bucket) in cache.iter_mut().enumerate() {
        let class_size = SIZE_CLASSES[class_idx];
        while !bucket.is_empty() {
            let ptr = bucket.pop();
            dealloc_func(ptr, class_size);
        }
    }
}

/// Check if a size is cacheable in thread-local cache.
pub fn is_tls_cacheable(size: usize) -> bool {
    size_to_class(size).is_some()
}

/// Get statistics about thread-local cache usage.
pub fn tls_stats() -> TLSStats {
    let cache = unsafe { get_cache() };
    let mut stats = TLSStats::default();
    
    for (class_idx, bucket) in cache.iter().enumerate() {
        stats.total_cached += bucket.len;
        stats.per_class[class_idx] = bucket.len;
    }
    
    stats
}

/// Statistics about thread-local cache usage.
#[derive(Default, Debug)]
pub struct TLSStats {
    /// Total number of cached blocks across all size classes.
    pub total_cached: usize,
    /// Number of cached blocks per size class.
    pub per_class: [usize; NUM_SIZE_CLASSES],
}