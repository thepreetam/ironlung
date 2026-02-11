//! ABI compliance check for libironlung.so.
//! Compares exported symbols against symbols.baseline.

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_so = Path::new(manifest_dir)
        .join("../..")
        .join("target/release/libironlung.so");

    let so_path = args.get(1).map(|s| s.as_str()).filter(|p| Path::new(p).exists());
    let so_path = so_path
        .or_else(|| workspace_so.exists().then(|| workspace_so.to_str().unwrap()))
        .unwrap_or_else(|| {
            eprintln!("error: libironlung.so not found (tried {:?} and {:?})", args.get(1), workspace_so);
            std::process::exit(1);
        });

    let baseline_path = Path::new(manifest_dir).join("symbols.baseline");
    let baseline = fs::read_to_string(&baseline_path).unwrap_or_else(|e| {
        eprintln!("error: could not read {:?}: {}", baseline_path, e);
        std::process::exit(1);
    });

    let expected: HashSet<&str> = baseline
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    let output = Command::new("nm")
        .args(["-D", "-g", so_path])
        .output()
        .unwrap_or_else(|e| {
            eprintln!("error: nm failed: {}", e);
            std::process::exit(1);
        });

    if !output.status.success() {
        eprintln!("error: nm failed: {}", String::from_utf8_lossy(&output.stderr));
        std::process::exit(1);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let actual: HashSet<&str> = stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let ty = parts[1];
                if ["T", "D", "B", "R", "W"].contains(&ty) {
                    return Some(parts[2]);
                }
            }
            None
        })
        .collect();

    let mut missing: Vec<&str> = expected.difference(&actual).copied().collect();
    missing.sort();

    let mut unexpected: Vec<&str> = actual.difference(&expected).copied().collect();
    unexpected.sort();

    let mut failed = false;

    if !missing.is_empty() {
        eprintln!("error: missing symbols: {}", missing.join(", "));
        failed = true;
    }

    if !unexpected.is_empty() {
        eprintln!("warn: unexpected symbols: {}", unexpected.join(", "));
    }

    if failed {
        std::process::exit(1);
    }

    println!("ABI check passed: {} symbols present", expected.len());
}
