//! The equalizer panel: the bands of a measurement's equalizer, what a fit may
//! produce, and the changes that can be undone.
//!
//! The reference keeps its equalizer, its constraints and its change history
//! in the same panel under the list, and this is that panel.

use egui::{RichText, Ui};
use squiglab_core::constraints::{Constraints, PRESETS};
use squiglab_core::equalizer::{Filter, FilterType};
use squiglab_core::session::EqRow;

use crate::app::App;
use crate::theme::{self, metrics, TypeStep};

/// The changes that can be undone, and the ones that can be redone.
#[derive(Clone, Default)]
pub struct History {
    past: Vec<Snapshot>,
    future: Vec<Snapshot>,
}

/// One state of the equalizer, as the history keeps it.
#[derive(Clone, PartialEq)]
pub struct Snapshot {
    filters: std::collections::BTreeMap<usize, Vec<EqRow>>,
    constraints: Constraints,
}

impl History {
    /// Record a state, so it can be come back to.
    pub fn record(&mut self, snapshot: Snapshot) {
        self.past.push(snapshot);
        self.future.clear();
    }

    /// Whether there is anything to undo.
    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    /// Whether there is anything to redo.
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    /// Step back, returning the state to come back to.
    pub fn undo(&mut self) -> Option<Snapshot> {
        let state = self.past.pop()?;
        self.future.push(state.clone());
        Some(state)
    }

    /// Step forward, returning the state to come back to.
    pub fn redo(&mut self) -> Option<Snapshot> {
        let state = self.future.pop()?;
        self.past.push(state.clone());
        Some(state)
    }
}

/// The equalizer of the selected measurement.
pub fn show(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Parametric Equalizer")
                .font(TypeStep::Title.font_bold())
                .color(palette.text_primary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button("Reset")
                .on_hover_text("Clear this curve's bands")
                .clicked()
            {
                if let Some(phone) = app.session.eq_phone {
                    app.session.eq_filters.insert(phone, Vec::new());
                    app.history.record(snapshot(app));
                    app.session.rebuild_display_curves();
                }
            }
        });
    });
    ui.add_space(metrics::GAP);

    let Some(phone) = app.session.eq_phone else {
        ui.label(
            RichText::new("Choose a measurement to give it an equalizer.")
                .font(TypeStep::Body.font())
                .color(palette.text_secondary),
        );
        return;
    };

    // --- the measurement, the target, and the fit ---
    let measurements = app.session.active_measurements();
    let names: Vec<String> = measurements
        .iter()
        .map(|&index| app.session.groups[index].name.clone())
        .collect();
    let targets: Vec<usize> = app.session.active_targets.iter().copied().collect();
    ui.horizontal(|ui| {
        let mut selected = Some(phone);
        egui::ComboBox::from_id_salt("eq_phone")
            .selected_text(
                measurements
                    .iter()
                    .position(|&index| Some(index) == selected)
                    .map(|position| names[position].as_str())
                    .unwrap_or("Measurement"),
            )
            .show_ui(ui, |ui| {
                for (index, name) in measurements.iter().zip(&names) {
                    ui.selectable_value(&mut selected, Some(*index), name);
                }
            });
        if selected != Some(phone) {
            app.session.eq_phone = selected;
        }

        let mut target = app.session.eq_target;
        let target_label = target
            .map(|index| app.session.groups[index].name.clone())
            .unwrap_or_else(|| "None".to_owned());
        egui::ComboBox::from_id_salt("eq_target")
            .selected_text(target_label.as_str())
            .show_ui(ui, |ui| {
                for index in &targets {
                    ui.selectable_value(
                        &mut target,
                        Some(*index),
                        &app.session.groups[*index].name,
                    );
                }
            });
        if target != app.session.eq_target {
            app.session.eq_target = target;
        }

        if ui
            .button("AutoEQ")
            .on_hover_text("Fit bands to the target")
            .clicked()
        {
            let before = snapshot(app);
            let outcome = app.session.run_auto_eq();
            app.history.record(before);
            match outcome {
                Ok(message) => app.say(message),
                Err(problem) => app.say(problem),
            }
            app.session.rebuild_display_curves();
        }
    });
    ui.add_space(metrics::GAP);

    // --- the band table ---
    // The rows are laid out by hand rather than as a grid: the column is
    // narrow, and a grid would give each column the room its heading needs
    // rather than the room its figures need.
    let mut rows = app
        .session
        .eq_filters
        .get(&phone)
        .cloned()
        .unwrap_or_default();
    let mut changed = false;

    ui.horizontal(|ui| {
        for (heading, width) in [
            ("", 22.0),
            ("Type", 52.0),
            ("Frequency", 62.0),
            ("Gain", 58.0),
            ("Q", 46.0),
        ] {
            ui.add_sized(
                [width, 18.0],
                egui::Label::new(
                    RichText::new(heading)
                        .font(TypeStep::Label.font())
                        .color(palette.text_tertiary),
                ),
            );
        }
    });
    for row in rows.iter_mut() {
        ui.horizontal(|ui| {
            let mut enabled = row.enabled;
            if ui
                .add_sized([22.0, 20.0], egui::Checkbox::new(&mut enabled, ""))
                .on_hover_text("Include this band in the response and the exports")
                .changed()
            {
                row.enabled = enabled;
                changed = true;
            }
            // A menu rather than a select: the select is wider than the names
            // it holds, and the column is narrow.
            ui.add_sized(
                [52.0, 20.0],
                egui::Button::new(row.filter.filter_type.short_name())
                    .min_size(egui::vec2(52.0, 20.0)),
            )
            .on_hover_text("The kind of filter this band is")
            .context_menu(|ui| {
                for filter_type in [
                    FilterType::Peaking,
                    FilterType::LowShelf,
                    FilterType::HighShelf,
                ] {
                    ui.selectable_value(
                        &mut row.filter.filter_type,
                        filter_type,
                        filter_type.short_name(),
                    );
                }
            });
            if number(ui, &mut row.filter.freq, 0.0, 20_000.0, 0, 62.0) {
                changed = true;
            }
            if number(ui, &mut row.filter.gain, -40.0, 40.0, 1, 58.0) {
                changed = true;
            }
            if number(ui, &mut row.filter.q, 0.0, 20.0, 2, 46.0) {
                changed = true;
            }
        });
    }
    ui.label(
        RichText::new("Frequencies in hertz, gains in decibels.")
            .font(TypeStep::Label.font())
            .color(palette.text_tertiary),
    );
    if changed {
        app.session.eq_filters.insert(phone, rows);
        app.history.record(snapshot(app));
        app.session.rebuild_display_curves();
    }

    // --- the row controls ---
    ui.horizontal(|ui| {
        if ui.button("+").on_hover_text("Add a band").clicked() {
            let mut rows = app
                .session
                .eq_filters
                .get(&phone)
                .cloned()
                .unwrap_or_default();
            rows.push(EqRow {
                filter: Filter::new(FilterType::Peaking, 1000.0, 1.0, 0.0),
                enabled: true,
            });
            app.session.eq_filters.insert(phone, rows);
            app.history.record(snapshot(app));
            app.session.rebuild_display_curves();
        }
        if ui
            .button("−")
            .on_hover_text("Remove the last band")
            .clicked()
        {
            if let Some(mut rows) = app.session.eq_filters.get(&phone).cloned() {
                rows.pop();
                app.session.eq_filters.insert(phone, rows);
                app.history.record(snapshot(app));
                app.session.rebuild_display_curves();
            }
        }
        if ui
            .button("Sort")
            .on_hover_text("Order the bands by frequency")
            .clicked()
        {
            if let Some(mut rows) = app.session.eq_filters.get(&phone).cloned() {
                rows.sort_by(|left, right| left.filter.freq.total_cmp(&right.filter.freq));
                app.session.eq_filters.insert(phone, rows);
                app.history.record(snapshot(app));
                app.session.rebuild_display_curves();
            }
        }
    });
    ui.add_space(metrics::GAP);

    // --- what a fit may produce ---
    constraints(ui, app);

    // --- the change history ---
    ui.add_space(metrics::GAP);
    history(ui, app);

    // --- the exports ---
    ui.add_space(metrics::GAP);
    ui.horizontal(|ui| {
        if ui
            .button("Export")
            .on_hover_text("Save an Equalizer APO file")
            .clicked()
        {
            match app.session.parametric_export() {
                Some((name, contents)) => app.say(format!("{name} is ready to save: {contents}")),
                None => app.say("There is no equalizer to export."),
            }
        }
        if ui
            .button("Export Graphic EQ")
            .on_hover_text("Save a Wavelet file")
            .clicked()
        {
            match app.session.graphic_eq_export() {
                Some((name, contents)) => app.say(format!("{name} is ready to save: {contents}")),
                None => app.say("There is no equalizer to export."),
            }
        }
    });
}

/// The constraints: what a fit is allowed to produce.
fn constraints(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();
    ui.label(
        RichText::new("CONSTRAINTS")
            .font(TypeStep::Label.font())
            .color(palette.text_tertiary),
    );

    let mut constraints = app.session.eq_constraints.clone();

    // The preset.
    let current = PRESETS
        .iter()
        .find(|preset| preset.constraints() == constraints)
        .map(|preset| preset.id);
    let mut selected = current.unwrap_or("");
    egui::ComboBox::from_id_salt("eq_preset")
        .selected_text(
            PRESETS
                .iter()
                .find(|preset| preset.id == selected)
                .map(|preset| preset.label)
                .unwrap_or("Custom"),
        )
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut selected, "", "Custom");
            for preset in PRESETS {
                ui.selectable_value(&mut selected, preset.id, preset.label);
            }
        });
    if selected != current.unwrap_or("") {
        if let Some(preset) = Constraints::preset(selected) {
            constraints = preset;
        }
    }

    // The filter kinds.
    ui.horizontal(|ui| {
        for (name, allowed) in [
            ("PK", &mut constraints.allow_peaking),
            ("LSQ", &mut constraints.allow_low_shelf),
            ("HSQ", &mut constraints.allow_high_shelf),
        ] {
            let mut value = *allowed;
            if ui
                .checkbox(&mut value, name)
                .on_hover_text("Whether the fit may use this kind of filter")
                .changed()
            {
                *allowed = value;
            }
        }
    });

    // The limits.
    ui.horizontal(|ui| {
        ui.label("Max bands");
        let mut max = constraints.max_filters as f64;
        if ui
            .add(egui::DragValue::new(&mut max).range(0.0..=20.0).speed(1.0))
            .on_hover_text("How many filters the fit may produce. Zero is no cap.")
            .changed()
        {
            constraints.max_filters = max.clamp(0.0, 20.0) as usize;
        }
    });
    ui.horizontal(|ui| {
        ui.label("Frequency");
        let mut low = constraints.freq_min;
        let mut high = constraints.freq_max;
        if ui
            .add(
                egui::DragValue::new(&mut low)
                    .range(0.0..=20_000.0)
                    .speed(10.0)
                    .suffix(" Hz"),
            )
            .changed()
        {
            constraints.freq_min = low;
        }
        if ui
            .add(
                egui::DragValue::new(&mut high)
                    .range(0.0..=20_000.0)
                    .speed(10.0)
                    .suffix(" Hz"),
            )
            .changed()
        {
            constraints.freq_max = high;
        }
    });
    ui.horizontal(|ui| {
        ui.label("Gain");
        let mut low = constraints.gain_min;
        let mut high = constraints.gain_max;
        if ui
            .add(
                egui::DragValue::new(&mut low)
                    .range(-40.0..=40.0)
                    .speed(0.5)
                    .suffix(" dB"),
            )
            .changed()
        {
            constraints.gain_min = low;
        }
        if ui
            .add(
                egui::DragValue::new(&mut high)
                    .range(-40.0..=40.0)
                    .speed(0.5)
                    .suffix(" dB"),
            )
            .changed()
        {
            constraints.gain_max = high;
        }
    });
    ui.horizontal(|ui| {
        ui.label("Q");
        let mut low = constraints.q_min;
        let mut high = constraints.q_max;
        if ui
            .add(egui::DragValue::new(&mut low).range(0.0..=10.0).speed(0.1))
            .changed()
        {
            constraints.q_min = low;
        }
        if ui
            .add(egui::DragValue::new(&mut high).range(0.0..=10.0).speed(0.1))
            .changed()
        {
            constraints.q_max = high;
        }
    });
    ui.horizontal(|ui| {
        ui.label("Graphic bands");
        let mut bands = constraints
            .graphic_bands
            .iter()
            .map(|band| format!("{band:.0}"))
            .collect::<Vec<_>>()
            .join(", ");
        if ui
            .add(egui::TextEdit::singleline(&mut bands).desired_width(ui.available_width() - 90.0))
            .on_hover_text("The bands of a graphic equalizer, in hertz")
            .changed()
        {
            constraints.graphic_bands = bands
                .split(',')
                .filter_map(|part| part.trim().parse::<f64>().ok())
                .filter(|band| *band > 0.0)
                .collect();
        }
    });

    if constraints != app.session.eq_constraints {
        app.session.eq_constraints = constraints;
    }
    let _ = palette;
}

/// The change history: what can be undone, and what can be redone.
fn history(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();
    ui.label(
        RichText::new("CHANGE HISTORY")
            .font(TypeStep::Label.font())
            .color(palette.text_tertiary),
    );
    ui.horizontal(|ui| {
        if ui
            .add_enabled(app.history.can_undo(), egui::Button::new("Undo"))
            .on_hover_text("Step back to the state before the last change")
            .clicked()
        {
            if let Some(state) = app.history.undo() {
                app.session.eq_filters = state.filters;
                app.session.eq_constraints = state.constraints;
                app.session.rebuild_display_curves();
            }
        }
        if ui
            .add_enabled(app.history.can_redo(), egui::Button::new("Redo"))
            .on_hover_text("Step forward again")
            .clicked()
        {
            if let Some(state) = app.history.redo() {
                app.session.eq_filters = state.filters;
                app.session.eq_constraints = state.constraints;
                app.session.rebuild_display_curves();
            }
        }
    });
}

/// A short summary of the equalizer, for the tabs that are not the equalizer.
pub fn show_summary(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();
    let Some(phone) = app.session.eq_phone else {
        return;
    };
    let rows = app
        .session
        .eq_filters
        .get(&phone)
        .map(|rows| rows.len())
        .unwrap_or(0);
    ui.label(
        RichText::new(format!(
            "{rows} bands on {}",
            app.session.groups[phone].name
        ))
        .font(TypeStep::Body.font())
        .color(palette.text_secondary),
    );
    if ui.button("Open the equalizer").clicked() {
        app.tab = crate::app::Tab::Equalizer;
    }
}

/// The state the history keeps.
fn snapshot(app: &App) -> Snapshot {
    Snapshot {
        filters: app.session.eq_filters.clone(),
        constraints: app.session.eq_constraints.clone(),
    }
}

/// A number in the band table, typed rather than dragged.
fn number(ui: &mut Ui, value: &mut f64, low: f64, high: f64, places: usize, width: f32) -> bool {
    let mut text = match places {
        0 => format!("{value:.0}"),
        1 => format!("{value:.1}"),
        _ => format!("{value:.2}"),
    };
    let response = ui.add_sized(
        [width, 20.0],
        egui::TextEdit::singleline(&mut text).desired_width(width),
    );
    if !response.changed() {
        return false;
    }
    match text.trim().parse::<f64>() {
        Ok(parsed) => {
            *value = parsed.clamp(low, high);
            true
        }
        Err(_) => false,
    }
}
