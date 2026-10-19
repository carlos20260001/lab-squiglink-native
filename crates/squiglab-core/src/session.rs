//! The program's state: what is loaded, what is selected, and what has been
//! changed.
//!
//! These are the rules of a session rather than of a picture. The interface
//! reads them and asks for changes; it does not hold them.

use crate::data::{CurveInfo, FRPoint};
use crate::equalizer;
use crate::graph::{LabelLayout, ZoomRegion};
use crate::processing::{self, NormParams, SmoothingParams};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Where a displayed curve came from. The reference tool renders an
/// equalizer's result as a synthetic curve added on the same path as a
/// measurement, back-linked to the parent so normalization and baseline
/// propagate to both. This is that back-link: an EQ curve names its parent
/// group, which owns the normalization both share.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CurveSource {
    /// A curve loaded from the measurement database.
    Measurement,
    /// An equalizer applied to a measurement group's curve.
    Equalizer,
}
#[derive(Clone, Debug)]
pub struct DisplayCurve {
    /// The measurement group this curve belongs to: the group itself for a
    /// measurement, its parent for an EQ curve.
    pub group_index: usize,
    pub source: CurveSource,
    pub label: String,
    pub points: Vec<FRPoint>,
    pub target: bool,
}

/// One row of a curve's equalizer: a filter and whether the row is switched
/// on. The reference tool keeps a disabled row in place rather than deleting
/// it, so a filter can be toggled back on without re-entering it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EqRow {
    #[serde(flatten)]
    pub filter: equalizer::Filter,
    pub enabled: bool,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub enum ChannelMode {
    Left,
    Right,
    #[default]
    Both,
    Average,
}

impl ChannelMode {
    /// How the mode reads in a sentence and in a file name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Both => "Both",
            Self::Average => "Average",
        }
    }

    /// Whether a prepared channel belongs to this mode.
    ///
    /// `Both` and `Average` are the only modes whose prepared channel carries
    /// their own name; `Left` and `Right` match the raw channel of the same
    /// name, so a mode does not need a channel named after it to be exported.
    pub fn accepts(self, channel: &str) -> bool {
        match self {
            Self::Left => channel.starts_with('L'),
            Self::Right => channel.starts_with('R'),
            Self::Both => channel.is_empty() || channel.starts_with('L'),
            Self::Average => channel.starts_with('A') || channel == "avg",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Preferences {
    pub active: Vec<SavedGroup>,
    pub baseline: Option<String>,
    pub zoom: ZoomRegion,
    pub phon_normalization: bool,
    pub target_phon: f32,
    pub target_hz: f32,
    pub smooth_level: usize,
    pub show_labels: bool,
    pub inspect_mode: bool,
    pub dark_mode: bool,
    pub color_offset: usize,
    pub show_measurements: bool,
    /// Whether the deviation window is open. The deviation is a second reading
    /// of the same selection rather than a view of it, so it lives in its own
    /// window and is off by default.
    pub show_delta: bool,
    pub label_positions: BTreeMap<String, [f32; 2]>,
    pub label_layout: crate::graph::LabelLayout,
    pub tutorial_segment: Option<usize>,
    pub y_scale: f32,
    pub curve_color_offsets: BTreeMap<String, usize>,
    /// Every curve's equalizer, keyed by group key. An equalizer belongs to
    /// the curve, so it survives the curve being switched off and comes back
    /// with it, as `p.eq` does in the reference tool.
    pub eq_filters: BTreeMap<String, Vec<EqRow>>,
    /// Selected delta targets, by name. A delta target is a rig correction
    /// rather than a curve to compare, so it carries none of the per-curve
    /// state a `SavedGroup` does.
    pub active_deltas: Vec<String>,
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
            show_delta: false,
            label_positions: BTreeMap::new(),
            label_layout: crate::graph::LabelLayout::default(),
            tutorial_segment: None,
            y_scale: 1.0,
            curve_color_offsets: BTreeMap::new(),
            eq_filters: BTreeMap::new(),
            active_deltas: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SavedGroup {
    pub key: String,
    pub variant: usize,
    pub channel_mode: ChannelMode,
    pub hidden: bool,
    pub pinned: bool,
    pub offset: f32,
}
#[derive(Clone, Debug)]
pub struct MeasurementVariant {
    pub name: String,
    pub curve_indices: Vec<usize>,
}
#[derive(Clone, Debug)]
pub struct MeasurementGroup {
    pub brand: String,
    pub brand_suffix: Option<String>,
    pub name: String,
    pub variants: Vec<MeasurementVariant>,
    pub target: bool,
}

/// The state of a session: what is loaded, what is selected, and what has been
/// changed.
///
/// The interface reads this and asks for changes to it. It is a rule of the
/// program, not a picture of one, so it does not know what a widget is.
pub struct Session {
    /// Every measurement in the database.
    pub curves: Vec<CurveInfo>,
    /// The measurements, grouped as the library lists them.
    pub groups: Vec<MeasurementGroup>,
    /// Which measurements are on the graph.
    pub active: BTreeSet<usize>,
    /// Which target curves are on the graph.
    pub active_targets: BTreeSet<usize>,
    /// Which measurements are hidden from the graph.
    pub hidden: BTreeSet<usize>,
    /// Which measurements stay selected when the graph is emptied.
    pub pinned: BTreeSet<usize>,
    /// The measurement the other curves are read against.
    pub baseline: Option<usize>,
    /// A manual offset for each measurement, in decibels.
    pub offsets: BTreeMap<usize, f32>,
    /// Which channels of each measurement are shown.
    pub channel_modes: BTreeMap<usize, ChannelMode>,
    /// Which variant of each measurement is shown.
    pub variant_choices: BTreeMap<usize, usize>,
    /// The curves the graph draws, measurement and equalizer alike.
    pub display_curves: Vec<DisplayCurve>,
    /// Rig-compensation curves, loaded from the delta directory. Empty when
    /// there is none, which is the normal case: they are optional.
    pub delta_targets: Vec<crate::delta::DeltaTarget>,
    /// Which delta targets are correcting the readings. Kept separate from
    /// `active` because a delta target is not a curve on the graph.
    pub active_deltas: BTreeSet<usize>,
    /// Why the delta directory could not be read, if it could not. Optional
    /// data failing to load must not take the measurements down with it.
    pub delta_error: Option<String>,
    /// How much of the frequency axis the graph shows.
    pub zoom: ZoomRegion,
    /// Whether the curves are normalized to a loudness level or matched at a
    /// frequency.
    pub phon_normalization: bool,
    /// The loudness level the curves are normalized to.
    pub target_phon: f32,
    /// The frequency the curves are matched at.
    pub target_hz: f32,
    /// Fractional-octave smoothing, in octaves.
    pub smooth_level: usize,
    /// Whether the curves carry labels.
    pub show_labels: bool,
    /// Whether the inspector follows the pointer.
    pub inspect_mode: bool,
    /// Whether the interface is dark.
    pub dark_mode: bool,
    /// The rotation applied to the curve colours.
    pub color_offset: usize,
    /// Whether the library column is shown.
    pub show_measurements: bool,
    /// Whether the deviation window is open. The deviation is a second reading
    /// of the same selection rather than a view of it, so it lives in its own
    /// window and is off by default.
    pub show_delta: bool,
    /// Where the user has dragged each label to.
    pub label_positions: BTreeMap<String, [f32; 2]>,
    /// Where the labels are put when the user has not dragged them.
    pub label_layout: LabelLayout,
    /// The frequency band the tutorial is showing.
    pub tutorial_segment: Option<usize>,
    /// The vertical scale of the graph.
    pub y_scale: f32,
    /// The colour each curve is drawn in, when it differs from the palette's.
    pub curve_color_offsets: BTreeMap<String, usize>,
    /// Every curve's equalizer, keyed by group index. An equalizer belongs to
    /// the curve, so it survives the curve being switched off and comes back
    /// with it, as `p.eq` does in the reference tool.
    pub eq_filters: BTreeMap<usize, Vec<EqRow>>,
    /// The measurement whose equalizer the panel is editing.
    pub eq_phone: Option<usize>,
    /// The target AutoEQ fits against.
    pub eq_target: Option<usize>,
    /// How many filters AutoEQ may produce.
    pub eq_max_filters: usize,
    /// The frequency band AutoEQ fits within.
    pub eq_range: (f64, f64),
    /// What AutoEQ is allowed to fit: which filter types, how many, and over
    /// what frequency, gain and Q.
    pub eq_constraints: crate::constraints::Constraints,
    /// The grid an uploaded measurement is resampled onto.
    pub upload_grid: Vec<f32>,
    /// The names of the curves the user uploaded, so they are excluded from
    /// shared state no matter which brand they were filed under.
    pub uploaded_names: BTreeSet<String>,
    /// Which of the two sound settings the panel is editing.
    pub sound_slot: usize,
    /// The two settings Compare A | B switches between.
    pub sound_settings: [crate::audio::Settings; 2],
    /// The player, when the sound is playing.
    pub player: Option<crate::audio::Player>,
    /// The preferences as they were last written, so a change that writes the
    /// same thing again is not written again.
    pub last_saved_preferences: Option<Preferences>,
    /// Whether the curves the graph draws are out of date.
    pub dirty: bool,
    /// The rig the measurements were taken on, as the configuration names it.
    /// The graph labels every curve with it.
    pub rig_description: Option<String>,
}

impl Default for Session {
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
            delta_targets: Vec::new(),
            active_deltas: BTreeSet::new(),
            delta_error: None,
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
            show_delta: false,
            label_positions: BTreeMap::new(),
            label_layout: LabelLayout::default(),
            tutorial_segment: None,
            y_scale: 1.0,
            curve_color_offsets: BTreeMap::new(),
            eq_filters: BTreeMap::new(),
            eq_phone: None,
            eq_target: None,
            eq_max_filters: DEFAULT_EQ_MAX_FILTERS,
            eq_range: crate::equalizer::AUTO_EQ_RANGE,
            eq_constraints: crate::constraints::Constraints::none(),
            upload_grid: Vec::new(),
            uploaded_names: BTreeSet::new(),
            sound_slot: 0,
            sound_settings: [crate::audio::Settings::default(); 2],
            player: None,
            last_saved_preferences: None,
            dirty: false,
            rig_description: None,
        }
    }
}

/// How many equalizer rows a curve may carry.
pub const MAX_EQ_ROWS: usize = 20;

/// How many measurements may be compared at once.
pub const MAX_ACTIVE_CURVES: usize = 8;

/// How many filters AutoEQ may produce when nothing says otherwise.
pub const DEFAULT_EQ_MAX_FILTERS: usize = 10;

/// Where the data is when the configuration does not say.
///
/// A program run from a checkout finds the repository's own directories;
/// an installed one finds the paths its configuration names.
#[derive(Clone, Debug)]
pub struct DataPaths {
    pub measurements: PathBuf,
    pub headphones: PathBuf,
    pub corrections: PathBuf,
}

/// What a load restored, and what went wrong.
#[derive(Clone, Debug, Default)]
pub struct Loaded {
    /// Whether a view was restored, so the program does not select a
    /// default curve over one the user asked for.
    pub restored: bool,
    /// Why a view could not be restored, if it could not.
    pub problem: Option<String>,
}

impl Session {
    /// Load the measurements, the targets, the rig corrections and the view.
    ///
    /// `shared_state` is a link the user opened, which takes precedence over
    /// the preferences this installation saved.
    pub fn load(
        &mut self,
        paths: &DataPaths,
        shared_state: Option<&str>,
    ) -> Result<Loaded, String> {
        let config_path = crate::config::app_config_path().map_err(|error| format!("{error:#}"))?;
        let config = crate::config::initialize_app_config_from(&config_path)
            .map_err(|error| format!("{error:#}"))?;
        crate::config::validate_app_config(&config).map_err(|error| format!("{error:#}"))?;
        self.smooth_level = config.defaults.smoothing_octaves;
        self.rig_description = config.rig_description.clone();

        let database_dir = crate::config::resolve_measurement_dir(
            config.measurements.database_dir.as_deref(),
            &paths.measurements,
            &config_path,
        );
        let (mut curves, grid) =
            crate::data::load_data(&database_dir).map_err(|error| format!("{error:#}"))?;
        self.upload_grid = grid;
        self.uploaded_names.clear();
        let mut phone_books =
            vec![
                crate::data::load_phone_book(&database_dir.join("phone_book.json"))
                    .map_err(|error| format!("{error:#}"))?,
            ];

        let headphone_database_dir = crate::config::resolve_measurement_dir(
            config.measurements.headphone_database_dir.as_deref(),
            &paths.headphones,
            &config_path,
        );
        if config.measurements.headphone_database_dir.is_some() || headphone_database_dir.is_dir() {
            let (headphone_curves, _) = crate::data::load_data(&headphone_database_dir)
                .map_err(|error| format!("{error:#}"))?;
            curves.extend(headphone_curves);
            let phone_book_path = headphone_database_dir.join("phone_book.json");
            if phone_book_path.is_file() {
                phone_books.push(
                    crate::data::load_phone_book(&phone_book_path)
                        .map_err(|error| format!("{error:#}"))?,
                );
            }
        }
        curves.sort_by(|left, right| {
            left.model
                .cmp(&right.model)
                .then(left.variant.cmp(&right.variant))
        });
        self.groups = build_groups(&phone_books, &curves);

        // Delta targets are optional, so a failure to read them is reported in
        // their own section rather than failing the whole load: losing a rig
        // correction should not cost you the measurements as well.
        self.delta_error = None;
        let delta_dir = crate::config::resolve_measurement_dir(
            config.measurements.delta_dir.as_deref(),
            &paths.corrections,
            &config_path,
        );
        self.delta_targets = if delta_dir.is_dir() {
            match crate::delta::load_delta_targets(&delta_dir, &self.upload_grid) {
                Ok(targets) => targets,
                Err(error) => {
                    self.delta_error = Some(format!("{error}"));
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };
        // A delta target that was selected in an earlier session but is no
        // longer in the directory cannot be honoured, and must not leave a
        // dangling index behind for the rest of the session.
        self.active_deltas
            .retain(|index| *index < self.delta_targets.len());

        let mut loaded = Loaded::default();
        if let Some(shared_state) = shared_state {
            match crate::config::decode_share_uri(shared_state)
                .map_err(|error| format!("{error:#}"))
                .and_then(|preferences| self.apply_preferences(preferences))
            {
                Ok(_) => loaded.restored = true,
                Err(error) => {
                    loaded.problem = Some(format!("Unable to restore shared state: {error}"));
                }
            }
        } else {
            match crate::config::read_preferences() {
                Ok(Some(preferences)) => match self.apply_preferences(preferences) {
                    Ok(restored) => loaded.restored = restored,
                    Err(error) => loaded.problem = Some(error),
                },
                Ok(None) => {}
                Err(error) => {
                    loaded.problem = Some(format!("Unable to load preferences: {error:#}"));
                }
            }
        }
        if self.active.is_empty() && self.active_targets.is_empty() && !loaded.restored {
            // The reference opens on one measurement, so the graph is never
            // empty when there is something to compare.
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
        self.dirty = true;
        self.last_saved_preferences = Some(self.preferences());
        Ok(loaded)
    }

    pub fn rebuild_display_curves(&mut self) -> Option<String> {
        let mut output = Vec::with_capacity(self.active.len());
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
                    self.display_curves.clear();
                    self.dirty = false;

                    return Some(format!("{}: {error}", group.name));
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
                    self.display_curves.clear();
                    self.dirty = false;

                    return Some(format!("{}: {error}", group.name));
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
                    // An equalizer curves its parent's response, so the child is
                    // built from the parent's already-normalized points and
                    // inherits the parent's normalization by construction.
                    let eq_rows = self.eq_filters.get(&group_index).filter(|rows| {
                        !group.target && rows.iter().any(|row| row.enabled && !eq_row_is_inert(row))
                    });
                    for (channel_name, mut points) in channels {
                        for point in &mut points {
                            point.db += offset + manual_offset;
                        }
                        let parent_label =
                            curve_label(&group.name, &variant.name, channel_name.clone());
                        if let Some(rows) = eq_rows {
                            let enabled: Vec<equalizer::Filter> = rows
                                .iter()
                                .filter(|row| row.enabled)
                                .map(|row| row.filter)
                                .collect();
                            let response: Vec<equalizer::ResponsePoint> = points
                                .iter()
                                .map(|point| {
                                    equalizer::ResponsePoint::new(point.hz as f64, point.db as f64)
                                })
                                .collect();
                            let equalized = equalizer::apply(
                                &response,
                                &enabled,
                                equalizer::DEFAULT_SAMPLE_RATE,
                            );
                            // The child is pushed before its parent so it
                            // draws on top of the curve it shapes.
                            output.push(DisplayCurve {
                                group_index,
                                source: CurveSource::Equalizer,
                                label: eq_curve_label(&parent_label),
                                points: equalized
                                    .into_iter()
                                    .map(|point| FRPoint {
                                        hz: point.hz as f32,
                                        db: point.db as f32,
                                    })
                                    .collect(),
                                target: false,
                            });
                        }
                        output.push(DisplayCurve {
                            group_index,
                            source: CurveSource::Measurement,
                            label: parent_label,
                            points,
                            target: group.target,
                        });
                    }
                }
                Err(error) => {
                    self.display_curves.clear();
                    self.dirty = false;

                    return Some(format!("{}: {error}", group.name));
                }
            }
        }
        if let Some(baseline) = self.baseline {
            if let Err(error) = apply_baseline(&mut output, baseline) {
                self.display_curves.clear();
                self.dirty = false;
                return Some(format!("Baseline: {error}"));
            }
        }
        output.retain(|curve| !self.hidden.contains(&curve.group_index));
        self.display_curves = output;
        self.dirty = false;
        None
    }

    pub fn toggle_curve(&mut self, index: usize) {
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

    pub fn eq_phone(&self) -> Option<usize> {
        self.eq_phone
            .filter(|&index| self.active.contains(&index) && !self.groups[index].target)
            .or_else(|| {
                self.active
                    .iter()
                    .copied()
                    .find(|&index| !self.groups[index].target)
            })
    }

    pub fn eq_target(&self) -> Option<usize> {
        self.eq_target
            .filter(|&index| self.active_targets.contains(&index))
            .or_else(|| self.active_targets.iter().copied().next())
            .or_else(|| self.active.iter().copied().nth(1))
    }

    pub fn active_measurements(&self) -> Vec<usize> {
        self.active
            .iter()
            .copied()
            .filter(|&index| !self.groups[index].target)
            .collect()
    }

    pub fn enabled_filters(&self, index: usize) -> Vec<equalizer::Filter> {
        self.eq_filters
            .get(&index)
            .map(|rows| {
                rows.iter()
                    .filter(|row| row.enabled)
                    .map(|row| row.filter)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn normalized_average_points(
        &self,
        group_index: usize,
    ) -> Result<Vec<FRPoint>, processing::ProcessingError> {
        let group = &self.groups[group_index];
        let variant_index = self
            .variant_choices
            .get(&group_index)
            .copied()
            .unwrap_or(0)
            .min(group.variants.len().saturating_sub(1));
        let variant = &group.variants[variant_index];
        let channels = prepare_channels(
            &self.curves,
            &variant.curve_indices,
            ChannelMode::Both,
            self.smooth_level,
        )?;
        let channel_points: Vec<&[FRPoint]> = channels
            .iter()
            .map(|(_, points)| points.as_slice())
            .collect();
        let mut average = processing::average_curves(&channel_points)?;
        let offset = if self.phon_normalization {
            let params = processing::NormParams::for_curve(&average)?;
            processing::normalize(&average, self.target_phon, &params)?
        } else {
            processing::normalize_by_hz(&channel_points, self.target_hz)?
        };
        for point in &mut average {
            point.db += offset;
        }
        Ok(average)
    }

    pub fn response_points(points: &[FRPoint]) -> Vec<equalizer::ResponsePoint> {
        points
            .iter()
            .map(|point| equalizer::ResponsePoint::new(point.hz as f64, point.db as f64))
            .collect()
    }

    /// Fit filters to the selected measurement against the selected target.
    ///
    /// Returns the message to show, or the reason it could not be done. The
    /// filters are installed on the session either way.
    pub fn run_auto_eq(&mut self) -> Result<String, String> {
        let (Some(phone), Some(target)) = (self.eq_phone(), self.eq_target()) else {
            return Err(
                "Auto EQ needs a measurement to fit and a target to fit against".to_owned(),
            );
        };
        let source = Self::response_points(
            &self
                .normalized_average_points(phone)
                .map_err(|error| format!("Auto EQ: {error}"))?,
        );
        let target_curve = Self::response_points(
            &self
                .normalized_average_points(target)
                .map_err(|error| format!("Auto EQ target: {error}"))?,
        );
        let fitted = equalizer::autoeq(
            &source,
            &target_curve,
            self.eq_max_filters,
            self.eq_range,
            &self.eq_constraints,
        );
        let message = format!(
            "Auto EQ fitted {} filters against {}.",
            fitted.len(),
            self.groups[target].name
        );
        self.eq_filters.insert(
            phone,
            fitted
                .into_iter()
                .map(|filter| EqRow {
                    filter,
                    enabled: true,
                })
                .collect(),
        );
        self.dirty = true;
        Ok(message)
    }

    pub fn parametric_export(&self) -> Option<(String, String)> {
        let phone = self.eq_phone()?;
        let source = self
            .normalized_average_points(phone)
            .ok()
            .map(|points| Self::response_points(&points))?;
        let filters = self.enabled_filters(phone);
        let equalized = equalizer::apply(&source, &filters, equalizer::DEFAULT_SAMPLE_RATE);
        let preamp = equalizer::calc_preamp(&source, &equalized);
        Some((
            format!("{} Filters.txt", self.groups[phone].name),
            equalizer::export_parametric(&filters, preamp),
        ))
    }

    pub fn graphic_eq_export(&self) -> Option<(String, String)> {
        let phone = self.eq_phone()?;
        Some((
            format!("{} Wavelet.txt", self.groups[phone].name),
            equalizer::export_graphic_eq(
                &self.enabled_filters(phone),
                equalizer::DEFAULT_SAMPLE_RATE,
            ),
        ))
    }

    pub fn preferences(&self) -> Preferences {
        Preferences {
            active: self
                .active
                .iter()
                .chain(self.active_targets.iter())
                // An uploaded curve is excluded: a shared link is a claim about
                // the reference database, and a measurement from another rig is
                // not part of it, so a link that carried one would restore a
                // curve the recipient does not have and cannot check.
                .filter(|&&index| !self.is_uploaded(index))
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
            label_positions: self.label_positions.clone(),
            label_layout: self.label_layout,
            tutorial_segment: self.tutorial_segment,
            y_scale: self.y_scale,
            curve_color_offsets: self.curve_color_offsets.clone(),
            eq_filters: self
                .eq_filters
                .iter()
                .map(|(&index, rows)| (group_key(&self.groups[index]), rows.clone()))
                .collect(),
            active_deltas: self
                .active_deltas
                .iter()
                .map(|&index| self.delta_targets[index].name.clone())
                .collect(),
            show_delta: self.show_delta,
        }
    }

    pub fn apply_preferences(&mut self, preferences: Preferences) -> Result<bool, String> {
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
            || !preferences
                .eq_filters
                .values()
                .all(|rows| eq_rows_are_valid(rows))
        {
            return Err("Unable to load preferences: invalid settings values".into());
        }
        self.active.clear();
        self.active_targets.clear();
        self.active_deltas.clear();
        for name in &preferences.active_deltas {
            if let Some(index) = self
                .delta_targets
                .iter()
                .position(|target| &target.name == name)
            {
                self.active_deltas.insert(index);
            }
        }
        self.show_delta = preferences.show_delta;
        self.hidden.clear();
        self.pinned.clear();
        self.offsets.clear();
        self.channel_modes.clear();
        self.variant_choices.clear();
        self.baseline = None;
        self.label_positions = preferences.label_positions.clone();
        self.label_layout = preferences.label_layout;
        self.tutorial_segment = preferences.tutorial_segment;
        self.y_scale = preferences.y_scale;
        self.curve_color_offsets = preferences.curve_color_offsets.clone();
        self.eq_filters.clear();
        for (index, group) in self.groups.iter().enumerate() {
            if let Some(rows) = preferences.eq_filters.get(&group_key(group)) {
                if eq_rows_are_valid(rows) {
                    self.eq_filters.insert(index, rows.clone());
                }
            }
        }
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
        Ok(true)
    }

    pub fn toggle_delta(&mut self, index: usize) {
        if self.active_deltas.contains(&index) {
            self.active_deltas.remove(&index);
        } else {
            self.active_deltas.insert(index);
        }
        self.dirty = true;
    }

    pub fn is_uploaded(&self, index: usize) -> bool {
        let group = &self.groups[index];
        group.brand == UPLOADED_BRAND || self.uploaded_names.contains(&group.name)
    }

    pub fn matching_models(
        &self,
        _counts: &[(String, usize)],
        brand_filter: Option<&str>,
        search: &str,
    ) -> Vec<(usize, String)> {
        let mut models: Vec<(usize, String)> = Vec::new();
        for (index, group) in self.groups.iter().enumerate() {
            if group.target {
                continue; // Reference targets live in the target strip, not here.
            }
            let brand = brand_name(group);
            let narrowed = brand_filter.is_none_or(|chosen| chosen == brand);
            if narrowed && matches_search(&format!("{brand} {}", group.name), search) {
                models.push((index, group.name.clone()));
            }
        }
        models
    }

    pub fn export_curve_text(&mut self, group_index: usize) -> Result<String, String> {
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
        // A smoothing level of zero: no smoothing, nothing derived.
        let channels = match prepare_channels(&self.curves, &variant.curve_indices, channel_mode, 0)
        {
            Ok(channels) => channels,
            Err(error) => {
                return Err(format!("Unable to export {}: {error}", group.name));
            }
        };
        let Some((_, points)) = channels
            .into_iter()
            .find(|(name, _)| channel_mode.accepts(name.as_str()))
        else {
            return Err(format!(
                "{} has no {} data to export.",
                group.name,
                channel_mode.label()
            ));
        };
        let path = download_dir().join(curve_file_name(group, variant, channel_mode));
        let body = curve_text(&points);
        match std::fs::write(&path, body) {
            Ok(()) => Ok(format!("{} exported to {}", group.name, path.display())),
            Err(error) => Err(format!("Unable to export {path:?}: {error}")),
        }
    }
    /// Import a set of filters into a measurement's equalizer.
    ///
    /// Returns how many filters were imported, or why they could not be.
    pub fn import_parametric(&mut self, phone: usize, text: &str) -> Result<usize, String> {
        let imported = equalizer::import_parametric(text)
            .map_err(|error| format!("Unable to import the file: {error:#}"))?;
        let count = imported.filters.len();
        self.eq_filters.insert(
            phone,
            imported
                .filters
                .iter()
                .zip(&imported.enabled)
                .map(|(filter, &enabled)| EqRow {
                    filter: *filter,
                    enabled,
                })
                .collect(),
        );
        self.dirty = true;
        Ok(count)
    }

    /// Read an uploaded file into the database of curves, as a measurement or
    /// as a target.
    ///
    /// Returns the message to show, or why the file was refused.
    pub fn import_upload(&mut self, path: &Path, as_target: bool) -> Result<String, String> {
        let file_name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|name| !name.is_empty())
            .ok_or_else(|| "That path has no file name in it.".to_owned())?
            .to_owned();
        let contents = std::fs::read_to_string(path)
            .map_err(|error| format!("Unable to read the file: {error:#}"))?;
        let uploaded =
            crate::data::parse_upload(&file_name, &contents, &self.upload_grid, as_target)
                .map_err(|error| format!("{file_name} was not accepted: {error}"))?;
        if as_target {
            // A target the database already carries would draw twice, so the
            // upload is refused rather than shadowing it.
            if self
                .groups
                .iter()
                .any(|group| group.target && group.name == file_name)
            {
                return Err(format!("{file_name} is already a target in this database."));
            }
        }
        let curve = uploaded.curve;
        // A right-channel-only file is stored beside a null left channel so the
        // two line up on the same frequencies, exactly as a right-only
        // measurement from the database is.
        let model = curve.model.clone();
        let right_only = curve.variant == "R";
        let mut indices = Vec::new();
        self.curves.push(curve);
        indices.push(self.curves.len() - 1);
        if !as_target && right_only {
            self.curves.push(crate::data::CurveInfo {
                model,
                variant: "L".to_owned(),
                points: self
                    .upload_grid
                    .iter()
                    .map(|frequency| crate::data::FRPoint {
                        hz: *frequency,
                        // A null channel: every point at zero, which the
                        // channel picker skips in favour of the right one.
                        db: 0.0,
                    })
                    .collect(),
            });
            indices.push(self.curves.len() - 1);
        }
        self.uploaded_names.insert(file_name.clone());
        self.rebuild_uploaded_group(&file_name, as_target, &indices);
        self.dirty = true;
        Ok(if as_target {
            format!("Added {file_name} as a target.")
        } else {
            format!("Added {file_name} under {UPLOADED_BRAND}.")
        })
    }

    /// Rebuild the group an uploaded measurement lives in.
    fn rebuild_uploaded_group(&mut self, name: &str, target: bool, curve_indices: &[usize]) {
        let brand = if target {
            target_category(name).to_owned()
        } else {
            UPLOADED_BRAND.to_owned()
        };
        let existing = self
            .groups
            .iter()
            .position(|group| group.brand == brand && group.name == name);
        let index = match existing {
            // Re-uploading a name the user already uploaded replaces that
            // group's curves rather than piling a second copy of the same
            // model into it.
            Some(index) if self.is_uploaded(index) => {
                self.groups[index].variants[0].curve_indices.clear();
                index
            }
            Some(index) => index,
            None => {
                self.groups.push(MeasurementGroup {
                    brand,
                    brand_suffix: None,
                    name: name.to_owned(),
                    variants: vec![MeasurementVariant {
                        name: "Uploaded".to_owned(),
                        curve_indices: Vec::new(),
                    }],
                    target,
                });
                self.groups.len() - 1
            }
        };
        let variant = &mut self.groups[index].variants[0];
        variant.curve_indices.extend(curve_indices.iter().copied());
        variant.curve_indices.sort_unstable();
        variant.curve_indices.dedup();
        // Keep the group list in the same order the database loader uses, so
        // the library and the shared links do not reshuffle.
        self.groups.sort_by(|left, right| {
            left.brand
                .cmp(&right.brand)
                .then(left.name.cmp(&right.name))
        });
    }
}

/// The brand an uploaded measurement is filed under.
pub const UPLOADED_BRAND: &str = "Uploaded";

pub fn prepare_channels(
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
pub fn group_key(group: &MeasurementGroup) -> String {
    format!("{}\u{1f}{}", group.brand, group.name)
}

pub fn eq_row_is_inert(row: &EqRow) -> bool {
    let filter = row.filter;
    filter.freq == 0.0 || filter.q == 0.0 || filter.gain == 0.0
}
pub fn curve_label(model: &str, variant: &str, channel_name: String) -> String {
    let name = if variant.is_empty() || variant == model {
        model.to_owned()
    } else {
        variant.to_owned()
    };
    if channel_name.is_empty() {
        name
    } else {
        format!("{name} ({channel_name})")
    }
}
pub fn eq_curve_label(parent_label: &str) -> String {
    format!("{parent_label} EQ")
}
pub fn eq_rows_are_valid(rows: &[EqRow]) -> bool {
    !rows.is_empty()
        && rows.len() <= MAX_EQ_ROWS
        && rows.iter().all(|row| {
            let (filter, q) = (row.filter, row.filter.q);
            filter.freq.is_finite()
                && (1.0..=24_000.0).contains(&filter.freq)
                && q.is_finite()
                && q > 0.0
                && filter.gain.is_finite()
                && (-40.0..=40.0).contains(&filter.gain)
        })
}

pub fn apply_baseline(curves: &mut [DisplayCurve], baseline_group: usize) -> anyhow::Result<()> {
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
            point.db -= processing::interpolate_db(&baseline, point.hz)
                .ok_or_else(|| anyhow::anyhow!("unable to sample the baseline curve"))?;
        }
    }
    Ok(())
}

pub fn brand_name(group: &MeasurementGroup) -> String {
    group
        .brand_suffix
        .as_ref()
        .map(|suffix| format!("{} {suffix}", group.brand))
        .unwrap_or_else(|| group.brand.clone())
}
pub fn matches_search(candidate: &str, query: &str) -> bool {
    let candidate = candidate.to_lowercase();
    let candidate_words = candidate.split_whitespace().collect::<Vec<_>>();
    query.to_lowercase().split_whitespace().all(|query_word| {
        candidate.contains(query_word)
            || candidate_words
                .iter()
                .any(|word| fuzzy_word_match(word, query_word))
    })
}
pub fn fuzzy_word_match(candidate: &str, query: &str) -> bool {
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

pub fn download_dir() -> PathBuf {
    dirs::download_dir().unwrap_or_else(std::env::temp_dir)
}
pub fn curve_file_name(
    group: &MeasurementGroup,
    variant: &MeasurementVariant,
    mode: ChannelMode,
) -> String {
    let safe = |text: &str| {
        text.chars()
            .map(|c| match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => c,
                _ => '_',
            })
            .collect::<String>()
    };
    // A target is the reference the measurements are read against, so it says
    // so: an exported target sitting in a downloads folder with no marker is
    // easy to mistake for a measurement of something.
    let kind = if group.target {
        "target"
    } else {
        "measurement"
    };
    let mut name = format!(
        "{}-{}-{}-{}",
        safe(&group.name),
        kind,
        safe(&variant.name),
        mode.label()
    );
    // A name that is nothing but separators would collide with every other.
    if name.chars().all(|c| c == '-') {
        name = format!("curve-{kind}-{}", mode.label());
    }
    format!("{name}.txt")
}
pub fn curve_text(points: &[FRPoint]) -> String {
    points
        .iter()
        .map(|point| format!("{:.6},{:.6}\n", point.hz, point.db))
        .collect()
}

pub fn prepare_curve(
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

pub fn target_category(target: &str) -> &'static str {
    match target {
        "Diffuse Field" | "Etymotic" | "Free Field" | "Innerfidelity ID" | "IEF Neutral" => {
            "Neutral targets"
        }
        "Antdroid" | "Bad Guy" | "Banbeucmas" | "Crinacle" | "Precogvision" | "Super Review" => {
            "Reviewer targets"
        }
        "Harman" | "Rtings" | "Sonarworks" | "IEF \"Harman\"" | "IEF Harman" => {
            "Preference targets"
        }
        _ => "Other targets",
    }
}

pub fn build_groups(
    phone_books: &[crate::data::PhoneBook],
    curves: &[CurveInfo],
) -> Vec<MeasurementGroup> {
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
