/* printf/vprintf shim: validate format string, then delegate to system libc.
 * Rejects %n (format-string attack), limits format length.
 */
#define _GNU_SOURCE
#include <stdarg.h>
#include <stddef.h>
#include <stdio.h>
#include <dlfcn.h>

#define FORMAT_MAX 4096

static int validate_format(const char *fmt) {
    if (!fmt) return -1;
    size_t n = 0;
    while (fmt[n] && n < FORMAT_MAX) {
        if (fmt[n] == '%') {
            n++;
            if (!fmt[n]) return -1;
            if (fmt[n] == 'n') return -1; /* reject %n */
            if (fmt[n] != '%') n++; /* skip width/precision/modifier */
        } else {
            n++;
        }
    }
    return (fmt[n] == '\0' && n < FORMAT_MAX) ? 0 : -1;
}

#ifndef STDIO_KERNEL
typedef int (*vprintf_fn)(const char *, va_list);

static vprintf_fn get_real_vprintf(void) {
    static vprintf_fn f;
    if (!f) f = (vprintf_fn)dlsym(RTLD_NEXT, "vprintf");
    return f;
}

int vprintf(const char *format, va_list ap) {
    if (validate_format(format) != 0) return -1;
    vprintf_fn real = get_real_vprintf();
    if (!real) return -1;
    return real(format, ap);
}

int printf(const char *format, ...) {
    if (validate_format(format) != 0) return -1;
    vprintf_fn real = get_real_vprintf();
    if (!real) return -1;
    va_list ap;
    va_start(ap, format);
    int r = real(format, ap);
    va_end(ap);
    return r;
}
#else
extern int vprintf(const char *format, va_list ap);

int printf(const char *format, ...) {
    if (validate_format(format) != 0) return -1;
    va_list ap;
    va_start(ap, format);
    int r = vprintf(format, ap);
    va_end(ap);
    return r;
}
#endif

/* fprintf - validate and delegate */
typedef int (*vfprintf_fn)(FILE *, const char *, va_list);

int fprintf(FILE *stream, const char *format, ...) {
    if (validate_format(format) != 0) return -1;
    vfprintf_fn real = (vfprintf_fn)dlsym(RTLD_NEXT, "vfprintf");
    if (!real) return -1;
    va_list ap;
    va_start(ap, format);
    int r = real(stream, format, ap);
    va_end(ap);
    return r;
}

/* snprintf - validate format and size, delegate */
typedef int (*snprintf_fn)(char *, size_t, const char *, ...);
int snprintf(char *str, size_t size, const char *format, ...) {
    if (!str || size == 0 || size > FORMAT_MAX * 4 || validate_format(format) != 0)
        return -1;
    snprintf_fn real = (snprintf_fn)dlsym(RTLD_NEXT, "snprintf");
    if (!real) return -1;
    va_list ap;
    va_start(ap, format);
    int r = real(str, size, format, ap);
    va_end(ap);
    return r;
}

/* Referenced from Rust so the linker keeps printf/fprintf/vprintf/snprintf in the cdylib. */
void ironlung_printf_keep(void) {
    (void)&printf;
    (void)&fprintf;
#ifndef STDIO_KERNEL
    (void)&vprintf;
#endif
    (void)&snprintf;
}
