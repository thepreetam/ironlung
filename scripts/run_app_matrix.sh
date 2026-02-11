#!/bin/bash
# Real-world application matrix: nginx, bash, python, redis.
# Quick smoke test: does the app start with LD_PRELOAD?

set -e
ROOT="${WORKSPACE:-$(cd "$(dirname "$0")/.." && pwd)}"
SO="$ROOT/target/release/libironlung.so"

if [ ! -f "$SO" ]; then
    echo "Build IronLung first: cargo build --release (checked $SO)" >&2
    exit 1
fi

export LD_PRELOAD="$SO"
PASS=0
FAIL=0

run_test() {
    local name="$1"
    local cmd="$2"
    echo -n "Testing $name... "
    if eval "$cmd" >/dev/null 2>&1; then
        echo "PASS"
        PASS=$((PASS + 1))
        return 0
    else
        echo "FAIL"
        FAIL=$((FAIL + 1))
        return 1
    fi
}

echo "=== IronLung Application Matrix ==="

run_test "bash --version" "bash --version" || true
run_test "bash -c 'echo ok'" "bash -c 'echo ok'" || true

if command -v python3 &>/dev/null; then
    run_test "python3 -c 'print(1)'" "python3 -c 'print(1)'" || true
fi

if command -v redis-server &>/dev/null; then
    run_test "redis-server --version" "redis-server --version" || true
fi

if command -v nginx &>/dev/null; then
    run_test "nginx -v" "nginx -v" || true
fi

echo "---"
echo "PASS: $PASS  FAIL: $FAIL"
