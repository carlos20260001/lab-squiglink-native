// Renders the product's icons from its mark, so that every interface draws
// the same one and the packaging can ship them.
//
// The mark and the manifest belong to the program rather than to any picture
// of it, so they are rendered here and handed on as bytes.

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
    println!(
        "cargo:rustc-env=CRINGRAPH_WINDOW_ICON_ICO={}",
        icons.ico.display()
    );
}
