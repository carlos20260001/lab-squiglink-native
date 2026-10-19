//! The constraints panel: what a fit is allowed to produce.
//!
//! The reference can say what its AutoEQ is allowed to fit: which filter
//! types, how many, over what frequency, gain and Q, and it ships presets for
//! the devices it knows. These are the rules of the program, not of the
//! picture: the panel that sets them is one of several ways to set them, and
//! the core keeps them with the rest of the program's state.

use egui::{RichText, Ui};
use squiglab_core::constraints::{Constraints, PRESETS};

use crate::theme::{self, TypeStep};

/// The controls that set the constraints, laid out as the reference lays them
/// out: the preset, then the three filter kinds, then the limits.
pub fn show(ui: &mut Ui, constraints: &mut Constraints) -> bool {
    let mut changed = false;
    let palette = theme::current_palette();

    // --- the preset ---
    // The reference's presets for particular devices are its data about those
    // devices, so only the generic ones are offered here.
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("CONSTRAINTS")
                .font(theme::font(TypeStep::Micro))
                .color(palette.text_tertiary),
        );
        let current = PRESETS
            .iter()
            .find(|preset| preset.constraints() == *constraints)
            .map(|preset| preset.id);
        let mut selected = current.unwrap_or("");
        egui::ComboBox::from_id_salt("eq_constraint_preset")
            .selected_text(if selected.is_empty() {
                "Custom".to_owned()
            } else {
                PRESETS
                    .iter()
                    .find(|preset| preset.id == selected)
                    .map(|preset| preset.label.to_owned())
                    .unwrap_or_else(|| "Custom".to_owned())
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut selected, "", "Custom");
                for preset in PRESETS {
                    ui.selectable_value(&mut selected, preset.id, preset.label);
                }
            });
        if selected != current.unwrap_or("") {
            if let Some(preset) = Constraints::preset(&selected) {
                *constraints = preset;
                changed = true;
            }
        }
    });

    // --- the filter kinds the fit may use ---
    ui.horizontal(|ui| {
        ui.label("Filters");
        for (label, allowed) in [
            ("PK", &mut constraints.allow_peaking),
            ("LSQ", &mut constraints.allow_low_shelf),
            ("HSQ", &mut constraints.allow_high_shelf),
        ] {
            let mut value = *allowed;
            if ui
                .add(egui::Checkbox::new(&mut value, label))
                .on_hover_text("Whether the fit may use this kind of filter")
                .changed()
            {
                *allowed = value;
                changed = true;
            }
        }
    });

    // --- how many filters the fit may produce ---
    ui.horizontal(|ui| {
        ui.label("Max bands");
        let mut max = constraints.max_filters as f64;
        if ui
            .add(egui::DragValue::new(&mut max).range(0.0..=20.0).speed(1.0))
            .on_hover_text("How many filters the fit may produce. Zero is no cap.")
            .changed()
        {
            constraints.max_filters = max.clamp(0.0, 20.0) as usize;
            changed = true;
        }
    });

    // --- the band of frequencies the fit may use ---
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
            .on_hover_text("The lowest frequency the fit may use. Zero is no limit.")
            .changed()
        {
            constraints.freq_min = low;
            changed = true;
        }
        if ui
            .add(
                egui::DragValue::new(&mut high)
                    .range(0.0..=20_000.0)
                    .speed(10.0)
                    .suffix(" Hz"),
            )
            .on_hover_text("The highest frequency the fit may use. Zero is no limit.")
            .changed()
        {
            constraints.freq_max = high;
            changed = true;
        }
        // The high bound never falls below the low one, as in the reference.
        if constraints.freq_max != 0.0 && constraints.freq_min > constraints.freq_max {
            constraints.freq_min = constraints.freq_max;
        }
    });

    // --- the gain the fit may use ---
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
            .on_hover_text("The lowest gain the fit may use. Zero is no limit.")
            .changed()
        {
            constraints.gain_min = low;
            changed = true;
        }
        if ui
            .add(
                egui::DragValue::new(&mut high)
                    .range(-40.0..=40.0)
                    .speed(0.5)
                    .suffix(" dB"),
            )
            .on_hover_text("The highest gain the fit may use. Zero is no limit.")
            .changed()
        {
            constraints.gain_max = high;
            changed = true;
        }
        if constraints.gain_max != 0.0 && constraints.gain_min > constraints.gain_max {
            constraints.gain_min = constraints.gain_max;
        }
    });

    // --- the Q the fit may use ---
    ui.horizontal(|ui| {
        ui.label("Q");
        let mut low = constraints.q_min;
        let mut high = constraints.q_max;
        if ui
            .add(egui::DragValue::new(&mut low).range(0.0..=10.0).speed(0.1))
            .on_hover_text("The lowest Q the fit may use. Zero is no limit.")
            .changed()
        {
            constraints.q_min = low;
            changed = true;
        }
        if ui
            .add(egui::DragValue::new(&mut high).range(0.0..=10.0).speed(0.1))
            .on_hover_text("The highest Q the fit may use. Zero is no limit.")
            .changed()
        {
            constraints.q_max = high;
            changed = true;
        }
        if constraints.q_max != 0.0 && constraints.q_min > constraints.q_max {
            constraints.q_min = constraints.q_max;
        }
    });

    // --- the bands of a graphic fit ---
    // A graphic fit places one filter on each of these frequencies rather than
    // wherever the curve deviates, so the list is the fit's shape.
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
            .on_hover_text(
                "The band frequencies of a graphic equalizer, in hertz, separated by commas.",
            )
            .changed()
        {
            let parsed = bands
                .split(',')
                .filter_map(|part| part.trim().parse::<f64>().ok())
                .filter(|band| *band > 0.0)
                .collect::<Vec<_>>();
            if parsed != constraints.graphic_bands {
                constraints.graphic_bands = parsed;
                changed = true;
            }
        }
    });

    changed
}
