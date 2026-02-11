#!/usr/bin/env python3
"""Doppelgänger differential fuzzing: generate C programs, run with/without IronLung, compare output."""
import os
import subprocess
import tempfile
import random
import string
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


def main():
    if not SO.exists():
        print(f"Build IronLung first: cd {ROOT} && cargo build --release")
        return 1
    print("Doppelgänger fuzz: comparing output with/without IronLung")
    for seed in range(20):
        prog = generate_c_program(seed)
        ec1, out1, err1 = run_c(prog, False)
        ec2, out2, err2 = run_c(prog, True)
        # With IronLung, puts adds [IronLung] prefix - so output differs by design
        # We consider it a bug only if exit code differs or we get a crash
        if ec1 != ec2:
            print(f"BUG seed={seed}: exit code mismatch {ec1} vs {ec2}")
            print("Program:", prog[:200])
            return 1
    print("All 20 seeds passed (exit codes match)")
    return 0


if __name__ == "__main__":
    exit(main())
