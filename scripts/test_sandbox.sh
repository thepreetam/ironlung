#!/bin/bash
# End-to-end test for sandboxed getaddrinfo.
# Run on Linux with: ./scripts/test_sandbox.sh

set -e

cd "$(dirname "$0")/.."

echo "Building sandbox..."
cargo build --release --features sandbox -p ironlung 2>/dev/null || {
    echo "Build failed. Ensure you're on Linux with libseccomp-dev installed."
    exit 1
}
cargo build --release -p ironlung-sandbox 2>/dev/null || {
    echo "ironlung-sandbox build failed."
    exit 1
}

SANDBOX=$(pwd)/target/release/ironlung-sandbox
SO=$(pwd)/target/release/libironlung.so

if [[ ! -x "$SANDBOX" ]]; then
    echo "ironlung-sandbox not found at $SANDBOX"
    exit 1
fi

echo "Testing getaddrinfo via sandbox..."
# Use a small C program that calls getaddrinfo
out=$(echo '
#include <stdio.h>
#include <netdb.h>
#include <string.h>
struct addrinfo hints = {0}, *res = 0;
int main() {
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    int r = getaddrinfo("localhost", "http", &hints, &res);
    if (r) { printf("getaddrinfo failed: %d\n", r); return 1; }
    printf("getaddrinfo OK\n");
    freeaddrinfo(res);
    return 0;
}
' | gcc -x c -o /tmp/ga_test - - 2>/dev/null && \
    LD_PRELOAD="$SO" IRONLUNG_SANDBOX_PATH="$SANDBOX" /tmp/ga_test 2>&1)

if echo "$out" | grep -q "getaddrinfo OK"; then
    echo "PASS: getaddrinfo returned successfully"
else
    echo "FAIL: $out"
    exit 1
fi

echo "All sandbox tests passed."
