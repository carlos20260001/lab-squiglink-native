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

    // --- the filter banks ---
    // Separate banks for the left and the right channel: the shared filters
    // shape every channel, and each side may carry filters of its own.
    if app.session.eq_constraints.two_channel {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Bank")
                    .font(TypeStep::Label.font())
                    .color(palette.text_tertiary),
            );
            for bank in [
                squiglab_core::session::EqBank::Left,
                squiglab_core::session::EqBank::Both,
                squiglab_core::session::EqBank::Right,
            ] {
                let selected = app.session.eq_bank == bank;
                if ui
                    .selectable_label(selected, bank.label())
                    .on_hover_text("Which channel's filters the table shows")
                    .clicked()
                {
                    app.session.eq_bank = bank;
                }
            }
        });
        ui.add_space(metrics::GAP);
    }

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
    // The table edits one bank at a time: the shared one, or a channel's own.
    let bank = app.session.eq_bank;
    rows.retain(|row| row.bank == bank);
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
            if number(ui, &mut row.filter.freq, 0.0, 20_000.0, 0, 56.0, "Hz") {
                changed = true;
            }
            if number(ui, &mut row.filter.gain, -40.0, 40.0, 1, 50.0, "dB") {
                changed = true;
            }
            if number(ui, &mut row.filter.q, 0.0, 20.0, 2, 36.0, "") {
                changed = true;
            }
        });
    }
    if changed {
        write_bank(app, phone, bank, rows);
        crate::sound::push_chain(app);
    }

    // --- the row controls ---
    // The buttons share the width between them, as the reference's do.
    let third = (ui.available_width() - metrics::GAP * 2.0) / 3.0;
    ui.horizontal(|ui| {
        if ui
            .add_sized([third, 28.0], egui::Button::new("+"))
            .on_hover_text("Add a band")
            .clicked()
        {
            let mut rows = app
                .session
                .eq_filters
                .get(&phone)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|row| row.bank == bank)
                .collect::<Vec<_>>();
            // The reference caps the bands at twenty, and says so on the
            // button rather than refusing the click.
            if rows.len() < squiglab_core::session::MAX_EQ_ROWS {
                rows.push(EqRow {
                    filter: Filter::new(FilterType::Peaking, 1000.0, 1.0, 0.0),
                    enabled: true,
                    bank,
                });
                write_bank(app, phone, bank, rows);
            }
        }
        if ui
            .add_sized([third, 28.0], egui::Button::new("−"))
            .on_hover_text("Remove the last band")
            .clicked()
        {
            if let Some(mut rows) = app.session.eq_filters.get(&phone).cloned() {
                rows.retain(|row| row.bank == bank);
                // A flat band says nothing, so the last one that does is the
                // one that goes; and the last row stays.
                while rows.len() > 1
                    && rows.last().is_some_and(|row| {
                        row.filter.gain == 0.0 && row.filter.freq == 0.0 && row.filter.q == 0.0
                    })
                {
                    rows.pop();
                }
                if rows.len() > 1 {
                    rows.pop();
                }
                write_bank(app, phone, bank, rows);
            }
        }
        if ui
            .add_sized([third, 28.0], egui::Button::new("Sort"))
            .on_hover_text("Order the bands by frequency")
            .clicked()
        {
            if let Some(mut rows) = app.session.eq_filters.get(&phone).cloned() {
                rows.retain(|row| row.bank == bank);
                rows.sort_by(|left, right| left.filter.freq.total_cmp(&right.filter.freq));
                write_bank(app, phone, bank, rows);
            }
        }
    });
    ui.add_space(metrics::GAP);

    // --- what a fit may produce ---
    // The reference hides the constraints behind a gear, and the column is
    // narrow enough that the room they take is worth having back.
    let constraints_open = app.constraints_open;
    ui.horizontal(|ui| {
        if ui
            .selectable_label(constraints_open, "Constraints")
            .on_hover_text("What a fit is allowed to produce")
            .clicked()
        {
            app.constraints_open = !constraints_open;
        }
    });
    if constraints_open {
        constraints(ui, app);
    }

    // --- the change history ---
    ui.add_space(metrics::GAP);
    history(ui, app);

    // --- the exports ---
    ui.add_space(metrics::GAP);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Import")
                .font(TypeStep::Label.font())
                .color(palette.text_tertiary),
        );
        ui.add(
            egui::TextEdit::singleline(&mut app.import_path)
                .hint_text("An Equalizer APO file")
                .desired_width(ui.available_width() - 60.0),
        );
    });
    ui.add_space(metrics::GAP);
    let half = (ui.available_width() - metrics::GAP) / 2.0;
    ui.horizontal(|ui| {
        if ui
            .add_sized([half, 28.0], egui::Button::new("Import"))
            .on_hover_text("Load an Equalizer APO file")
            .clicked()
        {
            import(app, phone);
        }
        if ui
            .add_sized([half, 28.0], egui::Button::new("Export"))
            .on_hover_text("Save an Equalizer APO file")
            .clicked()
        {
            match app.session.parametric_export() {
                Some((name, contents)) => app.say(format!("{name} is ready to save: {contents}")),
                None => app.say("There is no equalizer to export."),
            }
        }
        if ui
            .add_sized(
                [ui.available_width(), 28.0],
                egui::Button::new("Export Graphic EQ (Wavelet)"),
            )
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
        let mut two_channel = constraints.two_channel;
        if ui
            .checkbox(&mut two_channel, "Separate left and right filter banks")
            .on_hover_text("Let each channel carry filters of its own")
            .changed()
        {
            constraints.two_channel = two_channel;
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
    // The reference writes the shortcuts under the buttons, so they are
    // written here too.
    ui.label(
        RichText::new("Undo: Ctrl+Z / ⌘Z, Redo: Shift+Ctrl+Z / Shift+⌘Z")
            .font(TypeStep::Label.font())
            .color(palette.text_tertiary),
    );
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

/// Put a bank's rows back into a curve's whole set, leaving the other banks
/// as they were.
fn write_bank(app: &mut App, phone: usize, bank: squiglab_core::session::EqBank, rows: Vec<EqRow>) {
    let mut all = app
        .session
        .eq_filters
        .get(&phone)
        .cloned()
        .unwrap_or_default();
    all.retain(|row| row.bank != bank);
    all.extend(rows);
    app.session.eq_filters.insert(phone, all);
    app.history.record(snapshot(app));
    app.session.rebuild_display_curves();
}

/// The state the history keeps.
fn snapshot(app: &App) -> Snapshot {
    Snapshot {
        filters: app.session.eq_filters.clone(),
        constraints: app.session.eq_constraints.clone(),
    }
}

/// A number in the band table, typed rather than dragged.
fn number(
    ui: &mut Ui,
    value: &mut f64,
    low: f64,
    high: f64,
    places: usize,
    width: f32,
    unit: &str,
) -> bool {
    let mut text = match places {
        0 => format!("{value:.0}"),
        1 => format!("{value:.1}"),
        _ => format!("{value:.2}"),
    };
    let response = ui.add_sized(
        [width, 20.0],
        egui::TextEdit::singleline(&mut text).desired_width(width),
    );
    if !unit.is_empty() {
        // The unit sits beside the figure, as the reference writes it.
        ui.label(
            RichText::new(unit)
                .font(TypeStep::Label.font())
                .color(theme::current_palette().text_tertiary),
        );
    }
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

/// Load an Equalizer APO file into the curve's equalizer.
///
/// There is no file dialog, so the path is typed, as the upload panel's is.
fn import(app: &mut App, phone: usize) {
    let path = app.import_path.trim();
    if path.is_empty() {
        app.say("Name the file to import.");
        return;
    }
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(problem) => {
            app.say(format!("The file was not read: {problem}"));
            return;
        }
    };
    match app.session.import_parametric(phone, &text) {
        Ok(count) => {
            app.import_path.clear();
            app.say(format!("{count} bands were imported."));
            app.session.rebuild_display_curves();
        }
        Err(problem) => app.say(format!("The file was not imported: {problem}")),
    }
}
