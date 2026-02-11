/* Minimal test for native pthread mutex + cond under LD_PRELOAD.
 * Build: gcc -o pthread_mutex_cond_test tests/pthread_mutex_cond_test.c -lpthread
 * Run: LD_PRELOAD=./target/release/libironlung.so ./pthread_mutex_cond_test
 */
#include <pthread.h>
#include <stdio.h>
#include <string.h>

static pthread_mutex_t m = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t c = PTHREAD_COND_INITIALIZER;
static int ready;

static void *worker(void *arg) {
    (void)arg;
    pthread_mutex_lock(&m);
    ready = 1;
    pthread_cond_signal(&c);
    pthread_mutex_unlock(&m);
    return NULL;
}

int main(void) {
    pthread_t t;
    if (pthread_mutex_init(&m, NULL) != 0) {
        fprintf(stderr, "mutex_init failed\n");
        return 1;
    }
    if (pthread_cond_init(&c, NULL) != 0) {
        fprintf(stderr, "cond_init failed\n");
        return 1;
    }
    ready = 0;
    if (pthread_create(&t, NULL, worker, NULL) != 0) {
        fprintf(stderr, "pthread_create failed\n");
        return 1;
    }
    pthread_mutex_lock(&m);
    while (!ready)
        pthread_cond_wait(&c, &m);
    pthread_mutex_unlock(&m);
    if (pthread_join(t, NULL) != 0) {
        fprintf(stderr, "pthread_join failed\n");
        return 1;
    }
    pthread_cond_destroy(&c);
    pthread_mutex_destroy(&m);
    printf("pthread mutex+cond test OK\n");
    return 0;
}
