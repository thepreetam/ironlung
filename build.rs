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
        if std::env::var_os("CARGO_FEATURE_STDIO_KERNEL").is_some() {
            cmd.arg("-DSTDIO_KERNEL");
        }
        cmd.current_dir(&manifest_dir);
        let status = cmd.status().expect("compile printf_shim.c");
        assert!(status.success(), "printf_shim.c compile failed");

        // Link the .o directly (absolute path) so the linker always pulls in printf/fprintf/vprintf.
        let obj_abs = std::fs::canonicalize(&obj).expect("canonicalize printf_shim.o");
        println!("cargo:rustc-link-arg=-Wl,--export-dynamic");
        println!("cargo:rustc-link-arg=-Wl,-u,printf");
        println!("cargo:rustc-link-arg=-Wl,-u,fprintf");
        println!("cargo:rustc-link-arg=-Wl,-u,vprintf");
        println!("cargo:rustc-link-arg={}", obj_abs.display());
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
