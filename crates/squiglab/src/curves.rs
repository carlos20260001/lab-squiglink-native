//! What belongs to the graph: the target chips, and the rows of the curves
//! that are on it.
//!
//! The reference puts the targets in a strip under the graph, grouped by the
//! kind of target they are, and the active curves in a table below that: a row
//! each, with the curve's colour, its name, the channels shown, the offset
//! that moves it, and the buttons that act on it.

use egui::{Align, Layout, RichText, Ui};

use crate::app::App;
use crate::theme::{self, metrics, TypeStep};
use squiglab_core::session::{ChannelMode, EqBank};

/// The target chips, and the rows of the active curves.
pub fn show(ui: &mut Ui, app: &mut App) {
    ui.add_space(metrics::GAP);
    targets(ui, app);
    ui.add_space(metrics::GAP);
    manager(ui, app);
}

/// The chips that turn the targets on and off, grouped as the reference groups
/// them.
fn targets(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();

    // The strip lists every target the library knows, grouped as the
    // reference groups them, and the ones on the graph are picked out. The
    // reference's list comes from its own configuration; here the groups are
    // the ones the core builds from the measurement files.
    let mut first = true;
    let mut heading = |ui: &mut Ui, label: &str, first: &mut bool| {
        if !*first {
            ui.label(
                RichText::new("|")
                    .font(TypeStep::Label.font())
                    .color(palette.text_tertiary),
            );
        }
        *first = false;
        ui.label(
            RichText::new(format!("{label}:"))
                .font(TypeStep::Label.font())
                .color(palette.text_tertiary),
        );
    };

    // The rig corrections are a group of their own, marked Δ, as the
    // reference marks them.
    if !app.session.delta_targets.is_empty() {
        ui.horizontal_wrapped(|ui| {
            heading(ui, "Δ", &mut first);
            for index in 0..app.session.delta_targets.len() {
                let name = app.session.delta_targets[index].name.clone();
                let on = app.session.active_deltas.contains(&index);
                let response = ui
                    .selectable_label(
                        on,
                        RichText::new(name.clone())
                            .font(TypeStep::Label.font())
                            .color(if on {
                                palette.on_accent
                            } else {
                                palette.text_secondary
                            }),
                    )
                    .on_hover_text(format!(
                        "The correction for the {} rig{}",
                        app.session.delta_targets[index].clone_rig,
                        match &app.session.delta_targets[index].note {
                            Some(note) => format!(": {note}"),
                            None => String::new(),
                        }
                    ));
                if response.clicked() {
                    if on {
                        app.session.active_deltas.remove(&index);
                    } else {
                        app.session.active_deltas.insert(index);
                    }
                    app.session.rebuild_display_curves();
                }
            }
        });
    }

    // The categories keep their own order, so the strip reads the same way
    // every time.
    let order = [
        "Neutral targets",
        "Reviewer targets",
        "Preference targets",
        "Other targets",
    ];
    for category in order {
        let targets: Vec<usize> = app
            .session
            .groups
            .iter()
            .enumerate()
            .filter(|(_, group)| group.target && group.brand == category)
            .map(|(index, _)| index)
            .collect();
        if targets.is_empty() {
            continue;
        }
        ui.horizontal_wrapped(|ui| {
            heading(ui, category, &mut first);
            for index in targets {
                let name = app.session.groups[index].name.clone();
                let on = app.session.active_targets.contains(&index);
                let response = ui
                    .selectable_label(
                        on,
                        RichText::new(name.clone())
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
}

/// The rows of the curves that are on the graph.
fn manager(ui: &mut Ui, app: &mut App) {
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
    for index in active {
        row(ui, app, index);
    }
}

/// One row of the manager: who the curve is, and what can be done to it.
fn row(ui: &mut Ui, app: &mut App, index: usize) {
    let palette = theme::current_palette();
    let group = &app.session.groups[index];
    let name = group.name.clone();
    let color = theme::color(crate::graph::curve_color(app, index));

    ui.horizontal(|ui| {
        // The curve's colour, as a dot.
        let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 5.0, color);
        ui.label(
            RichText::new(name.clone())
                .font(TypeStep::Body.font())
                .color(palette.text_primary),
        );

        // The channels: the reference offers left, right and both.
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

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Remove.
            if crate::icons::button(
                ui,
                crate::icons::Icon::Close,
                "Take this curve off the graph",
                false,
            )
            .clicked()
            {
                app.session.toggle_curve(index);
                app.session.rebuild_display_curves();
            }
            // Export.
            if crate::icons::button(
                ui,
                crate::icons::Icon::Save,
                "Save this curve as a text file",
                false,
            )
            .clicked()
            {
                match app.session.export_curve_text(index) {
                    Ok(text) => app.say(format!("The curve is ready to save: {text}")),
                    Err(problem) => app.say(format!("The curve was not saved: {problem}")),
                }
            }
            // Pin.
            let pinned = app.session.pinned.contains(&index);
            if crate::icons::button(
                ui,
                if pinned {
                    crate::icons::Icon::Pin
                } else {
                    crate::icons::Icon::PinOutline
                },
                "Pin the curve so it stays when the others are taken off",
                pinned,
            )
            .clicked()
            {
                if pinned {
                    app.session.pinned.remove(&index);
                } else {
                    app.session.pinned.insert(index);
                }
                app.session.dirty = true;
            }
            // Hide.
            let hidden = app.session.hidden.contains(&index);
            if crate::icons::button(
                ui,
                if hidden {
                    crate::icons::Icon::EyeCrossed
                } else {
                    crate::icons::Icon::Eye
                },
                "Hide the curve without taking it off the graph",
                hidden,
            )
            .clicked()
            {
                if hidden {
                    app.session.hidden.remove(&index);
                } else {
                    app.session.hidden.insert(index);
                }
                app.session.rebuild_display_curves();
            }
        });
    });
}

/// The bank a row belongs to, for the panels that ask.
pub fn bank(app: &App) -> EqBank {
    app.session.eq_bank
}
