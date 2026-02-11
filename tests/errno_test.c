/*
 * Trigger error paths and report errno. Used to compare IronLung errno
 * with glibc (run with and without LD_PRELOAD; exit code = errno).
 * Run: gcc -o errno_test tests/errno_test.c
 *      ./errno_test; echo "glibc errno=$?"
 *      LD_PRELOAD=./target/release/libironlung.so ./errno_test; echo "ironlung errno=$?"
 */
#include <errno.h>
#include <stdio.h>
#include <unistd.h>

int main(void) {
    char buf[1];
    /* read from invalid fd -> EBADF (9) */
    if (read(-1, buf, 1) == -1) {
        return errno;
    }
    return 0;
}
