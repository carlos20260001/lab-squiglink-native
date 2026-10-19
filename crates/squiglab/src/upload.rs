//! The upload panel: measurements that come from somewhere else.
//!
//! The reference warns that another rig's measurements do not mix with the
//! ones here, and then offers to load them anyway. That warning is the
//! program's own: a measurement taken on a different rig is not a measurement
//! of the same thing.

use egui::{RichText, Ui};

use crate::app::App;
use crate::theme::{self, metrics, TypeStep};

/// The upload panel.
pub fn show(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();
    ui.label(
        RichText::new("Upload Data")
            .font(TypeStep::Title.font_bold())
            .color(palette.text_primary),
    );
    ui.add_space(metrics::GAP);
    ui.add(
        egui::Label::new(
            RichText::new(
                "Measurements from another rig do not mix with the ones here: they were \
                 taken with different equipment, so they are not the same measurement.",
            )
            .font(TypeStep::Label.font())
            .color(palette.text_secondary),
        )
        .wrap(),
    );
    ui.add_space(metrics::GAP);

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("File")
                .font(TypeStep::Label.font())
                .color(palette.text_tertiary),
        );
        ui.add(
            egui::TextEdit::singleline(&mut app.upload_path)
                .hint_text("A measurement file")
                .desired_width(ui.available_width() - 150.0),
        );
        // The system dialog, where there is one.
        if ui
            .button("Browse")
            .on_hover_text("Choose the file with the system's own dialog")
            .clicked()
        {
            app.ask_for_file(
                crate::app::Pick::Upload,
                "A measurement file",
                &["txt", "csv", "fr"],
            );
        }
    });
    ui.add_space(metrics::GAP);
    ui.horizontal(|ui| {
        if ui
            .button("Upload FR")
            .on_hover_text("Load this measurement")
            .clicked()
        {
            app.upload_chosen(false);
        }
        if ui
            .button("Upload Target")
            .on_hover_text("Load this measurement as a target")
            .clicked()
        {
            app.upload_chosen(true);
        }
    });
    if let Some(message) = &app.upload_message {
        ui.add_space(metrics::GAP);
        ui.label(
            RichText::new(message.clone())
                .font(TypeStep::Label.font())
                .color(palette.text_secondary),
        );
    }
}
