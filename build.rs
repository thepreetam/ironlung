fn main() {
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-lib=dl");
        cc::Build::new()
            .file("csrc/printf_shim.c")
            .compile("printf_shim");
    }
}
