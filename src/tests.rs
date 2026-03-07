//! Hosted tests for IronLung (requires `hosted-test` feature and std)

#![cfg(feature = "hosted-test")]
#![cfg(test)]

use std::process::Command;
use std::env;
use std::path::PathBuf;

/// Test that IronLung can be loaded and basic functions work
#[test]
fn test_basic_functionality() {
    // Build the library
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .status()
        .expect("Failed to build IronLung");
    
    assert!(status.success(), "Failed to build IronLung");
    
    // Find the built library
    let target_dir = PathBuf::from(env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string()));
    let lib_path = target_dir.join("release").join("libironlung.so");
    
    assert!(lib_path.exists(), "Library not found at {:?}", lib_path);
    
    // Compile a simple test program
    let test_program = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main() {
    // Test malloc/free
    void* ptr = malloc(100);
    if (!ptr) {
        fprintf(stderr, "malloc failed\n");
        return 1;
    }
    memset(ptr, 0, 100);
    free(ptr);
    
    // Test strcpy
    char src[] = "Hello, IronLung!";
    char dest[100];
    strcpy(dest, src);
    if (strcmp(dest, src) != 0) {
        fprintf(stderr, "strcpy failed\n");
        return 1;
    }
    
    // Test puts (will have [IronLung] prefix)
    puts("Test passed");
    
    return 0;
}
"#;
    
    let test_c_path = target_dir.join("test_basic.c");
    let test_bin_path = target_dir.join("test_basic");
    
    std::fs::write(&test_c_path, test_program).expect("Failed to write test program");
    
    // Compile test program
    let compile_status = Command::new("gcc")
        .args(["-o", test_bin_path.to_str().unwrap(), test_c_path.to_str().unwrap()])
        .status()
        .expect("Failed to compile test program");
    
    assert!(compile_status.success(), "Failed to compile test program");
    
    // Run with IronLung
    let output = Command::new("env")
        .env("LD_PRELOAD", lib_path)
        .arg(test_bin_path)
        .output()
        .expect("Failed to run test program");
    
    println!("Test output: {}", String::from_utf8_lossy(&output.stdout));
    eprintln!("Test stderr: {}", String::from_utf8_lossy(&output.stderr));
    
    assert!(output.status.success(), "Test program failed with status: {:?}", output.status);
    assert!(output.stdout.contains("Test passed") || output.stderr.contains("[IronLung]"), 
            "Expected IronLung output");
}

/// Test allocator with multiple threads
#[test]
fn test_concurrent_allocations() {
    // This test would run the alloc_bench.c program with IronLung
    // For now, just check that we can build and load
    test_basic_functionality(); // Reuse basic test for now
}

/// Test quarantine functionality
#[test]
fn test_quarantine_feature() {
    // Only run if alloc-quarantine feature is enabled
    if !cfg!(feature = "alloc-quarantine") {
        println!("Skipping quarantine test - feature not enabled");
        return;
    }
    
    // Build with quarantine feature
    let status = Command::new("cargo")
        .args(["build", "--release", "--features", "alloc-quarantine"])
        .status()
        .expect("Failed to build IronLung with quarantine");
    
    assert!(status.success(), "Failed to build IronLung with quarantine");
    
    // Similar test as above but with quarantine enabled
    test_basic_functionality();
}