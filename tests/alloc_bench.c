/* Allocation benchmark for IronLung.
 * Compile: gcc -O2 -lpthread -o alloc_bench tests/alloc_bench.c
 * Run: ./alloc_bench [iterations] [threads]
 * With IronLung: LD_PRELOAD=./target/release/libironlung.so ./alloc_bench 100000 4
 */
#include <stdlib.h>
#include <stdio.h>
#include <pthread.h>
#include <string.h>
#include <time.h>
#include <errno.h>

static unsigned long iterations = 100000;
static int num_threads = 4;
static volatile int start = 0;

static void *alloc_thread(void *arg) {
    (void)arg;
    while (!start) {}
    void *ptrs[64];
    for (unsigned long i = 0; i < iterations; i++) {
        for (int j = 0; j < 64; j++) {
            ptrs[j] = malloc(64 + (i + j) % 256);
            if (ptrs[j]) memset(ptrs[j], 0, 64);
        }
        for (int j = 0; j < 64; j++) {
            free(ptrs[j]);
        }
    }
    return NULL;
}

int main(int argc, char **argv) {
    if (argc >= 2) iterations = strtoul(argv[1], NULL, 10);
    if (argc >= 3) num_threads = atoi(argv[2]);
    if (num_threads < 1) num_threads = 1;

    pthread_t *threads = malloc((size_t)num_threads * sizeof(pthread_t));
    if (!threads) {
        perror("malloc threads");
        return 1;
    }

    struct timespec t0, t1;
    clock_gettime(CLOCK_MONOTONIC, &t0);
    for (int i = 0; i < num_threads; i++) {
        if (pthread_create(&threads[i], NULL, alloc_thread, NULL) != 0) {
            perror("pthread_create");
            return 1;
        }
    }
    start = 1;
    for (int i = 0; i < num_threads; i++) {
        pthread_join(threads[i], NULL);
    }
    clock_gettime(CLOCK_MONOTONIC, &t1);
    free(threads);

    double sec = (t1.tv_sec - t0.tv_sec) + 1e-9 * (t1.tv_nsec - t0.tv_nsec);
    unsigned long total = iterations * 64 * 2 * (unsigned long)num_threads;
    printf("alloc+free ops: %lu in %.3f s (%.0f ops/s)\n", total, sec, total / sec);
    return 0;
}
