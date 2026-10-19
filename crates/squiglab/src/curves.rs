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
    ui.add_space(metrics::GAP);
    databases(ui);
}

/// The other databases' graphs, as the reference lists them under the rows.
///
/// They are the reference's own links to other people's tools, so they are
/// carried as links and not as data.
fn databases(ui: &mut Ui) {
    let palette = theme::current_palette();
    egui::ScrollArea::horizontal()
        .id_salt("databases")
        .auto_shrink([false, false])
        .max_height(40.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("IEM GRAPH DATABASES")
                        .font(TypeStep::Label.font())
                        .color(palette.text_tertiary),
                );
                for (name, url) in DATABASES {
                    let response = ui
                        .add(
                            egui::Button::new(
                                RichText::new(*name)
                                    .font(egui::FontId::new(12.0, egui::FontFamily::Proportional))
                                    .color(palette.accent_text),
                            )
                            .fill(palette.page)
                            .stroke(egui::Stroke::new(1.0, palette.border))
                            .corner_radius(egui::CornerRadius::same(6))
                            .min_size(egui::vec2(0.0, 36.0)),
                        )
                        .on_hover_text(format!("Open {url} in the browser"));
                    if response.clicked() {
                        crate::links::open(url);
                    }
                }
            });
        });
}

/// The other graphs' tools, and where they are.
const DATABASES: &[(&str, &str)] = &[
    ("Audio Discourse", "https://iems.audiodiscourse.com/"),
    ("Bad Guy", "https://hbb.squig.link/"),
    ("Banbeucmas", "https://banbeu.com/graph/tool/"),
    ("HypetheSonics", "https://www.hypethesonics.com/iemdbc/"),
    (
        "In-Ear Fidelity",
        "https://crinacle.com/graphs/iems/graphtool/",
    ),
    ("Precogvision", "https://precog.squig.link/"),
    ("Rikudou Goku", "https://rg.squig.link/"),
    ("Super* Review", "https://squig.link/"),
    ("Audio Discourse", "https://headphones.audiodiscourse.com/"),
    (
        "In-Ear Fidelity",
        "https://crinacle.com/graphs/headphones/graph",
    ),
    ("Super* Review", "https://squig.link/hp.html"),
];

/// The chips that turn the targets on and off, grouped as the reference groups
/// them.
fn targets(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();

    // The strip is one line, as the reference's is: the categories follow one
    // another, separated, and the line scrolls sideways when there are more
    // targets than the width holds. The ones on the graph are picked out.
    egui::ScrollArea::horizontal()
        .id_salt("target_strip")
        .auto_shrink([false, false])
        .max_height(30.0)
        .show(ui, |ui| {
            // One row: the categories follow one another, and the line
            // scrolls sideways when there are more targets than it holds.
            ui.horizontal(|ui| {
                let mut first = true;
                let heading = |ui: &mut Ui, label: &str, first: &mut bool| {
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
                }

                // The categories keep their own order, so the strip reads the same
                // way every time.
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
                    heading(ui, category, &mut first);
                    for index in targets {
                        let name = app.session.groups[index].name.clone();
                        let on = app.session.active_targets.contains(&index);
                        let response =
                            chip(ui, &name, on).on_hover_text("Show this target on the graph");
                        if response.clicked() {
                            if on {
                                app.session.active_targets.remove(&index);
                            } else {
                                app.session.active_targets.insert(index);
                            }
                            app.session.rebuild_display_curves();
                        }
                    }
                }
            });
        });
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
    let target = group.target;

    // The hairline that separates one row from the next.
    let (line, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().hline(
        line.min.x..=line.max.x,
        line.center().y,
        egui::Stroke::new(1.0, palette.border),
    );

    ui.horizontal(|ui| {
        // The curve's colour, as a dot.
        let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 7.0, color);

        // The name. A target is a reference, and is written in the quieter
        // grey; the measurement is the subject.
        ui.label(
            RichText::new(name.clone())
                .font(egui::FontId::new(14.0, egui::FontFamily::Proportional))
                .strong()
                .color(if target {
                    palette.text_tertiary
                } else {
                    palette.text_primary
                }),
        );

        // The key: the reading of the curve, and the control that chooses the
        // channel. It is the reference's own drawing.
        let mode = app
            .session
            .channel_modes
            .get(&index)
            .copied()
            .unwrap_or(ChannelMode::Both);
        let (key_rect, key_response) =
            ui.allocate_exact_size(egui::vec2(65.0, 24.0), egui::Sense::click());
        draw_key(ui, key_rect, color, mode, target);
        if key_response.clicked() {
            let pointer = key_response
                .interact_pointer_pos()
                .unwrap_or(key_rect.center());
            let chosen = if pointer.x < key_rect.left() + 12.0 {
                ChannelMode::Left
            } else if pointer.x > key_rect.right() - 12.0 {
                ChannelMode::Right
            } else {
                ChannelMode::Both
            };
            if chosen != mode {
                app.session.channel_modes.insert(index, chosen);
                app.session.rebuild_display_curves();
            }
        }
        if key_response.hovered() {
            key_response.on_hover_text("Show the left, the right, or both channels");
        }

        // The offset: the curve moved up or down, in decibels.
        let offset = app.session.offsets.get(&index).copied().unwrap_or(0.0);
        let mut text = format!("{offset:.0}");
        if ui
            .add_sized(
                [70.0, 36.0],
                egui::TextEdit::singleline(&mut text)
                    .desired_width(70.0)
                    .font(egui::FontId::new(12.0, egui::FontFamily::Proportional)),
            )
            .on_hover_text("Move this curve up or down, in decibels")
            .changed()
        {
            if let Ok(parsed) = text.trim().parse::<f32>() {
                let parsed = parsed.clamp(-20.0, 20.0);
                app.session.offsets.insert(index, parsed);
                app.session.rebuild_display_curves();
            }
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Remove.
            if crate::icons::button(ui, crate::icons::Icon::Remove, "Remove graph", false).clicked()
            {
                app.session.toggle_curve(index);
                app.session.rebuild_display_curves();
            }
            // Export.
            if crate::icons::button(ui, crate::icons::Icon::Download, "Export graph", false)
                .clicked()
            {
                match app.session.export_curve_text(index) {
                    Ok(text) => app.say(format!("The curve is ready to save: {text}")),
                    Err(problem) => app.say(format!("The curve was not saved: {problem}")),
                }
            }
            // Pin.
            let pinned = app.session.pinned.contains(&index);
            if crate::icons::button(ui, crate::icons::Icon::Pin, "Pin graph", pinned).clicked() {
                if pinned {
                    app.session.pinned.remove(&index);
                } else {
                    app.session.pinned.insert(index);
                }
                app.session.dirty = true;
            }
            // The baseline: the curve the others are read against.
            let baseline = app.session.baseline == Some(index);
            if crate::icons::button(
                ui,
                crate::icons::Icon::Baseline,
                "Set as baseline",
                baseline,
            )
            .clicked()
            {
                app.session.baseline = if baseline { None } else { Some(index) };
                app.session.rebuild_display_curves();
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
                "Hide graph",
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

/// A target's chip: an outlined box, filled with the accent when the target is
/// on the graph.
fn chip(ui: &mut Ui, label: &str, on: bool) -> egui::Response {
    let palette = theme::current_palette();
    let (fill, text, border) = if on {
        (palette.accent, palette.on_accent, palette.accent)
    } else {
        (palette.page, palette.accent_text, palette.border_strong)
    };
    ui.add(
        egui::Button::new(
            RichText::new(label)
                .font(egui::FontId::new(12.0, egui::FontFamily::Proportional))
                .color(text),
        )
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, border))
        .corner_radius(egui::CornerRadius::same(6))
        .min_size(egui::vec2(0.0, 36.0)),
    )
}

/// The key: the reading of the curve, and the control that chooses the channel.
///
/// The reference draws it as a pair of tapered strokes meeting in a point on
/// the left and opening to the right, one per channel, in the curve's colour.
/// A single channel is a straight line, and a target's is a straight line with
/// the word "Target" beside it.
fn draw_key(ui: &Ui, rect: egui::Rect, color: egui::Color32, mode: ChannelMode, target: bool) {
    let painter = ui.painter();
    // The reference's box is 65 by 24, with its origin 19 to the left of the
    // point the strokes meet at.
    let origin = rect.left() + 19.0;
    let middle = rect.center().y;
    let stroke = egui::Stroke::new(2.0, color);
    let stroke_to = |x: f32, y: f32| egui::Pos2::new(origin + x, middle + y);

    match mode {
        ChannelMode::Left => {
            painter.add(egui::Shape::line(
                vec![
                    stroke_to(15.0, -6.0),
                    stroke_to(9.0, -6.0),
                    stroke_to(-9.0, 0.0),
                    stroke_to(-17.0, 0.0),
                ],
                stroke,
            ));
        }
        ChannelMode::Right => {
            painter.add(egui::Shape::line(
                vec![
                    stroke_to(-17.0, 0.0),
                    stroke_to(-9.0, 0.0),
                    stroke_to(9.0, 6.0),
                    stroke_to(15.0, 6.0),
                ],
                stroke,
            ));
        }
        _ => {
            painter.add(egui::Shape::line(
                vec![
                    stroke_to(15.0, -6.0),
                    stroke_to(9.0, -6.0),
                    stroke_to(-9.0, 0.0),
                    stroke_to(-17.0, 0.0),
                ],
                stroke,
            ));
            painter.add(egui::Shape::line(
                vec![
                    stroke_to(-17.0, 0.0),
                    stroke_to(-9.0, 0.0),
                    stroke_to(9.0, 6.0),
                    stroke_to(15.0, 6.0),
                ],
                stroke,
            ));
        }
    }

    // The channel labels, and the word under the one that is alone.
    let font = egui::FontId::new(10.5, egui::FontFamily::Proportional);
    let small = || egui::FontId::new(7.5, egui::FontFamily::Proportional);
    if target {
        painter.text(
            egui::Pos2::new(rect.left() + 26.0, middle),
            egui::Align2::LEFT_CENTER,
            "Target",
            egui::FontId::new(8.0, egui::FontFamily::Proportional),
            color,
        );
        return;
    }
    painter.text(
        egui::Pos2::new(origin + 17.0, middle - 6.0),
        egui::Align2::LEFT_CENTER,
        "L",
        font.clone(),
        color,
    );
    painter.text(
        egui::Pos2::new(origin + 17.0, middle + 6.0),
        egui::Align2::LEFT_CENTER,
        "R",
        font,
        color,
    );
    if mode == ChannelMode::Left {
        painter.text(
            egui::Pos2::new(origin + 17.0, middle - 6.0),
            egui::Align2::LEFT_CENTER,
            "only",
            small(),
            color,
        );
    }
    if mode == ChannelMode::Right {
        painter.text(
            egui::Pos2::new(origin + 17.0, middle + 6.0),
            egui::Align2::LEFT_CENTER,
            "only",
            small(),
            color,
        );
    }
}

/// The bank a row belongs to, for the panels that ask.
pub fn bank(app: &App) -> EqBank {
    app.session.eq_bank
}
