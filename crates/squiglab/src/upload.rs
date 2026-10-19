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
    ui.label(
        RichText::new(
            "Measurements from another rig do not mix with the ones here: they were taken \
             with different equipment, so they are not the same measurement.",
        )
        .font(TypeStep::Label.font())
        .color(palette.text_secondary),
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
    });
    ui.add_space(metrics::GAP);
    ui.horizontal(|ui| {
        if ui
            .button("Upload FR")
            .on_hover_text("Load this measurement")
            .clicked()
        {
            upload(app, false);
        }
        if ui
            .button("Upload Target")
            .on_hover_text("Load this measurement as a target")
            .clicked()
        {
            upload(app, true);
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

/// Load the file the user named.
fn upload(app: &mut App, as_target: bool) {
    let path = app.upload_path.trim();
    if path.is_empty() {
        app.upload_message = Some("Name the file to load.".to_owned());
        return;
    }
    match app
        .session
        .import_upload(std::path::Path::new(path), as_target)
    {
        Ok(message) => {
            app.upload_message = Some(message);
            app.session.rebuild_display_curves();
        }
        Err(problem) => app.upload_message = Some(format!("The file was not loaded: {problem}")),
    }
}
