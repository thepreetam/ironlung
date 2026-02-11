/* I/O throughput benchmark for IronLung.
 * Measures read/write on pipe. Run with and without LD_PRELOAD.
 * Compile: gcc -O2 -o io_bench tests/io_bench.c
 * Run: ./io_bench [iterations] [buf_size]
 * With IronLung: LD_PRELOAD=./target/release/libironlung.so ./io_bench
 */
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

static unsigned long iterations = 100000;
static size_t buf_size = 4096;

int main(int argc, char **argv) {
    if (argc >= 2) iterations = strtoul(argv[1], NULL, 10);
    if (argc >= 3) buf_size = (size_t)strtoul(argv[2], NULL, 10);
    if (buf_size == 0) buf_size = 4096;

    int pipefd[2];
    if (pipe(pipefd) != 0) {
        perror("pipe");
        return 1;
    }

    char *buf = malloc(buf_size);
    if (!buf) {
        perror("malloc");
        return 1;
    }
    memset(buf, 'x', buf_size);

    struct timespec t0, t1;
    clock_gettime(CLOCK_MONOTONIC, &t0);
    for (unsigned long i = 0; i < iterations; i++) {
        ssize_t w = write(pipefd[1], buf, buf_size);
        if (w != (ssize_t)buf_size) {
            perror("write");
            return 1;
        }
        ssize_t r = read(pipefd[0], buf, buf_size);
        if (r != (ssize_t)buf_size) {
            perror("read");
            return 1;
        }
    }
    clock_gettime(CLOCK_MONOTONIC, &t1);
    free(buf);
    close(pipefd[0]);
    close(pipefd[1]);

    double sec = (t1.tv_sec - t0.tv_sec) + 1e-9 * (t1.tv_nsec - t0.tv_nsec);
    unsigned long total_ops = iterations * 2;
    printf("read+write ops: %lu in %.3f s (%.0f ops/s)\n", total_ops, sec, total_ops / sec);
    return 0;
}
