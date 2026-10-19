//! What belongs to the graph: the targets, and the curves that are on it.
//!
//! The reference puts the target chips under the graph and the list of active
//! curves under those, and this is the same two things.

use egui::{RichText, Ui};

use crate::app::App;
use crate::theme::{self, metrics, TypeStep};
use squiglab_core::session::ChannelMode;

/// The target chips, and the list of active curves.
pub fn show(ui: &mut Ui, app: &mut App) {
    ui.add_space(metrics::GAP);
    targets(ui, app);
    ui.add_space(metrics::GAP);
    active(ui, app);
}

/// The chips that turn the targets on and off.
fn targets(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();
    let targets: Vec<usize> = app.session.active_targets.iter().copied().collect();
    if targets.is_empty() {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new("TARGETS")
                .font(TypeStep::Label.font())
                .color(palette.text_tertiary),
        );
        for index in targets {
            let name = app.session.groups[index].name.clone();
            let on = app.session.active_targets.contains(&index);
            let response = ui
                .selectable_label(
                    on,
                    RichText::new(name)
                        .font(TypeStep::Label.font())
                        .color(if on {
                            palette.on_accent
                        } else {
                            palette.text_secondary
                        }),
                )
                .on_hover_text("Show this target on the graph");
            if response.clicked() {
                if on {
                    app.session.active_targets.remove(&index);
                } else {
                    app.session.active_targets.insert(index);
                }
                app.session.rebuild_display_curves();
            }
        }
    });
}

/// The list of the curves that are on the graph.
fn active(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();
    let active: Vec<usize> = app.session.active.iter().copied().collect();
    if active.is_empty() {
        ui.label(
            RichText::new("No curves are on the graph. Choose a measurement on the left.")
                .font(TypeStep::Body.font())
                .color(palette.text_secondary),
        );
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new("ACTIVE CURVES")
                .font(TypeStep::Label.font())
                .color(palette.text_tertiary),
        );
        for index in active {
            let group = &app.session.groups[index];
            let name = group.name.clone();
            let response = ui
                .selectable_label(
                    true,
                    RichText::new(name.clone())
                        .font(TypeStep::Label.font())
                        .color(palette.on_accent),
                )
                .on_hover_text("This measurement is on the graph");
            if response.clicked() {
                app.session.active.remove(&index);
                app.session.rebuild_display_curves();
            }

            // The channel: the reference offers left, right and both.
            let mode = app
                .session
                .channel_modes
                .get(&index)
                .copied()
                .unwrap_or(ChannelMode::Both);
            for (option, name) in [
                (ChannelMode::Left, "L"),
                (ChannelMode::Right, "R"),
                (ChannelMode::Both, "Both"),
            ] {
                let selected = mode == option;
                if ui
                    .selectable_label(selected, RichText::new(name).font(TypeStep::Label.font()))
                    .on_hover_text("Which channels of the measurement to show")
                    .clicked()
                {
                    app.session.channel_modes.insert(index, option);
                    app.session.rebuild_display_curves();
                }
            }

            // The offset: the curve moved up or down, in decibels.
            let mut offset = app.session.offsets.get(&index).copied().unwrap_or(0.0);
            if ui
                .add(
                    egui::DragValue::new(&mut offset)
                        .range(-20.0..=20.0)
                        .speed(0.1)
                        .suffix(" dB"),
                )
                .on_hover_text("Move this curve up or down")
                .changed()
            {
                app.session.offsets.insert(index, offset);
                app.session.rebuild_display_curves();
            }
        }
    });
}
