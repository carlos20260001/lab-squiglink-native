fn main() {
    #[cfg(target_os = "linux")]
    {
        // On Linux systems where libxkbcommon-x11-0 is installed without the -dev package,
        // libxkbcommon-x11.so.0 is present in /usr/lib/x86_64-linux-gnu/ but libxkbcommon-x11.so
        // is missing. Create a symlink in target/lib and emit link-search if needed.
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let repo_root = std::path::Path::new(&manifest_dir)
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or_else(|| std::path::Path::new("."));
        let target_lib = repo_root.join("target/lib");
        let so_link = target_lib.join("libxkbcommon-x11.so");

        if !so_link.exists() {
            let candidates = [
                "/usr/lib/x86_64-linux-gnu/libxkbcommon-x11.so.0",
                "/usr/lib/libxkbcommon-x11.so.0",
            ];
            for cand in candidates {
                if std::path::Path::new(cand).exists() {
                    let _ = std::fs::create_dir_all(&target_lib);
                    let _ = std::os::unix::fs::symlink(cand, &so_link);
                    break;
                }
            }
        }

        if so_link.exists() {
            println!("cargo:rustc-link-search=native={}", target_lib.display());
        }
    }
}
