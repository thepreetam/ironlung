/* Per-thread allocator cache: 8 slots for one size class. Used only when
 * feature alloc-cache is enabled. __thread gives us TLS without pthread. */
#define CACHE_SLOTS 8

static __thread void *slots[CACHE_SLOTS];
static __thread int len;

int alloc_cache_pop(void **out) {
    if (len == 0)
        return 0;
    len--;
    *out = slots[len];
    return 1;
}

int alloc_cache_push(void *p) {
    if (len >= CACHE_SLOTS)
        return 0;
    slots[len++] = p;
    return 1;
}
