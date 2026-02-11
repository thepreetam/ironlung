/* Test fread/fwrite with kernel path (feature stdio-kernel-fread-fwrite).
 * Build: cargo build --release --features stdio-kernel-fread-fwrite -p ironlung
 * Run:  LD_PRELOAD=./target/release/libironlung.so ./fread_fwrite_kernel_test
 * Or:   gcc -o fread_fwrite_kernel_test tests/fread_fwrite_kernel_test.c && LD_PRELOAD=./target/release/libironlung.so ./fread_fwrite_kernel_test
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    const char *path = "/tmp/ironlung_fread_fwrite_test";
    const char *wbuf = "hello kernel fread/fwrite\n";
    size_t wlen = strlen(wbuf) + 1;
    char rbuf[64];
    FILE *f;
    size_t nw, nr;

    f = fopen(path, "w");
    if (!f) {
        perror("fopen w");
        return 1;
    }
    nw = fwrite(wbuf, 1, wlen, f);
    fclose(f);
    if (nw != wlen) {
        fprintf(stderr, "fwrite: wrote %zu, expected %zu\n", nw, wlen);
        return 1;
    }

    f = fopen(path, "r");
    if (!f) {
        perror("fopen r");
        return 1;
    }
    nr = fread(rbuf, 1, sizeof(rbuf), f);
    fclose(f);
    if (nr != wlen || memcmp(rbuf, wbuf, wlen) != 0) {
        fprintf(stderr, "fread: read %zu, expected %zu; or content mismatch\n", nr, wlen);
        return 1;
    }

    remove(path);
    printf("fread/fwrite kernel test OK\n");
    return 0;
}
