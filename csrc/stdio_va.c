/* va_list helpers for kernel-delegating vprintf (feature stdio-kernel).
 * Used by Rust to pull variadic args. */
#include <stdarg.h>

const char *stdio_va_arg_s(void *vap) {
    return va_arg(*(va_list *)vap, const char *);
}

void stdio_va_end(void *vap) {
    va_end(*(va_list *)vap);
}
