//! The window.

use squiglab::app::App;
use squiglab::theme;

fn main() -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(squiglab_core::product::APP_NAME)
            .with_app_id(squiglab_core::product::APP_ID)
            .with_icon(window_icon())
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([430.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native(
        squiglab_core::product::APP_NAME,
        options,
        Box::new(|creation_context| {
            // The fonts must be registered before the first frame: `set_fonts`
            // is deferred until the next frame, so registering later would let
            // the first frame lay out text against egui's default font set.
            theme::register_fonts(&creation_context.egui_ctx);
            // The loaders that turn the product's mark into a texture.
            egui_extras::install_image_loaders(&creation_context.egui_ctx);
            Ok(Box::new(App::new()))
        }),
    )
    .map_err(|error| anyhow::anyhow!("{error}"))
}

fn window_icon() -> eframe::egui::IconData {
    let image = image::load_from_memory(squiglab_core::product::WINDOW_ICON_PNG)
        .expect("generated application icon is a valid PNG")
        .into_rgba8();
    let (width, height) = image.dimensions();
    eframe::egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}
