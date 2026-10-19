// Embeds the product's resources in the Windows executable.
//
// The mark itself is the program's: it is rendered by the core's build script
// and handed over as bytes, so the executable carries the same icon every
// interface does.

use std::env;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && env::var("HOST").is_ok_and(|host| host.contains("windows"))
    {
        let manifest = squiglab_core::branding::load_manifest(std::path::Path::new(
            squiglab_core::product::MANIFEST_PATH,
        ))
        .expect("valid packaging/app.toml");
        let icon = std::env::temp_dir().join(format!("squiglab-{}.ico", std::process::id()));
        std::fs::write(&icon, squiglab_core::product::WINDOW_ICON_ICO)
            .expect("the icon is written");

        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon(icon.to_str().expect("ICO path is valid UTF-8"))
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
