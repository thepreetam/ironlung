/* Per-thread allocator cache: 8 slots for each of 4 size classes. 
 * Used only when feature alloc-cache is enabled. __thread gives us TLS without pthread. */
#include <stddef.h>

#define CACHE_SLOTS 8
#define NUM_SIZE_CLASSES 4

// Size classes: 32, 64, 128, 256 bytes
static const size_t size_classes[NUM_SIZE_CLASSES] = {32, 64, 128, 256};

typedef struct {
    void *slots[CACHE_SLOTS];
    int len;
} CacheBucket;

static __thread CacheBucket buckets[NUM_SIZE_CLASSES];

// Helper function to find bucket index for a given size
static int size_to_bucket(size_t size) {
    for (int i = 0; i < NUM_SIZE_CLASSES; i++) {
        if (size <= size_classes[i]) {
            return i;
        }
    }
    return -1; // Not cacheable
}

int alloc_cache_pop(void **out) {
    // Try each size class from smallest to largest
    for (int i = 0; i < NUM_SIZE_CLASSES; i++) {
        CacheBucket *bucket = &buckets[i];
        if (bucket->len > 0) {
            bucket->len--;
            *out = bucket->slots[bucket->len];
            return 1;
        }
    }
    return 0;
}

int alloc_cache_push_with_size(void *p, size_t size) {
    int bucket_idx = size_to_bucket(size);
    if (bucket_idx < 0) {
        return 0; // Not cacheable
    }
    
    CacheBucket *bucket = &buckets[bucket_idx];
    if (bucket->len >= CACHE_SLOTS) {
        return 0; // Bucket full
    }
    
    bucket->slots[bucket->len] = p;
    bucket->len++;
    return 1;
}

// Backward compatibility wrapper
int alloc_cache_push(void *p) {
    // Try all buckets, starting with smallest
    for (int i = 0; i < NUM_SIZE_CLASSES; i++) {
        CacheBucket *bucket = &buckets[i];
        if (bucket->len < CACHE_SLOTS) {
            bucket->slots[bucket->len] = p;
            bucket->len++;
            return 1;
        }
    }
    return 0;
}
