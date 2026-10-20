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

/// The width of the column that holds the kind of filter.
const TYPE_COLUMN: f32 = 53.0;
/// The width of the column that holds the mark.
const MARK_COLUMN: f32 = 15.0;
/// The gap between the kind of filter and the first field.
const TYPE_GAP: f32 = 12.0;
/// The gap between the fields.
const FIELD_GAP: f32 = 5.0;
/// What a field takes off its own width for its frame.
const FIELD_FRAME: f32 = 6.0;

/// The width of a field, worked out from the width the column has.
///
/// The reference's fields are seventy-six points wide. A field here is a
/// little narrower, because the frame around it takes a little of the room,
/// and the table has to fit the column it is in.
fn field_width(available: f32) -> f32 {
    let used = MARK_COLUMN + TYPE_COLUMN + TYPE_GAP + FIELD_GAP * 2.0;
    ((available - used) / 3.0 - FIELD_FRAME).max(40.0)
}

/// The equalizer of the selected measurement.
pub fn show(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Parametric Equalizer")
                .font(TypeStep::Heading.font_heading())
                .color(palette.text_tertiary),
        );
        // The reference's head: the word "Reset", the arrow that bends back,
        // and the cog.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::click());
            crate::icons::draw(
                ui.painter(),
                rect,
                crate::icons::Icon::Settings,
                palette.text_tertiary,
            );
            if response
                .on_hover_text("The settings of the equalizer")
                .clicked()
            {
                app.constraints_open = !app.constraints_open;
            }
            ui.add_space(metrics::GAP);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
            crate::icons::draw(
                ui.painter(),
                rect,
                crate::icons::Icon::Reset,
                palette.text_tertiary,
            );
            let response = crate::music::text_button(ui, "Reset");
            if response.on_hover_text("Clear this curve's bands").clicked() {
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
    // The reference's arrangement: the measurement on a row of its own, and
    // the target and the fit beside each other below it.
    let mut selected = Some(phone);
    egui::ComboBox::from_id_salt("eq_phone")
        .width(ui.available_width())
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
    ui.add_space(metrics::GAP);
    ui.horizontal(|ui| {
        let mut target = app.session.eq_target;
        let target_label = target
            .map(|index| app.session.groups[index].name.clone())
            .unwrap_or_else(|| "Choose EQ target".to_owned());
        egui::ComboBox::from_id_salt("eq_target")
            .width(ui.available_width() - 86.0)
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
            .add_sized([76.0, 36.0], egui::Button::new("Auto EQ"))
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
    // The reference always shows ten bands, empty ones included, so a curve
    // with no equalizer of its own still has the table to fill in. The bands
    // that are there keep their places, and the empty ones come after.
    let empty = rows.len() < squiglab_core::session::MAX_EQ_ROWS;
    while rows.len() < squiglab_core::session::MAX_EQ_ROWS {
        rows.push(EqRow {
            filter: Filter::new(FilterType::Peaking, 0.0, 0.0, 0.0),
            enabled: true,
            bank,
        });
    }
    let mut changed = false;
    // The reference's rows are thirty points apart: twenty-six for the
    // field, and four between one row and the next.
    ui.spacing_mut().item_spacing.y = 4.0;

    // The reference's table: a mark, the kind of filter, and three fields of
    // seventy-six points, with the headings over the fields they belong to.
    // The headings sit at the left of the column they belong to, as the
    // reference's do.
    let field = field_width(ui.available_width());
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (heading, width) in [
            ("Type", MARK_COLUMN + TYPE_COLUMN),
            ("Frequency", field + FIELD_FRAME),
            ("Gain", field + FIELD_FRAME),
            ("Q", field + FIELD_FRAME),
        ] {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 18.0), egui::Sense::hover());
            ui.painter().text(
                rect.left_center(),
                egui::Align2::LEFT_CENTER,
                heading,
                TypeStep::Label.font(),
                palette.text_tertiary,
            );
        }
    });
    ui.add_space(TYPE_GAP);
    // The reference's fields are twenty points tall, and the rows they sit
    // in no taller. The rest of the panel's controls keep the height they
    // have always had.
    // A row of the table is twenty points tall, as the reference's rows are.
    // The style is set for the length of the table, because a row is laid out
    // with the style's own measures.
    {
        for row in rows.iter_mut() {
            ui.horizontal(|ui| {
                // The reference's fields are short: twenty points, not the
                // thirty-six the rest of the program's controls are, and the
                // fields are all the same width.
                ui.spacing_mut().interact_size.y = 26.0;
                ui.spacing_mut().item_spacing.x = 0.0;
                let mut enabled = row.enabled;
                let check =
                    ui.add_sized([MARK_COLUMN, 26.0], egui::Checkbox::new(&mut enabled, ""));
                if check
                    .on_hover_text("Include this band in the response and the exports")
                    .changed()
                {
                    row.enabled = enabled;
                    changed = true;
                }
                // A menu rather than a select: the reference writes the kind of
                // filter in plain words with a caret beside it.
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(TYPE_COLUMN, 26.0), egui::Sense::click());
                let font = TypeStep::Body.font();
                ui.painter().text(
                    rect.left_center() + egui::vec2(2.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    row.filter.filter_type.short_name(),
                    font,
                    palette.text_primary,
                );
                // The caret, at the right of the column, as the reference puts it.
                let caret = egui::Rect::from_center_size(
                    rect.right_center() - egui::vec2(8.0, 0.0),
                    egui::vec2(8.0, 5.0),
                );
                ui.painter().add(egui::Shape::convex_polygon(
                    vec![
                        egui::pos2(caret.left(), caret.top()),
                        egui::pos2(caret.right(), caret.top()),
                        egui::pos2(caret.center().x, caret.bottom()),
                    ],
                    palette.text_tertiary,
                    egui::Stroke::NONE,
                ));
                response
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
                let _ = ui.allocate_exact_size(egui::vec2(TYPE_GAP, 0.0), egui::Sense::hover());
                if number(ui, &mut row.filter.freq, 0.0, 20_000.0, 0, field, "Hz") {
                    changed = true;
                }
                let _ = ui.allocate_exact_size(egui::vec2(FIELD_GAP, 0.0), egui::Sense::hover());
                if number(ui, &mut row.filter.gain, -40.0, 40.0, 1, field, "dB") {
                    changed = true;
                }
                let _ = ui.allocate_exact_size(egui::vec2(FIELD_GAP, 0.0), egui::Sense::hover());
                if number(ui, &mut row.filter.q, 0.0, 20.0, 2, field, "") {
                    changed = true;
                }
            });
        }
    }
    if changed {
        // The empty bands the reference shows are not kept: only the ones
        // that have been filled in are.
        let kept: Vec<EqRow> = rows
            .iter()
            .filter(|row| row.filter.freq > 0.0)
            .cloned()
            .collect();
        write_bank(app, phone, bank, kept);
        crate::sound::push_chain(app);
    }
    let _ = empty;

    // --- the row controls ---
    // The reference gives half the row to the sort, and the rest to the two
    // buttons that add and take a band away.
    let half = (ui.available_width() - metrics::GAP) / 2.0;
    let third = (ui.available_width() - half - metrics::GAP * 2.0) / 2.0;
    ui.horizontal(|ui| {
        if ui
            .add_sized([third, 36.0], egui::Button::new("+"))
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
            .add_sized([third, 36.0], egui::Button::new("−"))
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
            .add_sized([half, 36.0], egui::Button::new("Sort"))
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
    // The reference hides the constraints behind the cog in the head, and the
    // column is narrow enough that the room they take is worth having back.
    let constraints_open = app.constraints_open;
    if constraints_open {
        constraints(ui, app);
    }

    // --- the change history ---
    ui.add_space(metrics::GAP);
    history(ui, app);

    // --- the exports ---
    ui.add_space(metrics::GAP);
    // The file is chosen with the system's own dialog, and its name is
    // shown beside it, as the reference's file input shows it.
    ui.horizontal(|ui| {
        if ui
            .button("Choose a file")
            .on_hover_text("Choose the file with the system's own dialog")
            .clicked()
        {
            app.ask_for_file(
                crate::app::Pick::ImportParametric,
                "An Equalizer APO file",
                &["txt", "cfg", "apo"],
            );
        }
        let name = app
            .pending_import
            .as_ref()
            .map(|chosen| chosen.name.clone())
            .unwrap_or_default();
        if !name.is_empty() {
            ui.label(
                RichText::new(name)
                    .font(TypeStep::Label.font())
                    .color(palette.text_tertiary),
            );
        }
    });
    ui.add_space(metrics::GAP);
    let half = (ui.available_width() - metrics::GAP) / 2.0;
    ui.horizontal(|ui| {
        if ui
            .add_sized([half, 36.0], egui::Button::new("Import"))
            .on_hover_text("Load an Equalizer APO file")
            .clicked()
        {
            import(app, phone);
        }
        let export = ui.add_sized([half, 36.0], egui::Button::new("Export"));
        // The reference puts the download mark at the right of the button.
        let icon = egui::Rect::from_center_size(
            export.rect.right_center() - egui::vec2(14.0, 0.0),
            egui::vec2(16.0, 16.0),
        );
        crate::icons::draw(
            ui.painter(),
            icon,
            crate::icons::Icon::Download,
            palette.text_tertiary,
        );
        if export.on_hover_text("Save an Equalizer APO file").clicked() {
            match app.session.parametric_export() {
                Some((name, contents)) => app.say(format!("{name} is ready to save: {contents}")),
                None => app.say("There is no equalizer to export."),
            }
        }
    });
    // The reference puts the Wavelet export on a row of its own, the whole
    // width of the column, and writes it as plain words.
    ui.add_space(metrics::GAP);
    let width = ui.available_width();
    if crate::music::text_button_in(ui, "Export Graphic EQ (Wavelet)", Some(width))
        .on_hover_text("Save a Wavelet file")
        .clicked()
    {
        match app.session.graphic_eq_export() {
            Some((name, contents)) => app.say(format!("{name} is ready to save: {contents}")),
            None => app.say("There is no equalizer to export."),
        }
    }
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
        RichText::new("Change History")
            .font(TypeStep::Title.font_bold())
            .color(palette.text_tertiary.gamma_multiply(0.8)),
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
    // The state before the change is what the history keeps, so that undo
    // puts it back.
    let before = snapshot(app);
    let mut all = app
        .session
        .eq_filters
        .get(&phone)
        .cloned()
        .unwrap_or_default();
    all.retain(|row| row.bank != bank);
    all.extend(rows);
    app.session.eq_filters.insert(phone, all);
    app.history.record(before);
    app.session.rebuild_display_curves();
}

/// The rows of the bank that is being edited.
pub fn bank_rows(app: &App) -> Vec<(usize, EqRow)> {
    let Some(phone) = app.session.eq_phone else {
        return Vec::new();
    };
    let bank = app.session.eq_bank;
    app.session
        .eq_filters
        .get(&phone)
        .map(|rows| {
            rows.iter()
                .enumerate()
                .filter(|(_, row)| row.bank == bank)
                .map(|(index, row)| (index, row.clone()))
                .collect()
        })
        .unwrap_or_default()
}

/// Draw a new band, as the reference's graph does when a band is dragged out
/// of the curve. The row it lands in is given back, so that it can be moved.
pub fn add_band_at(app: &mut App, hz: f64, gain: f64) -> Option<usize> {
    let phone = app.session.eq_phone?;
    let bank = app.session.eq_bank;
    let mut rows = bank_rows(app)
        .into_iter()
        .map(|(_, row)| row)
        .collect::<Vec<_>>();
    if rows.len() >= squiglab_core::session::MAX_EQ_ROWS {
        return None;
    }
    rows.push(EqRow {
        filter: Filter::new(
            FilterType::Peaking,
            hz.clamp(20.0, 20_000.0).round(),
            1.0,
            gain.clamp(-40.0, 40.0),
        ),
        enabled: true,
        bank,
    });
    let index = rows.len() - 1;
    write_bank(app, phone, bank, rows);
    crate::sound::push_chain(app);
    Some(index)
}

/// Move a band, as the reference's graph does when a band is dragged.
pub fn move_band(app: &mut App, row: usize, hz: f64, gain: f64) {
    let Some(phone) = app.session.eq_phone else {
        return;
    };
    let bank = app.session.eq_bank;
    let mut rows = bank_rows(app)
        .into_iter()
        .map(|(_, row)| row)
        .collect::<Vec<_>>();
    let Some(slot) = rows.get_mut(row) else {
        return;
    };
    slot.filter.freq = hz.clamp(20.0, 20_000.0).round();
    slot.filter.gain = gain.clamp(-40.0, 40.0);
    write_bank(app, phone, bank, rows);
    crate::sound::push_chain(app);
}

/// The kind of filter a band has, turned round as the reference turns it.
pub fn cycle_band_type(app: &mut App, row: usize) {
    let Some(phone) = app.session.eq_phone else {
        return;
    };
    let bank = app.session.eq_bank;
    let mut rows = bank_rows(app)
        .into_iter()
        .map(|(_, row)| row)
        .collect::<Vec<_>>();
    let Some(slot) = rows.get_mut(row) else {
        return;
    };
    slot.filter.filter_type = match slot.filter.filter_type {
        FilterType::Peaking => FilterType::LowShelf,
        FilterType::LowShelf => FilterType::HighShelf,
        FilterType::HighShelf => FilterType::Peaking,
    };
    write_bank(app, phone, bank, rows);
    crate::sound::push_chain(app);
}

/// The width of a band.
pub fn set_band_q(app: &mut App, row: usize, q: f64) {
    let Some(phone) = app.session.eq_phone else {
        return;
    };
    let bank = app.session.eq_bank;
    let mut rows = bank_rows(app)
        .into_iter()
        .map(|(_, row)| row)
        .collect::<Vec<_>>();
    let Some(slot) = rows.get_mut(row) else {
        return;
    };
    slot.filter.q = q.clamp(0.0, 20.0);
    write_bank(app, phone, bank, rows);
    crate::sound::push_chain(app);
}

/// Add a band to the curve that is being looked at.
///
/// The reference's tone generator adds a band at the frequency it is
/// playing, so that a tone can be heard at the place a curve needs help.
pub fn add_band(app: &mut App, freq: f64, gain: f64, q: f64) -> bool {
    let Some(phone) = app.session.eq_phone else {
        app.say("Choose a measurement to give it an equalizer.");
        return false;
    };
    let bank = squiglab_core::session::EqBank::Both;
    let mut rows = app
        .session
        .eq_filters
        .get(&phone)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.bank == bank)
        .collect::<Vec<_>>();
    if rows.len() >= squiglab_core::session::MAX_EQ_ROWS {
        app.say(format!(
            "There are already {} bands, which is the most there may be.",
            squiglab_core::session::MAX_EQ_ROWS
        ));
        return false;
    }
    rows.push(EqRow {
        filter: Filter::new(FilterType::Peaking, freq, q, gain),
        enabled: true,
        bank,
    });
    write_bank(app, phone, bank, rows);
    crate::sound::push_chain(app);
    true
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
    let palette = theme::current_palette();
    let mut text = match places {
        0 => format!("{value:.0}"),
        1 => {
            // The reference writes the figure without the zero it does not
            // need: "6", not "6.0".
            let mut text = format!("{value:.1}");
            if text.ends_with(".0") {
                text.pop();
                text.pop();
            }
            text
        }
        _ => {
            // The reference does not write the zeros it does not need.
            let mut text = format!("{value:.2}");
            if text.contains('.') {
                while text.ends_with('0') {
                    text.pop();
                }
                if text.ends_with('.') {
                    text.pop();
                }
            }
            text
        }
    };
    // The unit sits inside the field, at its right, as the reference writes
    // it, and the figure is in the accent.
    let unit_font = TypeStep::Label.font();
    let unit_font_for_width = unit_font.clone();
    let unit_width = if unit.is_empty() {
        0.0
    } else {
        ui.painter()
            .layout_no_wrap(unit.to_owned(), unit_font_for_width, palette.text_tertiary)
            .size()
            .x
            + 6.0
    };
    let response = ui.add_sized(
        [width, 26.0],
        egui::TextEdit::singleline(&mut text)
            .desired_width(width - unit_width)
            .text_color(palette.text_input),
    );
    if !unit.is_empty() {
        let galley = ui
            .painter()
            .layout_no_wrap(unit.to_owned(), unit_font, palette.text_tertiary);
        let at = response.rect.right_center()
            + egui::vec2(-galley.size().x - 4.0, -galley.size().y * 0.5);
        ui.painter().galley(at, galley, palette.text_tertiary);
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
/// The file is the one that was chosen with the system's own dialog.
fn import(app: &mut App, phone: usize) {
    let Some(chosen) = app.pending_import.take() else {
        app.say("Choose a file to import.");
        return;
    };
    let Some(text) = chosen.text() else {
        app.say("That file could not be read.");
        return;
    };
    match app.session.import_parametric(phone, &text) {
        Ok(count) => {
            app.say(format!("{count} bands were imported."));
            app.session.rebuild_display_curves();
        }
        Err(problem) => app.say(format!("The file was not imported: {problem}")),
    }
}
