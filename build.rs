fn main() {
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-lib=dl");

        let out_dir = std::env::var("OUT_DIR").unwrap();
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let src = std::path::Path::new(&manifest_dir).join("csrc/printf_shim.c");
        let obj = std::path::Path::new(&out_dir).join("printf_shim.o");
        let lib = std::path::Path::new(&out_dir).join("libprintf_shim.a");

        let compiler = cc::Build::new()
            .file(&src)
            .get_compiler();
        let mut cmd = compiler.to_command();
        cmd.arg("-c").arg("-fPIC").arg(&src).arg("-o").arg(&obj);
        if std::env::var_os("CARGO_FEATURE_STDIO_KERNEL").is_some() {
            cmd.arg("-DSTDIO_KERNEL");
        }
        cmd.current_dir(&manifest_dir);
        let status = cmd.status().expect("compile printf_shim.c");
        assert!(status.success(), "printf_shim.c compile failed");

        let ar_status = std::process::Command::new("ar")
            .args(["cr", lib.to_str().unwrap(), obj.to_str().unwrap()])
            .status()
            .expect("ar failed");
        assert!(ar_status.success(), "ar failed");

        println!("cargo:rustc-link-search=native={}", out_dir);
        println!("cargo:rustc-link-arg=-Wl,--whole-archive");
        println!("cargo:rustc-link-lib=static=printf_shim");
        println!("cargo:rustc-link-arg=-Wl,--no-whole-archive");
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
