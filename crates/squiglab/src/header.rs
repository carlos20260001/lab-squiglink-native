//! The header across the top of the window.
//!
//! The reference's header carries its wordmark and the site's navigation. A
//! window has no site to navigate, so what is here is the wordmark, the
//! actions that apply to the whole window, and the theme.

use egui::{RichText, Ui};

use crate::app::App;
use crate::theme::{self, metrics, TypeStep};

/// The header's contents.
pub fn show(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();

    // --- the wordmark ---
    ui.add(
        egui::Image::new(egui::ImageSource::Bytes {
            uri: "bytes://mark.png".into(),
            bytes: squiglab_core::product::WINDOW_ICON_PNG.into(),
        })
        .fit_to_exact_size(egui::vec2(20.0, 20.0)),
    );
    ui.add_space(metrics::GAP);
    ui.label(
        RichText::new(squiglab_core::product::APP_NAME)
            .font(TypeStep::Logo.font_bold())
            .color(palette.on_accent),
    );

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.label(
            RichText::new(format!("{} curves", app.session.display_curves.len()))
                .font(TypeStep::Label.font())
                .color(palette.text_secondary),
        );
    });
}
