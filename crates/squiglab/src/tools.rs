//! The row of tools above the graph.
//!
//! The reference's tools sit in a row that scrolls sideways when it does not
//! fit, and this is that row: the reading controls on the left, the actions
//! on the right.

use egui::{RichText, Ui};

use crate::app::App;
use crate::theme::{self, metrics, TypeStep};

/// The tools.
pub fn show(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();

    egui::ScrollArea::horizontal()
        .id_salt("tools")
        .auto_shrink([false, false])
        .max_height(40.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                zoom(ui, app);
                rule(ui);
                normalize(ui, app);
                rule(ui);
                smooth(ui, app);
                rule(ui);
                toggles(ui, app);
                rule(ui);
                actions(ui, app);
            });
        });
    ui.add_space(metrics::GAP);
    let _ = palette;
}

/// A divider between the clusters.
fn rule(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 20.0), egui::Sense::hover());
    let palette = theme::current_palette();
    ui.painter().vline(
        rect.center().x,
        rect.min.y..=rect.max.y,
        egui::Stroke::new(1.0, palette.border),
    );
    ui.add_space(metrics::GAP);
}

/// A cluster's name.
fn label(ui: &mut Ui, text: &str) {
    let palette = theme::current_palette();
    ui.label(
        RichText::new(text.to_uppercase())
            .font(TypeStep::Label.font())
            .color(palette.text_tertiary),
    );
}

/// The zoom: which part of the frequency axis the graph shows.
fn zoom(ui: &mut Ui, app: &mut App) {
    label(ui, "Zoom");
    for (region, name) in [
        (squiglab_core::graph::ZoomRegion::Full, "Full"),
        (squiglab_core::graph::ZoomRegion::Bass, "Bass"),
        (squiglab_core::graph::ZoomRegion::Mids, "Mids"),
        (squiglab_core::graph::ZoomRegion::Treble, "Treble"),
    ] {
        let selected = app.session.zoom == region;
        if ui
            .selectable_label(selected, name)
            .on_hover_text("Show this part of the frequency axis")
            .clicked()
        {
            app.session.zoom = region;
        }
    }
}

/// The normalization: the level the curves are read at.
fn normalize(ui: &mut Ui, app: &mut App) {
    label(ui, "Normalize");
    let mut by_phon = app.session.phon_normalization;
    if ui
        .selectable_label(by_phon, "dB")
        .on_hover_text("Read the curves at a loudness level")
        .clicked()
    {
        app.session.phon_normalization = true;
        by_phon = true;
    }
    if ui
        .selectable_label(!by_phon, "Hz")
        .on_hover_text("Read the curves at a frequency")
        .clicked()
    {
        app.session.phon_normalization = false;
        by_phon = false;
    }
    let _ = by_phon;
    let mut phon = app.session.target_phon;
    if ui
        .add(
            egui::DragValue::new(&mut phon)
                .range(0.0..=100.0)
                .speed(1.0)
                .suffix(" dB"),
        )
        .changed()
    {
        app.session.target_phon = phon;
    }
    let mut hertz = app.session.target_hz;
    if ui
        .add(
            egui::DragValue::new(&mut hertz)
                .range(20.0..=20_000.0)
                .speed(10.0)
                .suffix(" Hz"),
        )
        .changed()
    {
        app.session.target_hz = hertz;
    }
}

/// The smoothing: how much of the curve's roughness is taken off.
fn smooth(ui: &mut Ui, app: &mut App) {
    label(ui, "Smooth");
    let mut level = app.session.smooth_level as f64;
    if ui
        .add(
            egui::Slider::new(
                &mut level,
                0.0..=squiglab_core::config::MAX_SMOOTH_LEVEL as f64,
            )
            .suffix("/oct"),
        )
        .on_hover_text("Fractional octaves of smoothing")
        .changed()
    {
        app.session.smooth_level =
            level.clamp(0.0, squiglab_core::config::MAX_SMOOTH_LEVEL as f64) as usize;
    }
}

/// The view toggles.
fn toggles(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();
    let mut labels = app.show_labels;
    if ui
        .checkbox(&mut labels, "Labels")
        .on_hover_text("Name each curve at its end")
        .changed()
    {
        app.show_labels = labels;
    }
    let mut inspect = app.inspect;
    if ui
        .checkbox(&mut inspect, "Inspect")
        .on_hover_text("Read the curve under the pointer")
        .changed()
    {
        app.inspect = inspect;
    }
    if ui
        .button("Recolor")
        .on_hover_text("Turn the curve colours around the wheel")
        .clicked()
    {
        app.session.color_offset = (app.session.color_offset + 1) % 16;
        app.session.rebuild_display_curves();
    }
    let _ = palette;
}

/// The actions that apply to the window.
fn actions(ui: &mut Ui, app: &mut App) {
    if ui
        .button("Copy URL")
        .on_hover_text("Copy a link to this view")
        .clicked()
    {
        let uri = squiglab_core::config::encode_share_uri(&app.session.preferences());
        match uri {
            Ok(uri) => {
                ui.ctx().copy_text(uri.clone());
                app.say(format!("Link copied: {uri}"));
            }
            Err(error) => app.say(format!("The link could not be made: {error}")),
        }
    }
    if ui
        .button("Share")
        .on_hover_text("Open a window with this view")
        .clicked()
    {
        match squiglab_core::config::encode_share_uri(&app.session.preferences()) {
            Ok(uri) => app.say(format!("Share this link: {uri}")),
            Err(error) => app.say(format!("The link could not be made: {error}")),
        }
    }
}
