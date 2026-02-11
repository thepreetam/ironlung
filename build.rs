fn main() {
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-lib=dl");

        let out_dir = std::env::var("OUT_DIR").unwrap();
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let src = std::path::Path::new(&manifest_dir).join("csrc/printf_shim.c");
        let obj = std::path::Path::new(&out_dir).join("printf_shim.o");

        let compiler = cc::Build::new()
            .file(&src)
            .get_compiler();
        let mut cmd = compiler.to_command();
        cmd.arg("-c")
            .arg("-fPIC")
            .arg("-fvisibility=default")
            .arg(&src)
            .arg("-o")
            .arg(&obj);
        if std::env::var_os("CARGO_FEATURE_STDIO_KERNEL").is_some()
            && std::env::var_os("CARGO_FEATURE_STDIO_LIBC").is_none()
        {
            cmd.arg("-DSTDIO_KERNEL");
        }
        cmd.current_dir(&manifest_dir);
        let status = cmd.status().expect("compile printf_shim.c");
        assert!(status.success(), "printf_shim.c compile failed");

        // Link the .o via -Wl, so the linker sees it.
        // On x86_64 we also pass a version script (GLIBC_2.2.5) for LD_PRELOAD compatibility.
        // On other arches (e.g. arm64) we skip the version script: rustc injects its own script for cdylib,
        // and combining it with ours triggers "anonymous version tag cannot be combined with other version tags".
        let obj_abs = std::fs::canonicalize(&obj).expect("canonicalize printf_shim.o");
        let target = std::env::var("TARGET").unwrap_or_default();
        let use_version_script = target.starts_with("x86_64-");

        if use_version_script {
            let baseline_path = std::path::Path::new(&manifest_dir).join("crates/ironlung-abi-check/symbols.baseline");
            let baseline_content = std::fs::read_to_string(&baseline_path).unwrap_or_default();
            let glibc_symbols: Vec<&str> = baseline_content
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .collect();
            let glibc_body = glibc_symbols.iter().map(|s| format!("  {};", s)).collect::<Vec<_>>().join("\n");
            let version_script = std::path::Path::new(&out_dir).join("ironlung.ver");
            std::fs::write(
                &version_script,
                format!("GLIBC_2.2.5 {{\n global:\n{}\n local: *;\n}};\n", glibc_body),
            )
            .expect("write version script");
            let version_script_abs = std::fs::canonicalize(&version_script).expect("canonicalize .ver");
            println!("cargo:rustc-link-arg=-Wl,--version-script={}", version_script_abs.display());
        }

        println!("cargo:rustc-link-arg=-Wl,-u,printf");
        println!("cargo:rustc-link-arg=-Wl,-u,fprintf");
        println!("cargo:rustc-link-arg=-Wl,-u,vprintf");
        println!("cargo:rustc-link-arg=-Wl,-u,snprintf");
        println!("cargo:rustc-link-arg=-Wl,{}", obj_abs.display());
    }

    if std::env::var_os("CARGO_FEATURE_ALLOC_CACHE").is_some() {
        #[cfg(target_os = "linux")]
        {
            cc::Build::new()
                .file("csrc/alloc_cache.c")
                .compile("alloc_cache");
        }
    }

    if std::env::var_os("CARGO_FEATURE_STDIO_KERNEL").is_some() {
        #[cfg(target_os = "linux")]
        {
            cc::Build::new()
                .file("csrc/stdio_va.c")
                .compile("stdio_va");
        }
    }
}
