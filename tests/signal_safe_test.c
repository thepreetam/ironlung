/*
 * Test that IronLung's memcpy/memmove are safe to use from a signal handler.
 * Contract: only memcpy and memmove are guaranteed async-signal-safe.
 * This program: set handler for SIGUSR1; in handler, use memcpy then _exit(0).
 * Run: gcc -o signal_safe_test tests/signal_safe_test.c && LD_PRELOAD=./target/release/libironlung.so ./signal_safe_test
 */
#include <signal.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static char src[8] = "ok";
static char dst[8];

static void handler(int sig) {
    (void)sig;
    memcpy(dst, src, sizeof(src));
    _exit(0);
}

int main(void) {
    struct sigaction sa = {0};
    sa.sa_handler = handler;
    sigemptyset(&sa.sa_mask);
    sa.sa_flags = 0;
    if (sigaction(SIGUSR1, &sa, NULL) != 0)
        return 1;
    raise(SIGUSR1);
    return 1; /* never reached */
}
