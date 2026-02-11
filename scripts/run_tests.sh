#!/bin/sh
# Note: cargo test is not supported for the main crate (no_std + panic=abort causes
# duplicate-core issues with -Z build-std). Use build + victim + LD_PRELOAD and
# scripts/doppelganger_fuzz.py for validation. This script runs build as a sanity check.
set -e
cd "$(dirname "$0")/.."
echo "IronLung: cargo test not supported (no_std + panic=abort). Use build + victim + fuzz for validation."
cargo build --release
echo "Build OK. Run: LD_PRELOAD=./target/release/libironlung.so ./victim"
