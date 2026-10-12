#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;

use app::CrinGraphApp;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    configure_platform_identity()?;
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let shared_state = arguments
        .iter()
        .find(|argument| argument.starts_with("cringraph://state/"))
        .cloned();
    let debug_hooks = arguments.iter().any(|argument| argument == "--debug")
        || std::env::var_os("CRINGRAPH_DEBUG").is_some();
    if debug_hooks {
        eprintln!("CrinGraph debug hooks enabled (egui widget hover diagnostics).");
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(env!("CRINGRAPH_APP_NAME"))
            .with_app_id(env!("CRINGRAPH_APP_ID"))
            .with_icon(window_icon())
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([860.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        env!("CRINGRAPH_APP_NAME"),
        options,
        Box::new(move |_creation_context| {
            Ok(Box::new(CrinGraphApp::with_debug_hooks(
                shared_state,
                debug_hooks,
            )))
        }),
    )?;
    Ok(())
}

fn window_icon() -> eframe::egui::IconData {
    let image = image::load_from_memory(include_bytes!(env!("CRINGRAPH_WINDOW_ICON_PNG")))
        .expect("generated application icon is a valid PNG")
        .into_rgba8();
    let (width, height) = image.dimensions();
    eframe::egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

fn configure_platform_identity() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    configure_macos_bundle_directory()?;

    #[cfg(windows)]
    {
        set_windows_app_user_model_id()?;
        configure_portable_bundle_directory()?;
    }

    #[cfg(target_os = "linux")]
    configure_portable_bundle_directory()?;

    Ok(())
}

#[cfg(any(target_os = "linux", windows))]
fn configure_portable_bundle_directory() -> Result<(), Box<dyn std::error::Error>> {
    let executable = std::env::current_exe()?;
    if let Some(directory) = executable.parent() {
        if directory.join("data/phone_book.json").is_file() {
            std::env::set_current_dir(directory)?;
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn configure_macos_bundle_directory() -> Result<(), Box<dyn std::error::Error>> {
    let executable = std::env::current_exe()?;
    let Some(macos_directory) = executable.parent() else {
        return Ok(());
    };
    let Some(contents_directory) = macos_directory.parent() else {
        return Ok(());
    };
    let is_app_bundle = macos_directory
        .file_name()
        .is_some_and(|name| name == "MacOS")
        && contents_directory
            .file_name()
            .is_some_and(|name| name == "Contents")
        && contents_directory.parent().is_some_and(|bundle| {
            bundle
                .extension()
                .is_some_and(|extension| extension == "app")
        });
    if is_app_bundle {
        let resources = contents_directory.join("Resources");
        if !resources.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "the CrinGraph app bundle is missing Contents/Resources",
            )
            .into());
        }
        std::env::set_current_dir(resources)?;
    }
    Ok(())
}

#[cfg(windows)]
fn set_windows_app_user_model_id() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "shell32")]
    extern "system" {
        fn SetCurrentProcessExplicitAppUserModelID(app_id: *const u16) -> i32;
    }

    let app_id = std::ffi::OsStr::new(env!("CRINGRAPH_APP_ID"))
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result = unsafe { SetCurrentProcessExplicitAppUserModelID(app_id.as_ptr()) };
    if result < 0 {
        return Err(std::io::Error::from_raw_os_error(result).into());
    }
    Ok(())
}
