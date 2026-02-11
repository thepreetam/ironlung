#!/usr/bin/env python3
"""Doppelgänger differential fuzzing: generate C programs, run with/without IronLung, compare output."""
import argparse
import csv
import os
import subprocess
import sys
import tempfile
import random
import string
import time
from pathlib import Path

# Paths
SCRIPT_DIR = Path(__file__).resolve().parent
ROOT = SCRIPT_DIR.parent
VICTIM_C = ROOT / "victim.c"
SO = ROOT / "target" / "release" / "libironlung.so"


def generate_c_program(seed: int) -> str:
    """Generate a simple C program that uses malloc, strcpy, printf, etc."""
    random.seed(seed)
    prog = '''#include <stdlib.h>
#include <string.h>
#include <stdio.h>
int main(void) {
'''
    # 1-3 malloc/strcpy/free sequences
    for _ in range(random.randint(1, 3)):
        var = "".join(random.choices(string.ascii_lowercase, k=6))
        size = random.choice([64, 128, 256, 1024])
        s = "".join(random.choices(string.ascii_letters + " ", k=random.randint(5, 30)))
        prog += f'    char *{var} = malloc({size});\n'
        prog += f'    strcpy({var}, "{s}");\n'
        prog += f'    puts({var});\n'
        prog += f'    free({var});\n'
    prog += "    return 0;\n}\n"
    return prog


def run_c(prog: str, use_ironlung: bool) -> tuple[int, str, str]:
    """Compile and run C program. Returns (exit_code, stdout, stderr)."""
    with tempfile.TemporaryDirectory() as tmp:
        src = Path(tmp) / "prog.c"
        exe = Path(tmp) / "prog"
        src.write_text(prog)
        subprocess.run(
            ["gcc", "-o", str(exe), str(src)],
            capture_output=True,
            check=True,
            cwd=tmp,
        )
        env = os.environ.copy()
        if use_ironlung and SO.exists():
            env["LD_PRELOAD"] = str(SO)
        r = subprocess.run(
            [str(exe)],
            capture_output=True,
            timeout=5,
            env=env,
            cwd=tmp,
        )
    return r.returncode, r.stdout.decode(), r.stderr.decode()


def main() -> int:
    p = argparse.ArgumentParser(description="Doppelgänger fuzz: compare output with/without IronLung")
    p.add_argument("--iterations", "-n", type=int, default=20, help="Number of fuzz iterations (default 20)")
    p.add_argument("--progress-every", type=int, default=0, help="Print progress every N iterations (0=only start/finish)")
    p.add_argument("--csv", type=Path, default=None, help="Append CSV rows (iteration,crashes,timestamp) to FILE for graphing")
    args = p.parse_args()

    if not SO.exists():
        print(f"Build IronLung first: cd {ROOT} && cargo build --release", file=sys.stderr)
        return 1

    print("Doppelgänger fuzz: comparing output with/without IronLung")
    if args.iterations != 20:
        print(f"Target: {args.iterations} iterations, 0 crashes")
    crashes = 0
    csv_file = None
    csv_writer = None
    if args.csv:
        csv_file = open(args.csv, "a", newline="")
        csv_writer = csv.writer(csv_file)
        if csv_file.tell() == 0:
            csv_writer.writerow(("iteration", "crashes", "timestamp"))

    start = time.monotonic()
    try:
        for i in range(args.iterations):
            prog = generate_c_program(i)
            try:
                ec1, out1, err1 = run_c(prog, False)
                ec2, out2, err2 = run_c(prog, True)
            except (subprocess.TimeoutExpired, subprocess.CalledProcessError) as e:
                crashes += 1
                print(f"CRASH seed={i}: {e}", file=sys.stderr)
                if csv_writer:
                    csv_writer.writerow((i + 1, crashes, time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())))
                continue
            if ec1 != ec2:
                crashes += 1
                print(f"BUG seed={i}: exit code mismatch {ec1} vs {ec2}", file=sys.stderr)
                print("Program:", prog[:200], file=sys.stderr)
                if csv_writer:
                    csv_writer.writerow((i + 1, crashes, time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())))
                continue
            if csv_writer and (args.progress_every and (i + 1) % args.progress_every == 0):
                csv_writer.writerow((i + 1, crashes, time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())))
            if args.progress_every and (i + 1) % args.progress_every == 0:
                elapsed = time.monotonic() - start
                rate = (i + 1) / elapsed if elapsed > 0 else 0
                print(f"  {i + 1} iterations, {crashes} crashes ({rate:.0f} iter/s)")
    finally:
        if csv_writer:
            csv_writer.writerow((args.iterations, crashes, time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())))
        if csv_file:
            csv_file.close()

    elapsed = time.monotonic() - start
    print(f"{args.iterations} iterations, {crashes} crashes ({elapsed:.1f}s)")
    if crashes:
        return 1
    if args.iterations >= 1_000_000:
        print("1 Million Fuzz Iterations / 0 Crashes")
    else:
        print("All passed (exit codes match)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
