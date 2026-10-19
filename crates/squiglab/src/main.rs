//! The window.

use squiglab::theme::{self, Theme};

fn main() -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(squiglab_core::product::APP_NAME)
            .with_app_id(squiglab_core::product::APP_ID)
            .with_inner_size([1440.0, 900.0]),
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
            Ok(Box::new(App::new()))
        }),
    )
    .map_err(|error| anyhow::anyhow!("{error}"))
}

/// The application: the window's state, and the frame it draws.
struct App {
    theme: Theme,
}

impl App {
    fn new() -> Self {
        Self { theme: Theme::Dark }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        theme::apply(ui.ctx(), self.theme);
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading(squiglab_core::product::APP_NAME);
            ui.label("The new interface is being built beside the old one.");
            for theme in [Theme::Light, Theme::Dark, Theme::Contrast] {
                let mut selected = self.theme == theme;
                if ui
                    .selectable_label(selected, format!("{theme:?}"))
                    .clicked()
                {
                    self.theme = theme;
                    selected = true;
                }
                let _ = selected;
            }
        });
    }
}
