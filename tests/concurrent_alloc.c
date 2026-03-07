/* Concurrent allocator stress test for IronLung.
 * Compile: gcc -O2 -lpthread -o concurrent_alloc tests/concurrent_alloc.c
 * Run: ./concurrent_alloc [threads] [iterations] [size]
 * With IronLung: LD_PRELOAD=./target/release/libironlung.so ./concurrent_alloc 16 10000 1024
 */

#include <stdlib.h>
#include <stdio.h>
#include <pthread.h>
#include <string.h>
#include <time.h>
#include <errno.h>
#include <stdatomic.h>

static int num_threads = 16;
static long iterations = 10000;
static size_t alloc_size = 1024;
static atomic_int start = 0;

typedef struct {
    long id;
    long allocations;
    long failures;
} thread_result_t;

static void *alloc_thread(void *arg) {
    thread_result_t *result = (thread_result_t *)arg;
    result->allocations = 0;
    result->failures = 0;
    
    // Wait for start signal
    while (!atomic_load(&start)) {
        // spin
    }
    
    // Allocate and free in patterns
    for (long i = 0; i < iterations; i++) {
        // Pattern 1: Single allocation
        void *ptr1 = malloc(alloc_size);
        if (ptr1) {
            memset(ptr1, result->id, alloc_size);
            free(ptr1);
            result->allocations++;
        } else {
            result->failures++;
        }
        
        // Pattern 2: Multiple allocations then free
        void *ptrs[8];
        for (int j = 0; j < 8; j++) {
            ptrs[j] = malloc(alloc_size / 2);
            if (ptrs[j]) {
                memset(ptrs[j], j, alloc_size / 2);
            }
        }
        for (int j = 0; j < 8; j++) {
            if (ptrs[j]) {
                free(ptrs[j]);
                result->allocations++;
            }
        }
        
        // Pattern 3: Realloc test
        void *ptr3 = malloc(alloc_size / 4);
        if (ptr3) {
            ptr3 = realloc(ptr3, alloc_size * 2);
            if (ptr3) {
                free(ptr3);
                result->allocations++;
            }
        }
        
        // Pattern 4: Calloc test
        void *ptr4 = calloc(10, alloc_size / 10);
        if (ptr4) {
            // Verify zero-initialized
            char *p = (char *)ptr4;
            for (size_t k = 0; k < alloc_size; k++) {
                if (p[k] != 0) {
                    fprintf(stderr, "Thread %ld: calloc not zeroed at byte %zu\n", result->id, k);
                    break;
                }
            }
            free(ptr4);
            result->allocations++;
        }
    }
    
    return NULL;
}

int main(int argc, char *argv[]) {
    if (argc > 1) num_threads = atoi(argv[1]);
    if (argc > 2) iterations = atol(argv[2]);
    if (argc > 3) alloc_size = atol(argv[3]);
    
    if (num_threads <= 0 || iterations <= 0 || alloc_size <= 0) {
        fprintf(stderr, "Invalid arguments\n");
        return 1;
    }
    
    printf("Starting concurrent allocator stress test:\n");
    printf("  Threads: %d\n", num_threads);
    printf("  Iterations per thread: %ld\n", iterations);
    printf("  Base allocation size: %zu bytes\n", alloc_size);
    printf("  Total expected allocations: %ld\n", num_threads * iterations * 12); // ~12 allocs per iteration
    
    pthread_t *threads = malloc(num_threads * sizeof(pthread_t));
    thread_result_t *results = malloc(num_threads * sizeof(thread_result_t));
    
    if (!threads || !results) {
        fprintf(stderr, "Failed to allocate memory for threads/results\n");
        return 1;
    }
    
    // Create threads
    for (int i = 0; i < num_threads; i++) {
        results[i].id = i;
        if (pthread_create(&threads[i], NULL, alloc_thread, &results[i]) != 0) {
            fprintf(stderr, "Failed to create thread %d\n", i);
            return 1;
        }
    }
    
    // Start timing
    struct timespec start_time, end_time;
    clock_gettime(CLOCK_MONOTONIC, &start_time);
    
    // Signal threads to start
    atomic_store(&start, 1);
    
    // Wait for threads
    for (int i = 0; i < num_threads; i++) {
        pthread_join(threads[i], NULL);
    }
    
    // End timing
    clock_gettime(CLOCK_MONOTONIC, &end_time);
    
    // Calculate statistics
    long total_allocations = 0;
    long total_failures = 0;
    for (int i = 0; i < num_threads; i++) {
        total_allocations += results[i].allocations;
        total_failures += results[i].failures;
    }
    
    double elapsed = (end_time.tv_sec - start_time.tv_sec) +
                    (end_time.tv_nsec - start_time.tv_nsec) / 1e9;
    
    printf("\nResults:\n");
    printf("  Total successful allocations: %ld\n", total_allocations);
    printf("  Total failures: %ld\n", total_failures);
    printf("  Time elapsed: %.3f seconds\n", elapsed);
    printf("  Allocations per second: %.0f\n", total_allocations / elapsed);
    printf("  Throughput: %.2f MB/s\n", 
           (total_allocations * (double)alloc_size) / elapsed / (1024 * 1024));
    
    free(threads);
    free(results);
    
    if (total_failures > 0) {
        fprintf(stderr, "WARNING: %ld allocation failures occurred\n", total_failures);
        return 1;
    }
    
    printf("\nTest PASSED\n");
    return 0;
}