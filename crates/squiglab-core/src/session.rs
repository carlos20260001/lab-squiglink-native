//! The program's state: what is loaded, what is selected, and what has been
//! changed.
//!
//! These are the rules of a session rather than of a picture. The interface
//! reads them and asks for changes; it does not hold them.

use crate::data::{CurveInfo, FRPoint};
use crate::equalizer;
use crate::graph::{LabelLayout, ZoomRegion};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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
            upload_grid: Vec::new(),
            uploaded_names: BTreeSet::new(),
            sound_slot: 0,
            sound_settings: [crate::audio::Settings::default(); 2],
            player: None,
            last_saved_preferences: None,
        }
    }
}

/// How many filters AutoEQ may produce when nothing says otherwise.
pub const DEFAULT_EQ_MAX_FILTERS: usize = 10;
