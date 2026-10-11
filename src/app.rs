use crigraph_native::color::target_color;
use crigraph_native::data::{self, CurveInfo, FRPoint, PhoneBook};
use crigraph_native::graph::{export_svg as render_svg, GraphCanvas, PlotCurve, ZoomRegion};
use crigraph_native::processing::{self, NormParams, SmoothingParams};
use egui::{Color32, Context, RichText};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const MAX_ACTIVE_CURVES: usize = 8;
const SHARE_URI_PREFIX: &str = "cringraph://state/v1/";
const MAX_SHARE_URI_LENGTH: usize = 65_536;

struct TutorialSegment {
    title: &'static str,
    range_hz: (f32, f32),
    description: &'static str,
}

const TUTORIAL_SEGMENTS: [TutorialSegment; 7] = [
    TutorialSegment {
        title: "Sub bass",
        range_hz: (20.0, 60.0),
        description: "The lowest tones provide rumble and physical weight. A roll-off here can make bass feel lighter.",
    },
    TutorialSegment {
        title: "Mid bass",
        range_hz: (60.0, 200.0),
        description: "This region carries bass impact and much of its warmth. Excess energy can sound thick or boomy.",
    },
    TutorialSegment {
        title: "Lower midrange",
        range_hz: (200.0, 500.0),
        description: "Lower voices and instruments have important fundamentals here. Too much energy may sound muddy.",
    },
    TutorialSegment {
        title: "Upper midrange",
        range_hz: (500.0, 2000.0),
        description: "Vocal body and many instrument fundamentals sit here. Compare broad changes rather than isolated peaks.",
    },
    TutorialSegment {
        title: "Presence region",
        range_hz: (2000.0, 5000.0),
        description: "This range strongly affects vocal presence, clarity, and perceived attack. Large peaks can become shouty.",
    },
    TutorialSegment {
        title: "Mid treble",
        range_hz: (5000.0, 10000.0),
        description: "Treble detail and consonant brightness are influenced here; measurement placement can change narrow peaks.",
    },
    TutorialSegment {
        title: "Air",
        range_hz: (10000.0, 20000.0),
        description: "The top octave contributes a sense of openness. High-frequency measurements are especially sensitive to fit.",
    },
];

struct DisplayCurve {
    group_index: usize,
    label: String,
    points: Vec<FRPoint>,
    target: bool,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
enum ChannelMode {
    Left,
    Right,
    #[default]
    Both,
    Average,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
struct Preferences {
    active: Vec<SavedGroup>,
    baseline: Option<String>,
    zoom: ZoomRegion,
    phon_normalization: bool,
    target_phon: f32,
    target_hz: f32,
    smooth_level: usize,
    show_labels: bool,
    inspect_mode: bool,
    dark_mode: bool,
    color_offset: usize,
    show_measurements: bool,
    show_manager: bool,
    label_positions: BTreeMap<String, [f32; 2]>,
    tutorial_segment: Option<usize>,
    y_scale: f32,
    curve_color_offsets: BTreeMap<String, usize>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            active: Vec::new(),
            baseline: None,
            zoom: ZoomRegion::Full,
            phon_normalization: true,
            target_phon: 60.0,
            target_hz: 500.0,
            smooth_level: 5,
            show_labels: false,
            inspect_mode: false,
            dark_mode: true,
            color_offset: 0,
            show_measurements: true,
            show_manager: true,
            label_positions: BTreeMap::new(),
            tutorial_segment: None,
            y_scale: 1.0,
            curve_color_offsets: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct SavedGroup {
    key: String,
    variant: usize,
    channel_mode: ChannelMode,
    hidden: bool,
    pinned: bool,
    offset: f32,
}

#[derive(Clone, Debug)]
struct MeasurementVariant {
    name: String,
    curve_indices: Vec<usize>,
}

#[derive(Clone, Debug)]
struct MeasurementGroup {
    brand: String,
    brand_suffix: Option<String>,
    name: String,
    variants: Vec<MeasurementVariant>,
    target: bool,
}

pub struct CrinGraphApp {
    curves: Vec<CurveInfo>,
    groups: Vec<MeasurementGroup>,
    active: BTreeSet<usize>,
    active_targets: BTreeSet<usize>,
    hidden: BTreeSet<usize>,
    pinned: BTreeSet<usize>,
    baseline: Option<usize>,
    offsets: BTreeMap<usize, f32>,
    channel_modes: BTreeMap<usize, ChannelMode>,
    variant_choices: BTreeMap<usize, usize>,
    display_curves: Vec<DisplayCurve>,
    search: String,
    zoom: ZoomRegion,
    phon_normalization: bool,
    target_phon: f32,
    target_hz: f32,
    smooth_level: usize,
    show_labels: bool,
    inspect_mode: bool,
    dark_mode: bool,
    color_offset: usize,
    show_measurements: bool,
    show_manager: bool,
    label_positions: BTreeMap<String, [f32; 2]>,
    tutorial_segment: Option<usize>,
    y_scale: f32,
    curve_color_offsets: BTreeMap<String, usize>,
    dirty: bool,
    loaded: bool,
    load_error: Option<String>,
    processing_error: Option<String>,
    settings_error: Option<String>,
    last_saved_preferences: Option<Preferences>,
    screenshot_pending: bool,
    screenshot_error: Option<String>,
    screenshot_message: Option<String>,
    svg_error: Option<String>,
    svg_message: Option<String>,
    shared_state: Option<String>,
    share_window_open: bool,
    share_input: String,
    share_message: Option<String>,
    debug_hooks: bool,
}

impl Default for CrinGraphApp {
    fn default() -> Self {
        Self {
            curves: Vec::new(),
            groups: Vec::new(),
            active: BTreeSet::new(),
            active_targets: BTreeSet::new(),
            hidden: BTreeSet::new(),
            pinned: BTreeSet::new(),
            baseline: None,
            offsets: BTreeMap::new(),
            channel_modes: BTreeMap::new(),
            variant_choices: BTreeMap::new(),
            display_curves: Vec::new(),
            search: String::new(),
            zoom: ZoomRegion::Full,
            phon_normalization: true,
            target_phon: 60.0,
            target_hz: 500.0,
            smooth_level: 5,
            show_labels: false,
            inspect_mode: false,
            dark_mode: true,
            color_offset: 0,
            show_measurements: true,
            show_manager: true,
            label_positions: BTreeMap::new(),
            tutorial_segment: None,
            y_scale: 1.0,
            curve_color_offsets: BTreeMap::new(),
            dirty: true,
            loaded: false,
            load_error: None,
            processing_error: None,
            settings_error: None,
            last_saved_preferences: None,
            screenshot_pending: false,
            screenshot_error: None,
            screenshot_message: None,
            svg_error: None,
            svg_message: None,
            shared_state: None,
            share_window_open: false,
            share_input: String::new(),
            share_message: None,
            debug_hooks: false,
        }
    }
}

impl CrinGraphApp {
    pub fn with_debug_hooks(shared_state: Option<String>, debug_hooks: bool) -> Self {
        Self {
            shared_state,
            debug_hooks,
            ..Self::default()
        }
    }

    fn load_measurements(&mut self) -> anyhow::Result<()> {
        let (mut curves, _) = data::load_data(Path::new("data"))?;
        let mut phone_books = vec![data::load_phone_book(Path::new("data/phone_book.json"))?];
        if Path::new("data_hp").is_dir() {
            let (headphone_curves, _) = data::load_data(Path::new("data_hp"))?;
            curves.extend(headphone_curves);
            if Path::new("data_hp/phone_book.json").is_file() {
                phone_books.push(data::load_phone_book(Path::new("data_hp/phone_book.json"))?);
            }
        }
        curves.sort_by(|left, right| {
            left.model
                .cmp(&right.model)
                .then(left.variant.cmp(&right.variant))
        });
        self.groups = build_groups(&phone_books, &curves);
        if self.debug_hooks {
            eprintln!(
                "CrinGraph loaded {} curves across {} measurement groups.",
                curves.len(),
                self.groups.len()
            );
        }
        let restored_preferences = if let Some(shared_state) = self.shared_state.take() {
            match decode_share_uri(&shared_state).and_then(|preferences| {
                self.apply_preferences(preferences)
                    .then_some(())
                    .ok_or_else(|| anyhow::anyhow!("shared state contains invalid settings"))
            }) {
                Ok(()) => true,
                Err(error) => {
                    self.share_message = Some(format!("Unable to restore shared state: {error:#}"));
                    false
                }
            }
        } else {
            match read_preferences() {
                Ok(Some(preferences)) => self.apply_preferences(preferences),
                Ok(None) => false,
                Err(error) => {
                    self.settings_error = Some(format!("Unable to load preferences: {error:#}"));
                    false
                }
            }
        };
        if self.active.is_empty() && self.active_targets.is_empty() && !restored_preferences {
            self.active = self
                .groups
                .iter()
                .enumerate()
                .filter(|(_, group)| group.name == "HTH67")
                .map(|(index, _)| index)
                .collect();
            if self.active.is_empty() {
                if let Some(index) = self.groups.iter().position(|group| group.target) {
                    self.active_targets.insert(index);
                }
            }
        }
        self.curves = curves;
        self.loaded = true;
        self.dirty = true;
        self.last_saved_preferences = Some(self.preferences());
        Ok(())
    }

    fn apply_preferences(&mut self, preferences: Preferences) -> bool {
        if !preferences.target_phon.is_finite()
            || !(0.0..=100.0).contains(&preferences.target_phon)
            || !preferences.target_hz.is_finite()
            || !(20.0..=20000.0).contains(&preferences.target_hz)
            || preferences.smooth_level > 12
            || !preferences.y_scale.is_finite()
            || !(0.35..=4.0).contains(&preferences.y_scale)
            || preferences
                .active
                .iter()
                .filter(|saved| {
                    self.groups
                        .iter()
                        .any(|group| !group.target && group_key(group) == saved.key)
                })
                .count()
                > MAX_ACTIVE_CURVES
            || preferences
                .active
                .iter()
                .any(|group| !group.offset.is_finite())
            || preferences.label_positions.values().any(|position| {
                position
                    .iter()
                    .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
            })
            || preferences
                .tutorial_segment
                .is_some_and(|index| index >= TUTORIAL_SEGMENTS.len())
        {
            self.settings_error =
                Some("Unable to load preferences: invalid settings values".into());
            return false;
        }
        self.active.clear();
        self.active_targets.clear();
        self.hidden.clear();
        self.pinned.clear();
        self.offsets.clear();
        self.channel_modes.clear();
        self.variant_choices.clear();
        self.baseline = None;
        self.label_positions = preferences.label_positions.clone();
        self.tutorial_segment = preferences.tutorial_segment;
        self.y_scale = preferences.y_scale;
        self.curve_color_offsets = preferences.curve_color_offsets.clone();
        for (index, group) in self.groups.iter().enumerate() {
            if let Some(saved) = preferences
                .active
                .iter()
                .find(|saved| saved.key == group_key(group))
            {
                if group.target {
                    self.active_targets.insert(index);
                } else {
                    self.active.insert(index);
                }
                self.variant_choices.insert(
                    index,
                    saved.variant.min(group.variants.len().saturating_sub(1)),
                );
                self.channel_modes.insert(index, saved.channel_mode);
                if saved.hidden {
                    self.hidden.insert(index);
                }
                if saved.pinned {
                    self.pinned.insert(index);
                }
                if saved.offset != 0.0 {
                    self.offsets.insert(index, saved.offset);
                }
            }
        }
        self.baseline = preferences.baseline.and_then(|key| {
            self.groups.iter().enumerate().position(|(index, group)| {
                (self.active.contains(&index) || self.active_targets.contains(&index))
                    && group_key(group) == key
            })
        });
        self.zoom = preferences.zoom;
        self.phon_normalization = preferences.phon_normalization;
        self.target_phon = preferences.target_phon;
        self.target_hz = preferences.target_hz;
        self.smooth_level = preferences.smooth_level;
        self.show_labels = preferences.show_labels;
        self.inspect_mode = preferences.inspect_mode;
        self.dark_mode = preferences.dark_mode;
        self.color_offset = preferences.color_offset;
        self.show_measurements = preferences.show_measurements;
        self.show_manager = preferences.show_manager;
        true
    }

    fn preferences(&self) -> Preferences {
        Preferences {
            active: self
                .active
                .iter()
                .chain(self.active_targets.iter())
                .map(|&index| SavedGroup {
                    key: group_key(&self.groups[index]),
                    variant: self.variant_choices.get(&index).copied().unwrap_or(0),
                    channel_mode: self.channel_modes.get(&index).copied().unwrap_or_default(),
                    hidden: self.hidden.contains(&index),
                    pinned: self.pinned.contains(&index),
                    offset: self.offsets.get(&index).copied().unwrap_or(0.0),
                })
                .collect(),
            baseline: self.baseline.map(|index| group_key(&self.groups[index])),
            zoom: self.zoom,
            phon_normalization: self.phon_normalization,
            target_phon: self.target_phon,
            target_hz: self.target_hz,
            smooth_level: self.smooth_level,
            show_labels: self.show_labels,
            inspect_mode: self.inspect_mode,
            dark_mode: self.dark_mode,
            color_offset: self.color_offset,
            show_measurements: self.show_measurements,
            show_manager: self.show_manager,
            label_positions: self.label_positions.clone(),
            tutorial_segment: self.tutorial_segment,
            y_scale: self.y_scale,
            curve_color_offsets: self.curve_color_offsets.clone(),
        }
    }

    fn persist_preferences_if_changed(&mut self) {
        let preferences = self.preferences();
        if self.last_saved_preferences.as_ref() == Some(&preferences) {
            return;
        }
        self.last_saved_preferences = Some(preferences.clone());
        match write_preferences(&preferences) {
            Ok(()) => self.settings_error = None,
            Err(error) => {
                self.settings_error = Some(format!("Unable to save preferences: {error:#}"));
            }
        }
    }

    fn rebuild_display_curves(&mut self) {
        let mut output = Vec::with_capacity(self.active.len());
        self.processing_error = None;
        for &group_index in self.active.iter().chain(self.active_targets.iter()) {
            if self.hidden.contains(&group_index) && self.baseline != Some(group_index) {
                continue;
            }
            let group = &self.groups[group_index];
            let variant_index = self
                .variant_choices
                .get(&group_index)
                .copied()
                .unwrap_or(0)
                .min(group.variants.len().saturating_sub(1));
            let variant = &group.variants[variant_index];
            let channel_mode = self
                .channel_modes
                .get(&group_index)
                .copied()
                .unwrap_or_default();
            let channels = match prepare_channels(
                &self.curves,
                &variant.curve_indices,
                channel_mode,
                self.smooth_level,
            ) {
                Ok(channels) => channels,
                Err(error) => {
                    self.processing_error = Some(format!("{}: {error}", group.name));
                    self.display_curves.clear();
                    self.dirty = false;
                    return;
                }
            };
            if channels.is_empty() {
                continue;
            }
            let normalization_channels = match prepare_channels(
                &self.curves,
                &variant.curve_indices,
                ChannelMode::Both,
                self.smooth_level,
            ) {
                Ok(channels) => channels,
                Err(error) => {
                    self.processing_error = Some(format!("{}: {error}", group.name));
                    self.display_curves.clear();
                    self.dirty = false;
                    return;
                }
            };
            let channel_points: Vec<&[FRPoint]> = normalization_channels
                .iter()
                .map(|(_, points)| points.as_slice())
                .collect();
            let normalization_offset = if self.phon_normalization {
                processing::average_curves(&channel_points).and_then(|average| {
                    let params = NormParams::for_curve(&average)?;
                    processing::normalize(&average, self.target_phon, &params)
                })
            } else {
                processing::normalize_by_hz(&channel_points, self.target_hz)
            };
            match normalization_offset {
                Ok(offset) => {
                    let manual_offset = self.offsets.get(&group_index).copied().unwrap_or(0.0);
                    for (channel_name, mut points) in channels {
                        for point in &mut points {
                            point.db += offset + manual_offset;
                        }
                        output.push(DisplayCurve {
                            group_index,
                            label: if channel_name.is_empty() {
                                format!("{} · {}", group.name, variant.name)
                            } else {
                                format!("{} · {} · {}", group.name, variant.name, channel_name)
                            },
                            points,
                            target: group.target,
                        });
                    }
                }
                Err(error) => {
                    self.processing_error = Some(format!("{}: {error}", group.name));
                    self.display_curves.clear();
                    self.dirty = false;
                    return;
                }
            }
        }
        if let Some(baseline) = self.baseline {
            if let Err(error) = apply_baseline(&mut output, baseline) {
                self.processing_error = Some(format!("Baseline: {error}"));
                self.display_curves.clear();
                self.dirty = false;
                return;
            }
        }
        output.retain(|curve| !self.hidden.contains(&curve.group_index));
        self.display_curves = output;
        self.dirty = false;
    }

    fn toggle_curve(&mut self, index: usize) {
        let is_target = self.groups[index].target;
        let is_selected = if is_target {
            self.active_targets.contains(&index)
        } else {
            self.active.contains(&index)
        };
        if is_selected && self.pinned.contains(&index) {
            return;
        }
        if is_selected {
            if is_target {
                self.active_targets.remove(&index);
            } else {
                self.active.remove(&index);
            }
            self.hidden.remove(&index);
            self.pinned.remove(&index);
            self.offsets.remove(&index);
            self.curve_color_offsets
                .remove(&group_key(&self.groups[index]));
            if self.baseline == Some(index) {
                self.baseline = None;
            }
        } else if is_target || self.active.len() < MAX_ACTIVE_CURVES {
            if is_target {
                self.active_targets.insert(index);
            } else {
                self.active.insert(index);
            }
            self.variant_choices.entry(index).or_insert(0);
            self.channel_modes.entry(index).or_default();
        }
        self.dirty = true;
    }

    fn curve_color(&self, index: usize) -> Color32 {
        let group = &self.groups[index];
        if group.target {
            target_color()
        } else {
            let personal_offset = self
                .curve_color_offsets
                .get(&group_key(group))
                .copied()
                .unwrap_or_default();
            crigraph_native::color::get_curve_color_with_offset(
                index,
                self.color_offset.wrapping_add(personal_offset),
            )
        }
    }

    fn toolbar(&mut self, ctx: &Context) {
        let previous_processing_settings = (
            self.phon_normalization,
            self.target_phon,
            self.target_hz,
            self.smooth_level,
        );
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("CrinGraph");
                ui.separator();
                ui.add_enabled_ui(self.tutorial_segment.is_none(), |ui| {
                    for (region, label) in [
                        (ZoomRegion::Full, "Full"),
                        (ZoomRegion::Bass, "Bass"),
                        (ZoomRegion::Mids, "Mids"),
                        (ZoomRegion::Treble, "Treble"),
                    ] {
                        ui.selectable_value(&mut self.zoom, region, label);
                    }
                });
                ui.separator();
                ui.selectable_value(&mut self.phon_normalization, true, "Normalize: phon");
                ui.add(
                    egui::DragValue::new(&mut self.target_phon)
                        .range(0.0..=100.0)
                        .speed(1.0)
                        .suffix(" phon"),
                );
                ui.selectable_value(&mut self.phon_normalization, false, "Hz");
                ui.add(
                    egui::DragValue::new(&mut self.target_hz)
                        .range(20.0..=20000.0)
                        .speed(5.0)
                        .suffix(" Hz"),
                );
                ui.separator();
                ui.label("Smooth");
                ui.add(egui::Slider::new(&mut self.smooth_level, 0..=12).suffix(" /oct"));
                ui.checkbox(&mut self.show_labels, "Labels")
                    .on_hover_text("Curve labels are auto-spaced; drag a label to position it.");
                ui.checkbox(&mut self.inspect_mode, "Inspect");
                ui.checkbox(&mut self.debug_hooks, "Debug hooks")
                    .on_hover_text("Show egui widget diagnostics when hovering controls.");
                if ui
                    .button(if self.dark_mode {
                        "Light theme"
                    } else {
                        "Dark theme"
                    })
                    .clicked()
                {
                    self.dark_mode = !self.dark_mode;
                }
                if ui
                    .button("Recolor")
                    .on_hover_text("Cycle the measurement curve colors.")
                    .clicked()
                {
                    self.color_offset = self.color_offset.wrapping_add(1);
                }
                if ui
                    .button("Export PNG")
                    .on_hover_text("Save a PNG of the current application window.")
                    .clicked()
                {
                    self.screenshot_pending = true;
                    self.screenshot_error = None;
                }
                if ui
                    .button("Export SVG")
                    .on_hover_text("Save the current graph as a scalable vector image.")
                    .clicked()
                {
                    self.export_svg();
                }
                if ui.button("Share state").clicked() {
                    self.share_window_open = true;
                    self.share_message = None;
                }
                if ui
                    .button(if self.tutorial_segment.is_some() {
                        "Close guide"
                    } else {
                        "Frequency guide"
                    })
                    .clicked()
                {
                    self.tutorial_segment = self.tutorial_segment.map_or(Some(0), |_| None);
                }
                ui.separator();
                if ui
                    .selectable_label(self.show_measurements, "Measurements")
                    .on_hover_text("Show or hide the measurement and target selector.")
                    .clicked()
                {
                    self.show_measurements = !self.show_measurements;
                }
                if ui
                    .selectable_label(self.show_manager, "Curve manager")
                    .on_hover_text("Show or hide the active curve controls.")
                    .clicked()
                {
                    self.show_manager = !self.show_manager;
                }
            });
        });
        ctx.set_visuals(if self.dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });
        self.dirty |= previous_processing_settings
            != (
                self.phon_normalization,
                self.target_phon,
                self.target_hz,
                self.smooth_level,
            );
    }

    fn share_window(&mut self, ctx: &Context) {
        if !self.share_window_open {
            return;
        }
        let mut window_open = self.share_window_open;
        egui::Window::new("Share graph state")
            .open(&mut window_open)
            .resizable(true)
            .default_width(480.0)
            .show(ctx, |ui| {
                ui.label("Copy this versioned state URI, or paste one to restore a graph.");
                if ui.button("Copy current state URI").clicked() {
                    match encode_share_uri(&self.preferences()) {
                        Ok(uri) => {
                            ctx.copy_text(uri.clone());
                            self.share_input = uri;
                            self.share_message =
                                Some("State URI copied to the clipboard.".to_owned());
                        }
                        Err(error) => {
                            self.share_message = Some(format!("Unable to encode state: {error:#}"));
                        }
                    }
                }
                ui.add(
                    egui::TextEdit::multiline(&mut self.share_input)
                        .desired_rows(4)
                        .hint_text("cringraph://state/v1/..."),
                );
                if ui.button("Restore pasted state").clicked() {
                    match self.restore_shared_state(&self.share_input.clone()) {
                        Ok(()) => self.share_message = Some("Shared state restored.".to_owned()),
                        Err(error) => {
                            self.share_message = Some(format!("Unable to restore state: {error:#}"))
                        }
                    }
                }
                if let Some(message) = &self.share_message {
                    ui.label(message);
                }
            });
        self.share_window_open = window_open;
    }

    fn tutorial_window(&mut self, ctx: &Context) {
        let Some(mut selected) = self.tutorial_segment else {
            return;
        };
        let mut open = true;
        let mut finished = false;
        egui::Window::new("Frequency response guide")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_width(340.0)
            .show(ctx, |ui| {
                let segment = &TUTORIAL_SEGMENTS[selected];
                ui.heading(segment.title);
                ui.label(format!(
                    "{:.0}–{:.0} Hz",
                    segment.range_hz.0, segment.range_hz.1
                ));
                ui.add_space(6.0);
                ui.label(segment.description);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(selected > 0, egui::Button::new("Previous"))
                        .clicked()
                    {
                        selected -= 1;
                    }
                    ui.label(format!("{} / {}", selected + 1, TUTORIAL_SEGMENTS.len()));
                    if ui
                        .add_enabled(
                            selected + 1 < TUTORIAL_SEGMENTS.len(),
                            egui::Button::new("Next"),
                        )
                        .clicked()
                    {
                        selected += 1;
                    }
                    if ui.button("Finish").clicked() {
                        finished = true;
                    }
                });
            });
        self.tutorial_segment = (open && !finished).then_some(selected);
    }

    fn restore_shared_state(&mut self, uri: &str) -> anyhow::Result<()> {
        let preferences = decode_share_uri(uri)?;
        if !self.apply_preferences(preferences) {
            anyhow::bail!("shared state contains invalid settings");
        }
        self.dirty = true;
        self.settings_error = None;
        self.share_message = None;
        Ok(())
    }

    fn selector_panel(&mut self, ctx: &Context) {
        if !self.show_measurements {
            return;
        }
        egui::SidePanel::left("selector")
            .resizable(true)
            .default_width(250.0)
            .width_range(190.0..=420.0)
            .show(ctx, |ui| {
                ui.heading("Measurements");
                ui.label(format!(
                    "{} selected (limit {MAX_ACTIVE_CURVES})",
                    self.active.len()
                ));
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text("Search measurements and targets"),
                );
                ui.separator();

                if let Some(error) = &self.load_error {
                    ui.colored_label(Color32::LIGHT_RED, error);
                    if ui.button("Retry loading").clicked() {
                        self.loaded = false;
                        self.load_error = None;
                    }
                    return;
                }

                let mut measurements = BTreeMap::<String, Vec<(usize, String)>>::new();
                let mut targets = BTreeMap::<String, Vec<(usize, String)>>::new();
                for (index, group) in self.groups.iter().enumerate() {
                    let brand = group
                        .brand_suffix
                        .as_ref()
                        .map(|suffix| format!("{} {suffix}", group.brand))
                        .unwrap_or_else(|| group.brand.clone());
                    if matches_search(&format!("{brand} {}", group.name), &self.search) {
                        let groups = if group.target {
                            &mut targets
                        } else {
                            &mut measurements
                        };
                        groups
                            .entry(brand)
                            .or_default()
                            .push((index, group.name.clone()));
                    }
                }
                let mut toggles = Vec::new();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (brand, models) in measurements {
                        ui.collapsing(format!("{brand} ({})", models.len()), |ui| {
                            for (index, model_name) in models {
                                let mut selected = self.active.contains(&index);
                                let response = ui.add_enabled(
                                    !self.pinned.contains(&index)
                                        && (selected || self.active.len() < MAX_ACTIVE_CURVES),
                                    egui::Checkbox::new(&mut selected, model_name),
                                );
                                if response.changed() {
                                    toggles.push(index);
                                }
                            }
                        });
                    }
                    ui.separator();
                    ui.heading("Targets");
                    ui.label(format!("{} selected", self.active_targets.len()));
                    for (category, models) in targets {
                        ui.collapsing(format!("{category} ({})", models.len()), |ui| {
                            for (index, target_name) in models {
                                let mut selected = self.active_targets.contains(&index);
                                if ui
                                    .add_enabled(
                                        !self.pinned.contains(&index) || !selected,
                                        egui::Checkbox::new(&mut selected, target_name),
                                    )
                                    .on_hover_text("Target curves do not use a measurement slot.")
                                    .changed()
                                {
                                    toggles.push(index);
                                }
                            }
                        });
                    }
                    for index in toggles {
                        self.toggle_curve(index);
                    }
                });
            });
    }

    fn manager_panel(&mut self, ctx: &Context) {
        if !self.show_manager {
            return;
        }
        egui::SidePanel::right("curve_manager")
            .resizable(true)
            .default_width(245.0)
            .width_range(190.0..=360.0)
            .show(ctx, |ui| {
                ui.heading("Active curves");
                if self.active.is_empty() && self.active_targets.is_empty() {
                    ui.label("Select up to eight measurements.");
                    return;
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut removals = Vec::new();
                    for index in self
                        .active
                        .iter()
                        .chain(self.active_targets.iter())
                        .copied()
                        .collect::<Vec<_>>()
                    {
                        let group = self.groups[index].clone();
                        ui.horizontal(|ui| {
                            let color = self.curve_color(index);
                            let (swatch, _) = ui
                                .allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                            ui.painter().circle_filled(swatch.center(), 5.0, color);
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&group.name).color(color));
                                ui.small(&group.brand);
                            });
                            if !group.target && ui.small_button("Color").clicked() {
                                let offset = self
                                    .curve_color_offsets
                                    .entry(group_key(&group))
                                    .or_default();
                                *offset = offset.wrapping_add(1);
                            }
                        });
                        if group.variants.len() > 1 {
                            let mut selected_variant = self
                                .variant_choices
                                .get(&index)
                                .copied()
                                .unwrap_or(0)
                                .min(group.variants.len() - 1);
                            egui::ComboBox::from_id_salt(("variant", index))
                                .selected_text(&group.variants[selected_variant].name)
                                .show_ui(ui, |ui| {
                                    for (variant_index, variant) in
                                        group.variants.iter().enumerate()
                                    {
                                        ui.selectable_value(
                                            &mut selected_variant,
                                            variant_index,
                                            &variant.name,
                                        );
                                    }
                                });
                            if self.variant_choices.get(&index).copied().unwrap_or(0)
                                != selected_variant
                            {
                                self.variant_choices.insert(index, selected_variant);
                                self.dirty = true;
                            }
                        }
                        ui.horizontal(|ui| {
                            let mut baseline = self.baseline == Some(index);
                            if ui.checkbox(&mut baseline, "Baseline").changed() {
                                self.baseline = baseline.then_some(index);
                                self.dirty = true;
                            }
                            let mut pinned = self.pinned.contains(&index);
                            if ui.checkbox(&mut pinned, "Pin").changed() {
                                if pinned {
                                    self.pinned.insert(index);
                                } else {
                                    self.pinned.remove(&index);
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            if !group.target {
                                for (mode, label) in [
                                    (ChannelMode::Left, "L"),
                                    (ChannelMode::Right, "R"),
                                    (ChannelMode::Both, "Both"),
                                    (ChannelMode::Average, "Avg"),
                                ] {
                                    if ui
                                        .selectable_label(
                                            self.channel_modes
                                                .get(&index)
                                                .copied()
                                                .unwrap_or_default()
                                                == mode,
                                            label,
                                        )
                                        .clicked()
                                    {
                                        self.channel_modes.insert(index, mode);
                                        self.dirty = true;
                                    }
                                }
                                let variant_index = self
                                    .variant_choices
                                    .get(&index)
                                    .copied()
                                    .unwrap_or(0)
                                    .min(group.variants.len() - 1);
                                if group_has_imbalance(
                                    &self.curves,
                                    &group.variants[variant_index].curve_indices,
                                ) {
                                    ui.colored_label(Color32::YELLOW, "!");
                                }
                            }
                            let mut hidden = self.hidden.contains(&index);
                            if ui.checkbox(&mut hidden, "Hide").changed() {
                                if hidden {
                                    self.hidden.insert(index);
                                } else {
                                    self.hidden.remove(&index);
                                }
                                self.dirty = true;
                            }
                            let offset = self.offsets.entry(index).or_default();
                            if ui
                                .add(egui::DragValue::new(offset).speed(0.1).suffix(" dB"))
                                .changed()
                            {
                                self.dirty = true;
                            }
                            let remove = ui.add_enabled(
                                !self.pinned.contains(&index),
                                egui::Button::new("Remove"),
                            );
                            let remove = if self.pinned.contains(&index) {
                                remove.on_hover_text("Unpin this curve before removing it.")
                            } else {
                                remove
                            };
                            if remove.clicked() {
                                removals.push(index);
                            }
                        });
                        ui.separator();
                    }
                    for index in removals {
                        self.toggle_curve(index);
                    }
                });
            });
    }

    fn central_panel(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(error) = &self.processing_error {
                ui.colored_label(Color32::LIGHT_RED, error);
            }
            if self.display_curves.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.heading(if self.curves.is_empty() {
                        "No measurements found"
                    } else {
                        "Select measurements to compare"
                    });
                });
                return;
            }
            let plot_curves: Vec<PlotCurve<'_>> = self
                .display_curves
                .iter()
                .map(|curve| PlotCurve {
                    label: &curve.label,
                    label_key: &curve.label,
                    points: &curve.points,
                    color: self.curve_color(curve.group_index),
                    dashed: curve.target,
                })
                .collect();
            let interaction = GraphCanvas {
                zoom: self.zoom,
                smooth_level: self.smooth_level,
                show_labels: self.show_labels,
                inspect_mode: self.inspect_mode,
                label_positions: &self.label_positions,
                tutorial_band: self.tutorial_segment.map(|index| {
                    let segment = &TUTORIAL_SEGMENTS[index];
                    (segment.title, segment.range_hz)
                }),
                y_scale: self.y_scale,
            }
            .show(ui, &plot_curves);
            self.label_positions.extend(interaction.label_positions);
            if let Some(y_scale) = interaction.y_scale {
                self.y_scale = y_scale;
            }
        });
    }

    fn export_svg(&mut self) {
        let plot_curves: Vec<PlotCurve<'_>> = self
            .display_curves
            .iter()
            .map(|curve| PlotCurve {
                label: &curve.label,
                label_key: &curve.label,
                points: &curve.points,
                color: self.curve_color(curve.group_index),
                dashed: curve.target,
            })
            .collect();
        let svg = render_svg(
            &plot_curves,
            self.zoom,
            self.smooth_level,
            &self.label_positions,
            self.dark_mode,
            self.y_scale,
        );
        match save_svg(&svg) {
            Ok(path) => {
                self.svg_error = None;
                self.svg_message = Some(format!("SVG exported to {}", path.display()));
            }
            Err(error) => {
                self.svg_message = None;
                self.svg_error = Some(format!("SVG export failed: {error:#}"));
            }
        }
    }
}

impl eframe::App for CrinGraphApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        ctx.set_debug_on_hover(self.debug_hooks);
        if self.screenshot_pending {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            self.screenshot_pending = false;
        }
        if !self.loaded {
            if let Err(error) = self.load_measurements() {
                self.load_error = Some(format!("Unable to load measurements: {error:#}"));
                self.loaded = true;
            }
        }
        if self.dirty {
            self.rebuild_display_curves();
        }
        self.toolbar(ctx);
        self.share_window(ctx);
        self.tutorial_window(ctx);
        self.selector_panel(ctx);
        self.manager_panel(ctx);
        self.central_panel(ctx);
        if self.loaded && self.load_error.is_none() {
            self.persist_preferences_if_changed();
        }
        let screenshot = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = screenshot {
            match save_screenshot(&image) {
                Ok(path) => {
                    self.screenshot_error = None;
                    self.screenshot_message = Some(format!("PNG exported to {}", path.display()));
                }
                Err(error) => {
                    self.screenshot_message = None;
                    self.screenshot_error = Some(format!("PNG export failed: {error:#}"));
                }
            }
        }
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{} active · {} total · {} Hz–{} Hz",
                    self.display_curves.len(),
                    self.groups.len(),
                    self.zoom.x_range().0 as u32,
                    self.zoom.x_range().1 as u32
                ));
                if let Some(index) = self.baseline {
                    ui.label(format!("Baseline: {}", self.groups[index].name));
                }
                if self.processing_error.is_some()
                    || self.load_error.is_some()
                    || self.settings_error.is_some()
                    || self.screenshot_error.is_some()
                    || self.svg_error.is_some()
                {
                    ui.colored_label(Color32::LIGHT_RED, "Some issues need attention.");
                }
            });
            if let Some(error) = &self.settings_error {
                ui.colored_label(Color32::LIGHT_RED, error);
            }
            if let Some(error) = &self.screenshot_error {
                ui.colored_label(Color32::LIGHT_RED, error);
            }
            if let Some(message) = &self.screenshot_message {
                ui.label(message);
            }
            if let Some(error) = &self.svg_error {
                ui.colored_label(Color32::LIGHT_RED, error);
            }
            if let Some(message) = &self.svg_message {
                ui.label(message);
            }
        });
    }
}

fn matches_search(candidate: &str, query: &str) -> bool {
    let candidate = candidate.to_lowercase();
    let candidate_words = candidate.split_whitespace().collect::<Vec<_>>();
    query.to_lowercase().split_whitespace().all(|query_word| {
        candidate.contains(query_word)
            || candidate_words
                .iter()
                .any(|word| fuzzy_word_match(word, query_word))
    })
}

fn fuzzy_word_match(candidate: &str, query: &str) -> bool {
    let query_chars = query.chars().collect::<Vec<_>>();
    let candidate_chars = candidate.chars().collect::<Vec<_>>();
    if query_chars.len() < 3 {
        return false;
    }

    let threshold = (query_chars.len() / 4).clamp(1, 2);
    if candidate_chars.len().abs_diff(query_chars.len()) > threshold {
        return false;
    }

    let mut previous = (0..=candidate_chars.len()).collect::<Vec<_>>();
    let mut current = vec![0; candidate_chars.len() + 1];
    for (query_index, query_char) in query_chars.iter().enumerate() {
        current[0] = query_index + 1;
        for (candidate_index, candidate_char) in candidate_chars.iter().enumerate() {
            current[candidate_index + 1] = (previous[candidate_index + 1] + 1)
                .min(current[candidate_index] + 1)
                .min(previous[candidate_index] + usize::from(query_char != candidate_char));
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[candidate_chars.len()] <= threshold
}

fn build_groups(phone_books: &[PhoneBook], curves: &[CurveInfo]) -> Vec<MeasurementGroup> {
    let mut groups = Vec::new();
    let mut represented = BTreeSet::new();
    for book in phone_books {
        for brand in &book.brands {
            for model in &brand.models {
                let variants: Vec<MeasurementVariant> = model
                    .variants
                    .iter()
                    .filter_map(|variant| {
                        let curve_indices: Vec<usize> = curves
                            .iter()
                            .enumerate()
                            .filter(|(_, curve)| curve.model == variant.file_stem)
                            .map(|(index, _)| index)
                            .collect();
                        if curve_indices.is_empty() {
                            return None;
                        }
                        represented.extend(curve_indices.iter().copied());
                        Some(MeasurementVariant {
                            name: variant.name.clone(),
                            curve_indices,
                        })
                    })
                    .collect();
                if !variants.is_empty() {
                    groups.push(MeasurementGroup {
                        brand: brand.name.clone(),
                        brand_suffix: brand.suffix.clone(),
                        name: model.name.clone(),
                        target: false,
                        variants,
                    });
                }
            }
        }
    }

    let mut targets = BTreeMap::<(String, String), Vec<usize>>::new();
    let mut other = BTreeMap::<String, Vec<usize>>::new();
    for (index, curve) in curves.iter().enumerate() {
        if represented.contains(&index) {
            continue;
        }
        if let Some(name) = curve.model.strip_suffix(" Target") {
            targets
                .entry((target_category(name).to_owned(), name.to_owned()))
                .or_default()
                .push(index);
        } else {
            other.entry(curve.model.clone()).or_default().push(index);
        }
    }
    for ((category, name), curve_indices) in targets {
        groups.push(MeasurementGroup {
            brand: category,
            brand_suffix: None,
            name,
            variants: vec![MeasurementVariant {
                name: "Target".to_owned(),
                curve_indices,
            }],
            target: true,
        });
    }
    for (name, curve_indices) in other {
        groups.push(MeasurementGroup {
            brand: "Other measurements".to_owned(),
            brand_suffix: None,
            name: name.clone(),
            variants: vec![MeasurementVariant {
                name: "Default".to_owned(),
                curve_indices,
            }],
            target: false,
        });
    }
    groups.sort_by(|left, right| {
        left.brand
            .cmp(&right.brand)
            .then(left.name.cmp(&right.name))
    });
    groups
}

fn group_key(group: &MeasurementGroup) -> String {
    format!("{}\u{1f}{}", group.brand, group.name)
}

fn preferences_path() -> anyhow::Result<PathBuf> {
    let config_dir = dirs::config_dir().ok_or_else(|| {
        anyhow::anyhow!("the operating system did not provide a config directory")
    })?;
    Ok(config_dir.join("CrinGraph-native").join("preferences.json"))
}

fn read_preferences() -> anyhow::Result<Option<Preferences>> {
    read_preferences_from(&preferences_path()?)
}

fn read_preferences_from(path: &Path) -> anyhow::Result<Option<Preferences>> {
    match std::fs::read(path) {
        Ok(contents) => Ok(Some(serde_json::from_slice(&contents)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn write_preferences(preferences: &Preferences) -> anyhow::Result<()> {
    write_preferences_to(&preferences_path()?, preferences)
}

fn write_preferences_to(path: &Path, preferences: &Preferences) -> anyhow::Result<()> {
    let directory = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("preferences path has no parent directory"))?;
    std::fs::create_dir_all(directory)?;
    let temporary_path = path.with_extension("json.tmp");
    let serialized = serde_json::to_vec_pretty(preferences)?;
    let mut file = std::fs::File::create(&temporary_path)?;
    use std::io::Write;
    file.write_all(&serialized)?;
    file.sync_all()?;
    std::fs::rename(temporary_path, path)?;
    Ok(())
}

fn encode_share_uri(preferences: &Preferences) -> anyhow::Result<String> {
    let serialized = serde_json::to_vec(preferences)?;
    let mut uri = String::with_capacity(SHARE_URI_PREFIX.len() + serialized.len() * 2);
    uri.push_str(SHARE_URI_PREFIX);
    for byte in serialized {
        use std::fmt::Write;
        write!(uri, "{byte:02x}")?;
    }
    if uri.len() > MAX_SHARE_URI_LENGTH {
        anyhow::bail!("shared state exceeds the maximum URI length");
    }
    Ok(uri)
}

fn decode_share_uri(uri: &str) -> anyhow::Result<Preferences> {
    let payload = uri
        .strip_prefix(SHARE_URI_PREFIX)
        .ok_or_else(|| anyhow::anyhow!("unsupported share URI"))?;
    if payload.is_empty() || payload.len() % 2 != 0 || uri.len() > MAX_SHARE_URI_LENGTH {
        anyhow::bail!("share URI payload has an invalid length");
    }
    let bytes = payload
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char)
                .to_digit(16)
                .ok_or_else(|| anyhow::anyhow!("share URI contains a non-hex character"))?;
            let low = (pair[1] as char)
                .to_digit(16)
                .ok_or_else(|| anyhow::anyhow!("share URI contains a non-hex character"))?;
            Ok(((high << 4) | low) as u8)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn save_screenshot(image: &egui::ColorImage) -> anyhow::Result<PathBuf> {
    let directory = dirs::picture_dir().ok_or_else(|| {
        anyhow::anyhow!("the operating system did not provide a pictures directory")
    })?;
    std::fs::create_dir_all(&directory)?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let path = directory.join(format!("cringraph-{timestamp}.png"));
    save_screenshot_to(image, &path)?;
    Ok(path)
}

fn save_svg(svg: &str) -> anyhow::Result<PathBuf> {
    let directory = dirs::picture_dir().ok_or_else(|| {
        anyhow::anyhow!("the operating system did not provide a pictures directory")
    })?;
    std::fs::create_dir_all(&directory)?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let path = directory.join(format!("cringraph-{timestamp}.svg"));
    std::fs::write(&path, svg)?;
    Ok(path)
}

fn save_screenshot_to(image: &egui::ColorImage, path: &Path) -> anyhow::Result<()> {
    let directory = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("screenshot path has no parent directory"))?;
    std::fs::create_dir_all(directory)?;
    let width = u32::try_from(image.size[0])?;
    let height = u32::try_from(image.size[1])?;
    let pixels: Vec<u8> = image
        .pixels
        .iter()
        .flat_map(|color| color.to_array())
        .collect();
    image::save_buffer(path, &pixels, width, height, image::ColorType::Rgba8)?;
    Ok(())
}

fn apply_baseline(curves: &mut [DisplayCurve], baseline_group: usize) -> anyhow::Result<()> {
    let baseline_channels: Vec<&[FRPoint]> = curves
        .iter()
        .filter(|curve| curve.group_index == baseline_group)
        .map(|curve| curve.points.as_slice())
        .collect();
    if baseline_channels.is_empty() {
        anyhow::bail!("the selected baseline has no active curve");
    }
    let baseline = processing::average_curves(&baseline_channels)?;
    for curve in curves {
        for point in &mut curve.points {
            point.db -= interpolate_db(&baseline, point.hz)
                .ok_or_else(|| anyhow::anyhow!("unable to sample the baseline curve"))?;
        }
    }
    Ok(())
}

fn interpolate_db(points: &[FRPoint], hz: f32) -> Option<f32> {
    let first = points.first()?;
    let last = points.last()?;
    if hz <= first.hz {
        return Some(first.db);
    }
    if hz >= last.hz {
        return Some(last.db);
    }
    let upper = points.partition_point(|point| point.hz < hz);
    let left = points.get(upper.checked_sub(1)?)?;
    let right = points.get(upper)?;
    let fraction = (hz.ln() - left.hz.ln()) / (right.hz.ln() - left.hz.ln());
    Some(left.db + fraction * (right.db - left.db))
}

fn target_category(target: &str) -> &'static str {
    match target {
        "Diffuse Field" | "Etymotic" | "Free Field" | "Innerfidelity ID" | "IEF Neutral" => {
            "Neutral targets"
        }
        "Antdroid" | "Bad Guy" | "Banbeucmas" | "Crinacle" | "Precogvision" | "Super Review" => {
            "Reviewer targets"
        }
        "Harman" | "Rtings" | "Sonarworks" | "IEF \"Harman\"" => "Preference targets",
        _ => "Other targets",
    }
}

fn prepare_channels(
    curves: &[CurveInfo],
    indices: &[usize],
    mode: ChannelMode,
    smooth_level: usize,
) -> Result<Vec<(String, Vec<FRPoint>)>, processing::ProcessingError> {
    let mut left = Vec::new();
    let mut right = Vec::new();
    let mut average = Vec::new();
    let mut other = Vec::new();
    for &index in indices {
        let curve = &curves[index];
        let prepared = prepare_curve(&curve.points, smooth_level)?;
        let variant = curve.variant.as_str();
        if variant.starts_with('L') {
            left.push(prepared);
        } else if variant.starts_with('R') {
            right.push(prepared);
        } else if variant == "Avg" {
            average.push(prepared);
        } else {
            other.push(prepared);
        }
    }
    let combine =
        |channels: Vec<Vec<FRPoint>>| -> Result<Option<Vec<FRPoint>>, processing::ProcessingError> {
            if channels.is_empty() {
                Ok(None)
            } else if channels.len() == 1 {
                Ok(channels.into_iter().next())
            } else {
                let refs: Vec<&[FRPoint]> = channels.iter().map(Vec::as_slice).collect();
                processing::average_curves(&refs).map(Some)
            }
        };
    let mut left = combine(left)?;
    let mut right = combine(right)?;
    let mut average = combine(average)?;
    let mut other = combine(other)?;
    let selected = match mode {
        ChannelMode::Left => left
            .take()
            .map(|curve| vec![("L".to_owned(), curve)])
            .or_else(|| right.take().map(|curve| vec![("R".to_owned(), curve)]))
            .or_else(|| average.take().map(|curve| vec![("Avg".to_owned(), curve)]))
            .or_else(|| other.take().map(|curve| vec![(String::new(), curve)])),
        ChannelMode::Right => right
            .take()
            .map(|curve| vec![("R".to_owned(), curve)])
            .or_else(|| left.take().map(|curve| vec![("L".to_owned(), curve)]))
            .or_else(|| average.take().map(|curve| vec![("Avg".to_owned(), curve)]))
            .or_else(|| other.take().map(|curve| vec![(String::new(), curve)])),
        ChannelMode::Both => match (left.take(), right.take()) {
            (Some(left), Some(right)) => {
                Some(vec![("L".to_owned(), left), ("R".to_owned(), right)])
            }
            (Some(left), None) => Some(vec![("L".to_owned(), left)]),
            (None, Some(right)) => Some(vec![("R".to_owned(), right)]),
            (None, None) => average
                .take()
                .map(|curve| vec![("Avg".to_owned(), curve)])
                .or_else(|| other.take().map(|curve| vec![(String::new(), curve)])),
        },
        ChannelMode::Average => match (left.take(), right.take()) {
            (Some(left), Some(right)) => Some(vec![(
                "Avg".to_owned(),
                processing::average_curves(&[&left, &right])?,
            )]),
            (None, None) => average
                .take()
                .map(|curve| vec![("Avg".to_owned(), curve)])
                .or_else(|| other.take().map(|curve| vec![(String::new(), curve)])),
            (Some(left), None) => Some(vec![("L".to_owned(), left)]),
            (None, Some(right)) => Some(vec![("R".to_owned(), right)]),
        },
    };
    Ok(selected.unwrap_or_default())
}

fn group_has_imbalance(curves: &[CurveInfo], indices: &[usize]) -> bool {
    let left: Vec<usize> = indices
        .iter()
        .copied()
        .filter(|index| curves[*index].variant.starts_with('L'))
        .collect();
    let right: Vec<usize> = indices
        .iter()
        .copied()
        .filter(|index| curves[*index].variant.starts_with('R'))
        .collect();
    if left.is_empty() || right.is_empty() {
        return false;
    }
    let left_curves: Vec<&[FRPoint]> = left
        .iter()
        .map(|index| curves[*index].points.as_slice())
        .collect();
    let right_curves: Vec<&[FRPoint]> = right
        .iter()
        .map(|index| curves[*index].points.as_slice())
        .collect();
    let (Ok(left), Ok(right)) = (
        processing::average_curves(&left_curves),
        processing::average_curves(&right_curves),
    ) else {
        return false;
    };
    let (mut positive, mut negative) = (0.0_f32, 0.0_f32);
    for (a, b) in left.iter().zip(&right) {
        let difference = (a.db - b.db) / (50.0 * (1.0 + (a.hz / 10000.0).powi(6)).sqrt());
        positive = (positive + difference).max(0.0);
        negative = (negative - difference).max(0.0);
        if positive.max(negative) > 5.0 {
            return true;
        }
    }
    false
}

fn prepare_curve(
    curve: &[FRPoint],
    smooth_level: usize,
) -> Result<Vec<FRPoint>, processing::ProcessingError> {
    if smooth_level == 0 {
        return Ok(curve.to_vec());
    }
    let frequencies: Vec<f32> = curve.iter().map(|point| point.hz).collect();
    let params = SmoothingParams::new(&frequencies, smooth_level)?;
    processing::smooth(curve, &params)
}

#[cfg(test)]
mod tests {
    use super::{
        apply_baseline, build_groups, decode_share_uri, encode_share_uri, group_has_imbalance,
        interpolate_db, matches_search, prepare_channels, read_preferences_from,
        write_preferences_to, ChannelMode, CrinGraphApp, DisplayCurve, Preferences, SavedGroup,
        MAX_SHARE_URI_LENGTH, TUTORIAL_SEGMENTS,
    };
    use crigraph_native::data::{CurveInfo, FRPoint};
    use crigraph_native::graph::ZoomRegion;
    use std::path::Path;

    #[test]
    fn default_smoothing_matches_the_original_graph_setting() {
        assert_eq!(Preferences::default().smooth_level, 5);
        assert_eq!(CrinGraphApp::default().smooth_level, 5);
    }

    #[test]
    fn measurement_search_matches_tokens_and_common_typos() {
        assert!(matches_search("Crinacle Zero Red", "crinacle red"));
        assert!(matches_search("Crinacle Zero Red", "crinacal"));
        assert!(matches_search("Crinacle Zero Red", "zero reed"));
        assert!(!matches_search("Crinacle Zero Red", "zebra"));
        assert!(matches_search("Crinacle Zero Red", ""));
    }

    #[test]
    fn startup_loads_the_configured_measurement_and_targets() {
        let mut app = CrinGraphApp::default();
        app.load_measurements().unwrap();
        assert!(!app.curves.is_empty());
        assert!(app
            .active
            .iter()
            .any(|&index| app.groups[index].name == "HTH67"));
        assert!(app
            .groups
            .iter()
            .any(|group| group.name == "Harman" && group.target));
        app.rebuild_display_curves();
        assert!(app.display_curves.len() >= app.active.len());
        assert!(app.processing_error.is_none());
    }

    #[test]
    fn channel_modes_select_left_right_both_and_linear_average() {
        let curves = vec![
            test_curve("Model", "L", 50.0),
            test_curve("Model", "R", 54.0),
        ];
        let indices = [0, 1];
        let both = prepare_channels(&curves, &indices, ChannelMode::Both, 0).unwrap();
        assert_eq!(both.len(), 2);
        assert_eq!(both[0].0, "L");
        assert_eq!(both[1].0, "R");
        let left = prepare_channels(&curves, &indices, ChannelMode::Left, 0).unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].1[0].db, 50.0);
        let right_only = prepare_channels(&curves[1..], &[0], ChannelMode::Left, 0).unwrap();
        assert_eq!(right_only[0].0, "R");
        let average = prepare_channels(&curves, &indices, ChannelMode::Average, 0).unwrap();
        assert_eq!(average[0].0, "Avg");
        let expected =
            20.0 * ((10.0_f64.powf(50.0 / 20.0) + 10.0_f64.powf(54.0 / 20.0)) / 2.0).log10();
        assert!((average[0].1[0].db as f64 - expected).abs() < 0.001);
    }

    #[test]
    fn changing_visible_channels_keeps_the_normalization_reference() {
        let curves = vec![
            test_curve("Model", "L", 50.0),
            test_curve("Model", "R", 60.0),
        ];
        let group = super::MeasurementGroup {
            brand: "Brand".to_owned(),
            brand_suffix: None,
            name: "Model".to_owned(),
            variants: vec![super::MeasurementVariant {
                name: "Default".to_owned(),
                curve_indices: vec![0, 1],
            }],
            target: false,
        };
        let mut app = CrinGraphApp {
            curves,
            groups: vec![group],
            active: [0].into_iter().collect(),
            ..CrinGraphApp::default()
        };
        app.channel_modes.insert(0, ChannelMode::Both);
        app.rebuild_display_curves();
        let both_left = app.display_curves[0].points[0].db;
        app.channel_modes.insert(0, ChannelMode::Left);
        app.dirty = true;
        app.rebuild_display_curves();
        assert!((app.display_curves[0].points[0].db - both_left).abs() < 0.001);
    }

    #[test]
    fn target_groups_and_model_variants_follow_the_phone_book() {
        let book =
            crigraph_native::data::load_phone_book(Path::new("data/phone_book.json")).unwrap();
        let curves = vec![
            test_curve("HTH67", "L", 50.0),
            test_curve("HTH67", "R", 51.0),
            test_curve("Combo Variant 1", "L", 55.0),
            test_curve("Combo Variant 2", "L", 56.0),
            test_curve("Harman Target", "", 60.0),
        ];
        let groups = build_groups(&[book], &curves);
        let combo = groups
            .iter()
            .find(|group| group.name == "Variations")
            .unwrap();
        assert_eq!(combo.variants.len(), 2);
        assert!(groups
            .iter()
            .any(|group| group.name == "Harman" && group.target));
        assert!(groups.iter().any(|group| group.name == "HTH67"));
    }

    #[test]
    fn channel_imbalance_accumulates_frequency_weighted_difference() {
        let curves = vec![
            test_curve("Model", "L", 60.0),
            test_curve("Model", "R", 40.0),
        ];
        assert!(group_has_imbalance(&curves, &[0, 1]));
    }

    #[test]
    fn baseline_subtracts_the_reference_response_from_every_curve() {
        let mut curves = vec![
            DisplayCurve {
                group_index: 0,
                label: "Base".to_owned(),
                points: vec![
                    FRPoint {
                        hz: 100.0,
                        db: 60.0,
                    },
                    FRPoint {
                        hz: 1000.0,
                        db: 50.0,
                    },
                ],
                target: false,
            },
            DisplayCurve {
                group_index: 1,
                label: "Compared".to_owned(),
                points: vec![
                    FRPoint {
                        hz: 100.0,
                        db: 65.0,
                    },
                    FRPoint {
                        hz: 1000.0,
                        db: 45.0,
                    },
                ],
                target: false,
            },
        ];
        apply_baseline(&mut curves, 0).unwrap();
        assert_eq!(curves[0].points[0].db, 0.0);
        assert_eq!(curves[0].points[1].db, 0.0);
        assert_eq!(curves[1].points[0].db, 5.0);
        assert_eq!(curves[1].points[1].db, -5.0);
    }

    #[test]
    fn baseline_interpolation_uses_log_frequency_and_clamps_endpoints() {
        let points = [
            FRPoint {
                hz: 100.0,
                db: 10.0,
            },
            FRPoint {
                hz: 1000.0,
                db: 20.0,
            },
        ];
        assert_eq!(interpolate_db(&points, 10.0), Some(10.0));
        assert_eq!(interpolate_db(&points, 10000.0), Some(20.0));
        assert!((interpolate_db(&points, 316.22778).unwrap() - 15.0).abs() < 0.001);
    }

    #[test]
    fn pinned_measurements_stay_selected_until_unpinned() {
        let mut app = CrinGraphApp {
            groups: vec![super::MeasurementGroup {
                brand: "Brand".into(),
                brand_suffix: None,
                name: "Model".into(),
                variants: vec![super::MeasurementVariant {
                    name: "Default".into(),
                    curve_indices: Vec::new(),
                }],
                target: false,
            }],
            active: [0].into_iter().collect(),
            pinned: [0].into_iter().collect(),
            ..CrinGraphApp::default()
        };
        app.toggle_curve(0);
        assert!(app.active.contains(&0));
        app.pinned.remove(&0);
        app.toggle_curve(0);
        assert!(!app.active.contains(&0));
    }

    #[test]
    fn per_measurement_color_offsets_override_the_global_palette() {
        let group = super::MeasurementGroup {
            brand: "Brand".into(),
            brand_suffix: None,
            name: "Model".into(),
            variants: vec![super::MeasurementVariant {
                name: "Default".into(),
                curve_indices: Vec::new(),
            }],
            target: false,
        };
        let key = super::group_key(&group);
        let app = CrinGraphApp {
            groups: vec![group],
            color_offset: 2,
            curve_color_offsets: [(key, 3)].into_iter().collect(),
            ..CrinGraphApp::default()
        };
        assert_eq!(
            app.curve_color(0),
            crigraph_native::color::get_curve_color_with_offset(0, 5)
        );
    }

    #[test]
    fn targets_are_selected_separately_and_do_not_consume_measurement_slots() {
        let groups = (0..11)
            .map(|index| super::MeasurementGroup {
                brand: if index < 9 {
                    "Brand"
                } else {
                    "Neutral targets"
                }
                .into(),
                brand_suffix: None,
                name: format!("Model {index}"),
                variants: vec![super::MeasurementVariant {
                    name: "Default".into(),
                    curve_indices: Vec::new(),
                }],
                target: index >= 9,
            })
            .collect();
        let mut app = CrinGraphApp {
            groups,
            ..CrinGraphApp::default()
        };
        for index in 0..9 {
            app.toggle_curve(index);
        }
        app.toggle_curve(9);
        app.toggle_curve(10);
        assert_eq!(app.active.len(), 8);
        assert_eq!(app.active_targets, [9, 10].into_iter().collect());
        assert_eq!(app.preferences().active.len(), 10);
    }

    #[test]
    fn saved_target_selection_and_target_baseline_are_restored() {
        let groups = vec![
            super::MeasurementGroup {
                brand: "Brand".into(),
                brand_suffix: None,
                name: "Model".into(),
                variants: vec![super::MeasurementVariant {
                    name: "Default".into(),
                    curve_indices: Vec::new(),
                }],
                target: false,
            },
            super::MeasurementGroup {
                brand: "Neutral targets".into(),
                brand_suffix: None,
                name: "Diffuse Field".into(),
                variants: vec![super::MeasurementVariant {
                    name: "Target".into(),
                    curve_indices: Vec::new(),
                }],
                target: true,
            },
        ];
        let preferences = Preferences {
            active: vec![0, 1]
                .into_iter()
                .map(|index| SavedGroup {
                    key: super::group_key(&groups[index]),
                    variant: 0,
                    channel_mode: ChannelMode::Both,
                    hidden: false,
                    pinned: false,
                    offset: 0.0,
                })
                .collect(),
            baseline: Some(super::group_key(&groups[1])),
            color_offset: 4,
            ..Preferences::default()
        };
        let mut app = CrinGraphApp {
            groups,
            ..CrinGraphApp::default()
        };
        assert!(app.apply_preferences(preferences));
        assert_eq!(app.active, [0].into_iter().collect());
        assert_eq!(app.active_targets, [1].into_iter().collect());
        assert_eq!(app.baseline, Some(1));
        assert_eq!(app.color_offset, 4);
    }

    #[test]
    fn independently_selected_target_is_included_in_graph_data() {
        let mut app = CrinGraphApp {
            curves: vec![test_curve("Diffuse Field Target", "", 60.0)],
            groups: vec![super::MeasurementGroup {
                brand: "Neutral targets".into(),
                brand_suffix: None,
                name: "Diffuse Field".into(),
                variants: vec![super::MeasurementVariant {
                    name: "Target".into(),
                    curve_indices: vec![0],
                }],
                target: true,
            }],
            active_targets: [0].into_iter().collect(),
            ..CrinGraphApp::default()
        };
        app.rebuild_display_curves();
        assert_eq!(app.display_curves.len(), 1);
        assert!(app.display_curves[0].target);
    }

    #[test]
    fn preferences_round_trip_and_missing_or_malformed_files_are_handled() {
        let directory = std::env::temp_dir().join(format!(
            "crigraph-native-preferences-{}",
            std::process::id()
        ));
        let path = directory.join("preferences.json");
        let _ = std::fs::remove_dir_all(&directory);
        assert_eq!(read_preferences_from(&path).unwrap(), None);

        let preferences = Preferences {
            active: vec![SavedGroup {
                key: "Brand\u{1f}Model".into(),
                variant: 2,
                channel_mode: ChannelMode::Right,
                hidden: true,
                pinned: true,
                offset: -2.5,
            }],
            baseline: Some("Brand\u{1f}Model".into()),
            zoom: ZoomRegion::Mids,
            phon_normalization: false,
            target_phon: 60.0,
            target_hz: 750.0,
            smooth_level: 3,
            show_labels: true,
            inspect_mode: true,
            dark_mode: false,
            color_offset: 7,
            show_measurements: false,
            show_manager: false,
            label_positions: [("Brand · Model".to_owned(), [0.4, 0.6])]
                .into_iter()
                .collect(),
            tutorial_segment: Some(3),
            y_scale: 1.75,
            curve_color_offsets: [("Brand\u{1f}Model".into(), 2)].into_iter().collect(),
        };
        write_preferences_to(&path, &preferences).unwrap();
        assert_eq!(read_preferences_from(&path).unwrap(), Some(preferences));

        std::fs::write(&path, b"{not valid json").unwrap();
        assert!(read_preferences_from(&path).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn png_export_writes_the_captured_rgba_image() {
        let directory =
            std::env::temp_dir().join(format!("crigraph-native-export-{}", std::process::id()));
        let path = directory.join("capture.png");
        let _ = std::fs::remove_dir_all(&directory);
        let image = egui::ColorImage {
            size: [2, 1],
            pixels: vec![
                egui::Color32::from_rgb(255, 0, 0),
                egui::Color32::from_rgb(0, 255, 0),
            ],
        };
        super::save_screenshot_to(&image, &path).unwrap();
        let exported = image::open(&path).unwrap().to_rgba8();
        assert_eq!(exported.dimensions(), (2, 1));
        assert_eq!(exported.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(exported.get_pixel(1, 0).0, [0, 255, 0, 255]);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn share_uri_round_trips_preferences_and_rejects_invalid_payloads() {
        let preferences = Preferences {
            active: vec![SavedGroup {
                key: "Brand\u{1f}Model".into(),
                variant: 1,
                channel_mode: ChannelMode::Average,
                hidden: false,
                pinned: true,
                offset: 3.5,
            }],
            baseline: Some("Brand\u{1f}Model".into()),
            zoom: ZoomRegion::Treble,
            color_offset: 11,
            label_positions: [("Model · Average".to_owned(), [0.2, 0.8])]
                .into_iter()
                .collect(),
            tutorial_segment: Some(5),
            y_scale: 2.25,
            curve_color_offsets: [("Brand\u{1f}Model".into(), 3)].into_iter().collect(),
            ..Preferences::default()
        };
        let uri = encode_share_uri(&preferences).unwrap();
        assert!(uri.starts_with(super::SHARE_URI_PREFIX));
        assert_eq!(decode_share_uri(&uri).unwrap(), preferences);
        assert!(decode_share_uri("https://example.com/").is_err());
        assert!(decode_share_uri("cringraph://state/v1/0").is_err());
        assert!(decode_share_uri("cringraph://state/v1/zz").is_err());
        assert!(encode_share_uri(&Preferences {
            active: vec![SavedGroup {
                key: "x".repeat(MAX_SHARE_URI_LENGTH),
                variant: 0,
                channel_mode: ChannelMode::Both,
                hidden: false,
                pinned: false,
                offset: 0.0,
            }],
            ..Preferences::default()
        })
        .is_err());
    }

    #[test]
    fn importing_shared_state_replaces_current_selection_and_graph_settings() {
        let groups = vec![
            super::MeasurementGroup {
                brand: "Brand".into(),
                brand_suffix: None,
                name: "First".into(),
                variants: vec![super::MeasurementVariant {
                    name: "Default".into(),
                    curve_indices: Vec::new(),
                }],
                target: false,
            },
            super::MeasurementGroup {
                brand: "Brand".into(),
                brand_suffix: None,
                name: "Second".into(),
                variants: vec![super::MeasurementVariant {
                    name: "Default".into(),
                    curve_indices: Vec::new(),
                }],
                target: false,
            },
        ];
        let state = Preferences {
            active: vec![SavedGroup {
                key: super::group_key(&groups[1]),
                variant: 0,
                channel_mode: ChannelMode::Right,
                hidden: false,
                pinned: false,
                offset: 0.0,
            }],
            target_hz: 1000.0,
            show_measurements: false,
            show_manager: true,
            tutorial_segment: Some(2),
            y_scale: 1.5,
            label_positions: [("Model · Default · R".to_owned(), [0.25, 0.75])]
                .into_iter()
                .collect(),
            curve_color_offsets: [(super::group_key(&groups[1]), 6)].into_iter().collect(),
            ..Preferences::default()
        };
        let mut app = CrinGraphApp {
            groups,
            active: [0].into_iter().collect(),
            ..CrinGraphApp::default()
        };
        app.restore_shared_state(&encode_share_uri(&state).unwrap())
            .unwrap();
        assert_eq!(app.active, [1].into_iter().collect());
        assert_eq!(app.target_hz, 1000.0);
        assert!(!app.show_measurements);
        assert!(app.show_manager);
        assert_eq!(app.tutorial_segment, Some(2));
        assert_eq!(app.y_scale, 1.5);
        assert_eq!(
            app.curve_color_offsets[&super::group_key(&app.groups[1])],
            6
        );
        assert_eq!(app.label_positions["Model · Default · R"], [0.25, 0.75]);
    }

    #[test]
    fn frequency_guide_segments_cover_the_audible_range_without_gaps() {
        assert_eq!(TUTORIAL_SEGMENTS.first().unwrap().range_hz.0, 20.0);
        assert_eq!(TUTORIAL_SEGMENTS.last().unwrap().range_hz.1, 20000.0);
        for pair in TUTORIAL_SEGMENTS.windows(2) {
            assert_eq!(pair[0].range_hz.1, pair[1].range_hz.0);
        }
        assert!(TUTORIAL_SEGMENTS
            .iter()
            .all(|segment| segment.range_hz.0 < segment.range_hz.1
                && !segment.description.is_empty()));
    }

    fn test_curve(model: &str, variant: &str, db: f32) -> CurveInfo {
        CurveInfo {
            model: model.to_owned(),
            variant: variant.to_owned(),
            points: (0..20)
                .map(|index| FRPoint {
                    hz: 100.0 + index as f32 * 100.0,
                    db,
                })
                .collect(),
        }
    }
}
