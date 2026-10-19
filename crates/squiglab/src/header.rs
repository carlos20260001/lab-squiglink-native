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
            .color(palette.text_primary),
    );

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        // --- the theme ---
        let next = app.session.theme.next();
        if ui
            .add(
                egui::Button::new(
                    RichText::new(next.name())
                        .font(TypeStep::Control.font())
                        .color(palette.text_secondary),
                )
                .fill(egui::Color32::TRANSPARENT)
                .stroke(egui::Stroke::NONE)
                .min_size(egui::vec2(0.0, 24.0)),
            )
            .on_hover_text("Switch to the next theme")
            .clicked()
        {
            app.session.theme = next;
        }

        ui.add_space(metrics::GAP_LARGE);
        ui.label(
            RichText::new(format!("{} curves", app.session.display_curves.len()))
                .font(TypeStep::Label.font())
                .color(palette.text_tertiary),
        );
    });
}
