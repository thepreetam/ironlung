#!/bin/sh
# Compare errno on error paths: run same program with and without LD_PRELOAD.
# Exit 0 if errno matches (IronLung matches glibc).
set -e
cd "$(dirname "$0")/.."
SO="$(pwd)/target/release/libironlung.so"
if [ ! -f "$SO" ]; then
    echo "Build first: cargo build --release" >&2
    exit 1
fi
gcc -o /tmp/errno_test tests/errno_test.c
E_GLIBC=$(/tmp/errno_test 2>/dev/null; echo $?)
E_IRON=$(LD_PRELOAD="$SO" /tmp/errno_test 2>/dev/null; echo $?)
# Normalize: we expect EBADF (9)
if [ "$E_GLIBC" = "$E_IRON" ]; then
    echo "errno match: $E_GLIBC"
    exit 0
fi
echo "errno mismatch: glibc=$E_GLIBC ironlung=$E_IRON" >&2
exit 1
