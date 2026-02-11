#!/bin/bash
# Run glibc tests with LD_PRELOAD=libironlung.so
# Requires glibc source and built tests.
# Usage: ./scripts/run_glibc_tests.sh [glibc_build_dir]

set -e
GLIBC_DIR="${1:-/usr/src/glibc}"
SO="$(pwd)/target/release/libironlung.so"

if [ ! -f "$SO" ]; then
    echo "Build IronLung first: cargo build --release" >&2
    exit 1
fi

if [ ! -d "$GLIBC_DIR" ]; then
    echo "Glibc source/build dir not found: $GLIBC_DIR" >&2
    echo "Usage: $0 [glibc_build_dir]" >&2
    exit 1
fi

echo "Running glibc tests with LD_PRELOAD=$SO"
cd "$GLIBC_DIR"
export LD_PRELOAD="$SO"
make check 2>&1 | tee ironlung_glibc_test.log || true
