/* Pthread race condition stress test for IronLung.
 * Tests pthread delegation under heavy contention.
 * Compile: gcc -O2 -lpthread -o pthread_race tests/pthread_race.c
 * Run: ./pthread_race [threads] [iterations]
 * With IronLung: LD_PRELOAD=./target/release/libironlung.so ./pthread_race 8 1000
 */

#include <stdlib.h>
#include <stdio.h>
#include <pthread.h>
#include <string.h>
#include <time.h>
#include <errno.h>
#include <stdatomic.h>

static int num_threads = 8;
static long iterations = 1000;
static atomic_int start = 0;
static atomic_long shared_counter = 0;
static pthread_mutex_t global_mutex = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t global_cond = PTHREAD_COND_INITIALIZER;

// Helper function for thread creation test
static void *temp_thread_func(void *arg) {
    int *val = (int *)arg;
    *val = *val * 2;
    return NULL;
}

typedef struct {
    long id;
    long mutex_operations;
    long cond_operations;
    long errors;
} thread_result_t;

static void *pthread_thread(void *arg) {
    thread_result_t *result = (thread_result_t *)arg;
    result->mutex_operations = 0;
    result->cond_operations = 0;
    result->errors = 0;
    
    // Thread-local mutex and cond
    pthread_mutex_t local_mutex;
    pthread_cond_t local_cond;
    
    if (pthread_mutex_init(&local_mutex, NULL) != 0) {
        result->errors++;
        return NULL;
    }
    
    if (pthread_cond_init(&local_cond, NULL) != 0) {
        pthread_mutex_destroy(&local_mutex);
        result->errors++;
        return NULL;
    }
    
    // Wait for start signal
    while (!atomic_load(&start)) {
        // spin
    }
    
    for (long i = 0; i < iterations; i++) {
        // Test 1: Local mutex operations
        if (pthread_mutex_lock(&local_mutex) != 0) result->errors++;
        // Critical section
        int local_data = result->id * 1000 + i;
        if (pthread_mutex_unlock(&local_mutex) != 0) result->errors++;
        result->mutex_operations += 2;
        
        // Test 2: Global mutex contention
        if (pthread_mutex_lock(&global_mutex) != 0) result->errors++;
        // Critical section - increment shared counter
        long current = atomic_fetch_add(&shared_counter, 1);
        if (pthread_mutex_unlock(&global_mutex) != 0) result->errors++;
        result->mutex_operations += 2;
        
        // Test 3: Condition variable with local mutex
        if (pthread_mutex_lock(&local_mutex) != 0) result->errors++;
        if (pthread_cond_signal(&local_cond) != 0) result->errors++;
        if (pthread_mutex_unlock(&local_mutex) != 0) result->errors++;
        result->cond_operations++;
        result->mutex_operations += 2;
        
        // Test 4: Global condition variable (every 10 iterations)
        if (i % 10 == 0) {
            if (pthread_mutex_lock(&global_mutex) != 0) result->errors++;
            if (pthread_cond_signal(&global_cond) != 0) result->errors++;
            if (pthread_mutex_unlock(&global_mutex) != 0) result->errors++;
            result->cond_operations++;
            result->mutex_operations += 2;
        }
        
        // Test 5: Create and join a short-lived thread (every 100 iterations)
        if (i % 100 == 0) {
            pthread_t temp_thread;
            

            
            int temp_arg = result->id;
            if (pthread_create(&temp_thread, NULL, temp_thread_func, &temp_arg) != 0) {
                result->errors++;
            } else {
                if (pthread_join(temp_thread, NULL) != 0) {
                    result->errors++;
                }
            }
        }
    }
    
    // Cleanup
    pthread_mutex_destroy(&local_mutex);
    pthread_cond_destroy(&local_cond);
    
    return NULL;
}

int main(int argc, char *argv[]) {
    if (argc > 1) num_threads = atoi(argv[1]);
    if (argc > 2) iterations = atol(argv[2]);
    
    if (num_threads <= 0 || iterations <= 0) {
        fprintf(stderr, "Invalid arguments\n");
        return 1;
    }
    
    printf("Starting pthread race condition stress test:\n");
    printf("  Threads: %d\n", num_threads);
    printf("  Iterations per thread: %ld\n", iterations);
    printf("  Expected mutex operations: %ld\n", num_threads * iterations * 6L); // ~6 per iteration
    printf("  Expected cond operations: %ld\n", num_threads * iterations / 10L + num_threads * iterations);
    
    // Initialize global cond
    if (pthread_cond_init(&global_cond, NULL) != 0) {
        fprintf(stderr, "Failed to initialize global condition variable\n");
        return 1;
    }
    
    pthread_t *threads = malloc(num_threads * sizeof(pthread_t));
    thread_result_t *results = malloc(num_threads * sizeof(thread_result_t));
    
    if (!threads || !results) {
        fprintf(stderr, "Failed to allocate memory for threads/results\n");
        pthread_cond_destroy(&global_cond);
        return 1;
    }
    
    // Create threads
    for (int i = 0; i < num_threads; i++) {
        results[i].id = i;
        if (pthread_create(&threads[i], NULL, pthread_thread, &results[i]) != 0) {
            fprintf(stderr, "Failed to create thread %d\n", i);
            free(threads);
            free(results);
            pthread_cond_destroy(&global_cond);
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
    long total_mutex_ops = 0;
    long total_cond_ops = 0;
    long total_errors = 0;
    for (int i = 0; i < num_threads; i++) {
        total_mutex_ops += results[i].mutex_operations;
        total_cond_ops += results[i].cond_operations;
        total_errors += results[i].errors;
    }
    
    double elapsed = (end_time.tv_sec - start_time.tv_sec) +
                    (end_time.tv_nsec - start_time.tv_nsec) / 1e9;
    
    printf("\nResults:\n");
    printf("  Final shared counter value: %ld (expected: %ld)\n", 
           atomic_load(&shared_counter), (long)num_threads * iterations);
    printf("  Total mutex operations: %ld\n", total_mutex_ops);
    printf("  Total cond operations: %ld\n", total_cond_ops);
    printf("  Total errors: %ld\n", total_errors);
    printf("  Time elapsed: %.3f seconds\n", elapsed);
    printf("  Operations per second: %.0f\n", (total_mutex_ops + total_cond_ops) / elapsed);
    
    // Cleanup
    free(threads);
    free(results);
    pthread_cond_destroy(&global_cond);
    
    if (total_errors > 0) {
        fprintf(stderr, "\nFAILED: %ld pthread errors occurred\n", total_errors);
        return 1;
    }
    
    if (atomic_load(&shared_counter) != (long)num_threads * iterations) {
        fprintf(stderr, "\nFAILED: Shared counter mismatch. Possible race condition.\n");
        return 1;
    }
    
    printf("\nTest PASSED\n");
    return 0;
}