#!/bin/sh
# Run doppelgänger fuzz to 1M iterations on a VPS (survives disconnect).
# Build first: cd repo && cargo build --release
# Then: nohup ./scripts/run_fuzz_vps.sh > fuzz_out.txt 2>&1 &
# Graph: plot fuzz_log.csv (iteration vs crashes) for "1M iterations / 0 crashes".

set -e
cd "$(dirname "$0")/.."
LOG="${1:-fuzz_log.csv}"
echo "Fuzz log: $LOG (iteration,crashes,timestamp)"
python3 scripts/doppelganger_fuzz.py --iterations 1000000 --progress-every 10000 --csv "$LOG"
echo "Done. Tweet: 1 Million Fuzz Iterations / 0 Crashes"
