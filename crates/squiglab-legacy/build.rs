#[allow(dead_code)]
#[path = "src/branding.rs"]
mod branding;

use std::{env, path::PathBuf};

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let manifest_path = root.join("packaging/app.toml");
    println!("cargo:rerun-if-changed={}", manifest_path.display());

    let manifest = branding::load_manifest(&manifest_path).expect("valid packaging/app.toml");
    let icon_path = branding::resolve_icon_path(&manifest_path, &manifest);
    println!("cargo:rerun-if-changed={}", icon_path.display());

    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"));
    let icons = branding::render_icons(&icon_path, &output).expect("render application icons");
    println!(
        "cargo:rustc-env=CRINGRAPH_APP_NAME={}",
        manifest.application.name
    );
    println!(
        "cargo:rustc-env=CRINGRAPH_APP_ID={}",
        manifest.application.identifier
    );
    println!(
        "cargo:rustc-env=CRINGRAPH_WINDOW_ICON_PNG={}",
        icons.png_128.display()
    );

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && env::var("HOST").is_ok_and(|host| host.contains("windows"))
    {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon(icons.ico.to_str().expect("ICO path is valid UTF-8"))
            .set("FileDescription", &manifest.application.description)
            .set("ProductName", &manifest.application.name)
            .set("InternalName", &manifest.application.binary_name)
            .set(
                "OriginalFilename",
                &format!("{}.exe", manifest.application.binary_name),
            )
            .set(
                "ProductVersion",
                &env::var("CARGO_PKG_VERSION").expect("package version"),
            );
        resource
            .compile()
            .expect("embed Windows application resources");
    } else if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!(
            "cargo:warning=Skipping Windows version resources when cross-compiling from a non-Windows host"
        );
    }
}
