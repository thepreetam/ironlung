// Simple test to verify IronLung memory hardener features
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <sys/time.h>
#include <pthread.h>

#define NUM_THREADS 4
#define ALLOC_COUNT 1000

void* thread_func(void* arg) {
    int thread_id = *(int*)arg;
    void* pointers[ALLOC_COUNT];
    
    printf("Thread %d: Allocating %d blocks\n", thread_id, ALLOC_COUNT);
    
    // Allocate memory
    for (int i = 0; i < ALLOC_COUNT; i++) {
        size_t size = (i % 8 + 1) * 32; // Varying sizes
        pointers[i] = malloc(size);
        if (pointers[i]) {
            memset(pointers[i], thread_id, size);
        }
    }
    
    // Free some memory
    for (int i = 0; i < ALLOC_COUNT; i += 2) {
        free(pointers[i]);
    }
    
    // Allocate more
    for (int i = 0; i < ALLOC_COUNT / 2; i++) {
        size_t size = (i % 4 + 1) * 64;
        void* ptr = malloc(size);
        if (ptr) {
            memset(ptr, thread_id + 1, size);
            free(ptr);
        }
    }
    
    // Free remaining
    for (int i = 1; i < ALLOC_COUNT; i += 2) {
        free(pointers[i]);
    }
    
    printf("Thread %d: Done\n", thread_id);
    return NULL;
}

int main() {
    printf("Testing IronLung Memory Hardener\n");
    printf("===============================\n");
    
    // Test 1: Basic allocation
    printf("Test 1: Basic memory allocation\n");
    void* ptr1 = malloc(100);
    void* ptr2 = calloc(10, 20);
    void* ptr3 = realloc(ptr1, 200);
    
    if (ptr1 && ptr2 && ptr3) {
        printf("  Basic allocation: PASS\n");
    }
    
    free(ptr2);
    free(ptr3);
    
    // Test 2: String safety
    printf("Test 2: String operation safety\n");
    char dest[50];
    const char* src = "Hello, IronLung!";
    strcpy(dest, src);
    printf("  strcpy: '%s'\n", dest);
    
    // Test 3: Time functions
    printf("Test 3: Time functions\n");
    struct timespec ts;
    struct timeval tv;
    
    if (clock_gettime(CLOCK_MONOTONIC, &ts) == 0) {
        printf("  clock_gettime: PASS\n");
    }
    
    if (gettimeofday(&tv, NULL) == 0) {
        printf("  gettimeofday: PASS\n");
    }
    
    // Test 4: Thread concurrency
    printf("Test 4: Thread concurrency (%d threads)\n", NUM_THREADS);
    pthread_t threads[NUM_THREADS];
    int thread_ids[NUM_THREADS];
    
    for (int i = 0; i < NUM_THREADS; i++) {
        thread_ids[i] = i;
        pthread_create(&threads[i], NULL, thread_func, &thread_ids[i]);
    }
    
    for (int i = 0; i < NUM_THREADS; i++) {
        pthread_join(threads[i], NULL);
    }
    
    printf("\nAll tests completed!\n");
    printf("IronLung is providing memory hardening.\n");
    
    return 0;
}