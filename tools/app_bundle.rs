#[allow(dead_code)]
#[path = "../src/branding.rs"]
mod branding;

use branding::{IconSet, Manifest, Result};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

fn main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 4 || args[0] != "package" {
        eprintln!("Usage: app-bundle package <linux|windows|macos> <binary> <output-directory>");
        std::process::exit(2);
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest_path = root.join("packaging/app.toml");
    let manifest = branding::load_manifest(&manifest_path)?;
    let icon_path = branding::resolve_icon_path(&manifest_path, &manifest);
    let output = PathBuf::from(&args[3]);
    match args[1].as_str() {
        "linux" => package_linux(&root, &manifest, &icon_path, Path::new(&args[2]), &output)?,
        "windows" => package_windows(&root, &manifest, &icon_path, Path::new(&args[2]), &output)?,
        "macos" => package_macos(&root, &manifest, &icon_path, Path::new(&args[2]), &output)?,
        platform => return Err(format!("unsupported packaging platform: {platform}").into()),
    }
    println!("Package created at {}", output.display());
    Ok(())
}

fn prepare_package(icon_path: &Path, output: &Path) -> Result<IconSet> {
    fs::create_dir_all(output)?;
    let assets = output.join("icons");
    let icons = branding::render_icons(icon_path, &assets)?;
    fs::copy(icon_path, assets.join("cringraph-app-icon.svg"))?;
    Ok(icons)
}

fn reset_package_directory(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(format!(
                "refusing to replace symlink package path {}",
                path.display()
            )
            .into());
        }
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(path)?,
        Ok(_) => fs::remove_file(path)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    fs::create_dir_all(path)?;
    Ok(())
}

fn remove_icon_containers(icons: &IconSet) -> Result<()> {
    fs::remove_file(&icons.ico)?;
    fs::remove_file(&icons.icns)?;
    Ok(())
}

fn copy_app_files(root: &Path, binary: &Path, destination: &Path, binary_name: &str) -> Result<()> {
    fs::create_dir_all(destination)?;
    fs::copy(binary, destination.join(binary_name))?;
    copy_directory(&root.join("data"), &destination.join("data"))?;
    copy_directory(&root.join("data_hp"), &destination.join("data_hp"))?;
    copy_directory(
        &root.join("config-examples"),
        &destination.join("config-examples"),
    )?;
    fs::copy(root.join("README.md"), destination.join("README.md"))?;
    fs::copy(root.join("LICENSE"), destination.join("LICENSE"))?;
    Ok(())
}

fn copy_directory(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_directory(&source_path, &destination_path)?;
        } else {
            fs::copy(source_path, destination_path)?;
        }
    }
    Ok(())
}

fn package_linux(
    root: &Path,
    manifest: &Manifest,
    icon_path: &Path,
    binary: &Path,
    output: &Path,
) -> Result<()> {
    let app_dir = output.join("CrinGraph");
    reset_package_directory(&app_dir)?;
    copy_app_files(root, binary, &app_dir, &manifest.application.binary_name)?;
    let icons = prepare_package(icon_path, &app_dir)?;
    let app_id = &manifest.application.identifier;
    let desktop_dir = app_dir.join("share/applications");
    fs::create_dir_all(&desktop_dir)?;
    fs::write(
        desktop_dir.join(format!("{app_id}.desktop")),
        format!(
            "[Desktop Entry]\nType=Application\nName={}\nComment={}\nExec={}\nIcon={}\nTerminal=false\nCategories={}\nStartupWMClass={}\n",
            manifest.application.name,
            manifest.application.description,
            manifest.application.binary_name,
            app_id,
            manifest.application.category,
            manifest.application.startup_wm_class,
        ),
    )?;
    let icon_root = app_dir.join("share/icons/hicolor");
    for (size, path) in &icons.pngs {
        let directory = icon_root.join(format!("{size}x{size}/apps"));
        fs::create_dir_all(&directory)?;
        fs::copy(path, directory.join(format!("{app_id}.png")))?;
    }
    let scalable = icon_root.join("scalable/apps");
    fs::create_dir_all(&scalable)?;
    fs::copy(icon_path, scalable.join(format!("{app_id}.svg")))?;
    remove_icon_containers(&icons)?;
    Ok(())
}

fn package_windows(
    root: &Path,
    manifest: &Manifest,
    icon_path: &Path,
    binary: &Path,
    output: &Path,
) -> Result<()> {
    let app_dir = output.join("CrinGraph");
    reset_package_directory(&app_dir)?;
    copy_app_files(
        root,
        binary,
        &app_dir,
        &format!("{}.exe", manifest.application.binary_name),
    )?;
    let icons = prepare_package(icon_path, &app_dir)?;
    fs::copy(&icons.ico, app_dir.join("cringraph.ico"))?;
    remove_icon_containers(&icons)?;
    let script = root.join("scripts/create_shortcut.ps1");
    let status = Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(script)
        .arg("-TargetDirectory")
        .arg(&app_dir)
        .arg("-ShortcutName")
        .arg(&manifest.application.name)
        .arg("-ExecutableName")
        .arg(format!("{}.exe", manifest.application.binary_name))
        .arg("-Description")
        .arg(&manifest.application.description)
        .status()?;
    if !status.success() {
        return Err(format!("Windows shortcut generation failed: {status}").into());
    }
    Ok(())
}

fn package_macos(
    root: &Path,
    manifest: &Manifest,
    icon_path: &Path,
    binary: &Path,
    output: &Path,
) -> Result<()> {
    let bundle = output.join(&manifest.application.macos_bundle_name);
    reset_package_directory(&bundle)?;
    let contents = bundle.join("Contents");
    let macos = contents.join("MacOS");
    let resources = contents.join("Resources");
    fs::create_dir_all(&macos)?;
    fs::create_dir_all(&resources)?;
    fs::copy(binary, macos.join(&manifest.application.binary_name))?;
    copy_directory(&root.join("data"), &resources.join("data"))?;
    copy_directory(&root.join("data_hp"), &resources.join("data_hp"))?;
    copy_directory(
        &root.join("config-examples"),
        &resources.join("config-examples"),
    )?;
    fs::copy(root.join("README.md"), resources.join("README.md"))?;
    fs::copy(root.join("LICENSE"), resources.join("LICENSE"))?;
    let icons = prepare_package(icon_path, &resources)?;
    fs::copy(&icons.icns, resources.join("CrinGraph.icns"))?;
    remove_icon_containers(&icons)?;
    fs::write(contents.join("PkgInfo"), b"APPL????")?;
    fs::write(
        contents.join("Info.plist"),
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict>\n<key>CFBundleName</key><string>{}</string>\n<key>CFBundleDisplayName</key><string>{}</string>\n<key>CFBundleIdentifier</key><string>{}</string>\n<key>CFBundleExecutable</key><string>{}</string>\n<key>CFBundleIconFile</key><string>{}</string>\n<key>CFBundlePackageType</key><string>APPL</string>\n<key>CFBundleShortVersionString</key><string>{}</string>\n<key>CFBundleVersion</key><string>{}</string>\n</dict></plist>\n",
            xml_escape(&manifest.application.name),
            xml_escape(&manifest.application.name),
            xml_escape(&manifest.application.identifier),
            xml_escape(&manifest.application.binary_name),
            "CrinGraph.icns",
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_VERSION"),
        ),
    )?;
    Ok(())
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
