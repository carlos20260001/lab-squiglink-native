use egui::{Color32, Context, RichText, Ui};
use serde::{Deserialize, Serialize};
use squiglab::audio::Player;
use squiglab::color::target_color;
use squiglab::data::{self, CurveInfo, FRPoint, PhoneBook};
use squiglab::delta::{self, DeltaTarget};
use squiglab::equalizer;
use squiglab::graph::{export_svg as render_svg, DeltaBar, GraphCanvas, PlotCurve, ZoomRegion};
use squiglab::processing::{self, NormParams, SmoothingParams};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const MAX_ACTIVE_CURVES: usize = 8;
const MAX_SMOOTH_LEVEL: usize = 12;
/// Most filters one curve's equalizer can hold. The reference panel shows five
/// rows and AutoEQ produces at most a handful more; the bound keeps a corrupt
/// preferences file from growing without limit.
const MAX_EQ_ROWS: usize = 20;
/// Palette slots between a curve and its EQ curve, so the two lines stay
/// distinguishable without introducing a hue the graph palette does not have.
const EQ_CURVE_COLOR_SPREAD: usize = 4;
/// How many filters Auto EQ fits unless the panel's settings say otherwise.
/// The reference tool's panel shows five bands; the budget is ours, since the
/// live page does not expose one.
const DEFAULT_EQ_MAX_FILTERS: usize = 10;
/// The five bands a curve starts with, a Harman-shaped starting point. The
/// reference shows five default bands; the values on the live page could not
/// be read, so these are this tool's own defaults rather than a port.
fn default_eq_filters() -> Vec<EqRow> {
    [
        (equalizer::FilterType::LowShelf, 105.0, 0.7, 6.0),
        (equalizer::FilterType::Peaking, 280.0, 1.0, -4.0),
        (equalizer::FilterType::Peaking, 1000.0, 1.0, -3.0),
        (equalizer::FilterType::Peaking, 3500.0, 1.0, 3.0),
        (equalizer::FilterType::HighShelf, 9000.0, 0.7, -4.0),
    ]
    .map(|(filter_type, freq, q, gain)| EqRow {
        filter: equalizer::Filter::new(filter_type, freq, q, gain),
        enabled: true,
    })
    .into()
}
/// Height of the manager panel's header and padding before any curve rows.
const MANAGER_HEIGHT_BASE: f32 = 46.0;
/// Vertical room one curve row occupies when identity and controls share a line.
const MANAGER_ROW_HEIGHT: f32 = 32.0;
/// Vertical room one row occupies when it stacks: a second line of controls plus
/// the gap between them. Sizing the panel with the single-line figure here is
/// what made stacked rows overflow and scroll out of sight.
const MANAGER_STACKED_ROW_HEIGHT: f32 = 48.0;
const _: () = assert!(MANAGER_STACKED_ROW_HEIGHT > MANAGER_ROW_HEIGHT);
/// Row width below which the manager stacks a curve's controls under its
/// identity instead of pressing everything into one line. The identity and
/// control clusters measure ~800px together at the app's type scale, and a
/// single line narrower than that clips the right-hand controls off the edge.
const MANAGER_ROW_STACK_WIDTH: f32 = 800.0;
/// The most the manager will ever take from the graph.
/// How tall the target strip is, and the only height it can be: it is a single
/// row of chips, and letting it grow would take the room from the manager and
/// the graph rather than from anything a user chose to give it.
const TARGET_STRIP_HEIGHT: f32 = 44.0;

const MANAGER_MAX_HEIGHT: f32 = 460.0;

/// How wide a curve's preview is in its row.
const PREVIEW_WIDTH: f32 = 78.0;
/// How tall a curve's preview is in its row.
const PREVIEW_HEIGHT: f32 = 18.0;

/// One of the marks that runs down a manager row.
///
/// Split out from the row so the order the reference tool draws them in lives
/// in one place, and so adding a mark is a case here rather than an argument
/// threaded through the drawing code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RowControl {
    /// The offset box, in dB.
    Offset,
    /// Smoothing, applied to every curve.
    Smoothing,
    Hide,
    Pin,
    /// Subtract this curve from the others.
    Baseline,
    /// Write this curve's raw measurement out as a file.
    Download,
    Remove,
}

impl RowControl {
    /// The marks a row carries, in the order the row lays them out.
    const ORDER: [Self; 7] = [
        Self::Offset,
        Self::Smoothing,
        Self::Hide,
        Self::Pin,
        Self::Baseline,
        Self::Download,
        Self::Remove,
    ];

    /// What clicking it does.
    fn tooltip(self) -> &'static str {
        match self {
            Self::Offset => "Offset this curve by however many dB.",
            Self::Smoothing => "Cycle the fractional-octave smoothing.",
            Self::Hide => "Show or hide this curve in the graph.",
            Self::Pin => "Keep this curve in the graph.",
            Self::Baseline => "Subtract this curve from every other curve.",
            Self::Download => "Save this curve's raw measurement as a .txt file.",
            Self::Remove => "Remove this curve from the graph.",
        }
    }
}

/// The next smoothing level in the cycle.
///
/// A cycle rather than a slider, because the reference tool keeps a smoothing
/// control in every row and there is no room for a slider there. The levels are
/// the ones worth stopping at: 0 is the measurement as it was taken, and the
/// rest are the round values people actually read at.
fn next_smoothing_level(current: usize) -> usize {
    match current {
        0 => 1,
        1 => 2,
        2 => 5,
        5 => 10,
        10 => MAX_SMOOTH_LEVEL,
        _ => 0,
    }
}

const _: () = assert!(MAX_SMOOTH_LEVEL == 12);

/// How tall the manager wants to be for a selection, and the smallest it is
/// allowed to be, at a given viewport width and delta-bar height.
///
/// The minimum is the load-bearing half. A resizable egui panel keeps whatever
/// height it was last given: `default_size` only seeds it, once, on the frame
/// the panel id is first seen, and every frame after that reads the stored
/// height instead. A height stored before the delta bar existed is therefore
/// still in force after the bar lands, and against a fixed floor it is smaller
/// than the bar plus one row, which leaves the rows nothing to draw into.
/// Sizing the minimum from the content is what makes egui lift that stale
/// height back up on the very next frame.
///
/// Past `MANAGER_MAX_HEIGHT` the content stops fitting and the minimum falls
/// back to the bar and a single row, which is what leaves the manager
/// shrinkable so the overflow can be scrolled to.
fn manager_heights(width: f32, curve_count: usize) -> (f32, f32) {
    let row_height = if width < MANAGER_ROW_STACK_WIDTH {
        MANAGER_STACKED_ROW_HEIGHT
    } else {
        MANAGER_ROW_HEIGHT
    };
    let content = MANAGER_HEIGHT_BASE + curve_count as f32 * row_height;
    let min_height = if content <= MANAGER_MAX_HEIGHT {
        content
    } else {
        MANAGER_HEIGHT_BASE + row_height
    };
    (content.clamp(min_height, MANAGER_MAX_HEIGHT), min_height)
}
/// Content width below which the layout stacks instead of running the tab column
/// beside the graph. Matches the reference tool's transition, which sits between
/// 900 and 1100px.
const COMPACT_LAYOUT_WIDTH: f32 = 1024.0;
/// The brand uploaded measurements are listed under. A synthetic brand keeps a
/// user's own measurements out of the reference database's brands, and marks
/// them so they are left out of shared links.
const UPLOADED_BRAND: &str = "Uploaded";
const CONFIG_DIRECTORY_NAME: &str = "squiglab";
const SHARE_URI_PREFIX: &str = "squiglab://state/v1/";
/// Share links issued before the rename. Still decoded so links people already
/// shared keep working.
const LEGACY_SHARE_URI_PREFIX: &str = "cringraph://state/v1/";
const MAX_SHARE_URI_LENGTH: usize = 65_536;

#[derive(Clone, Debug, Deserialize, Default, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
struct AppConfig {
    measurements: MeasurementConfig,
    defaults: DefaultConfig,
    /// Optional provenance line drawn in the corner of the graph, e.g. the
    /// rig the measurements were taken on. The web tool labels every graph this
    /// way; here it is opt-in because the native app has no rig metadata of its
    /// own to fall back on.
    rig_description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Default, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
struct MeasurementConfig {
    database_dir: Option<PathBuf>,
    headphone_database_dir: Option<PathBuf>,
    /// Where rig-compensation delta targets live. Kept out of `database_dir`
    /// on purpose: the calibration curves are not headphones, and the database
    /// loader would list them as phantom measurements.
    delta_dir: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
struct DefaultConfig {
    smoothing_octaves: usize,
}

impl Default for DefaultConfig {
    fn default() -> Self {
        Self {
            smoothing_octaves: 5,
        }
    }
}

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

/// The frequency regions drawn beneath the axis, derived from the tutorial so
/// the two can never disagree about where a band starts or ends.
const FREQUENCY_BANDS: [squiglab::graph::FrequencyBand; 7] = {
    use squiglab::graph::FrequencyBand;
    [
        FrequencyBand {
            title: TUTORIAL_SEGMENTS[0].title,
            range_hz: TUTORIAL_SEGMENTS[0].range_hz,
        },
        FrequencyBand {
            title: TUTORIAL_SEGMENTS[1].title,
            range_hz: TUTORIAL_SEGMENTS[1].range_hz,
        },
        FrequencyBand {
            title: TUTORIAL_SEGMENTS[2].title,
            range_hz: TUTORIAL_SEGMENTS[2].range_hz,
        },
        FrequencyBand {
            title: TUTORIAL_SEGMENTS[3].title,
            range_hz: TUTORIAL_SEGMENTS[3].range_hz,
        },
        FrequencyBand {
            title: TUTORIAL_SEGMENTS[4].title,
            range_hz: TUTORIAL_SEGMENTS[4].range_hz,
        },
        FrequencyBand {
            title: TUTORIAL_SEGMENTS[5].title,
            range_hz: TUTORIAL_SEGMENTS[5].range_hz,
        },
        FrequencyBand {
            title: TUTORIAL_SEGMENTS[6].title,
            range_hz: TUTORIAL_SEGMENTS[6].range_hz,
        },
    ]
};

/// Where a displayed curve came from. The reference tool renders an
/// equalizer's result as a synthetic curve added on the same path as a
/// measurement, back-linked to the parent so normalization and baseline
/// propagate to both. This is that back-link: an EQ curve names its parent
/// group, which owns the normalization both share.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CurveSource {
    /// A curve loaded from the measurement database.
    Measurement,
    /// An equalizer applied to a measurement group's curve.
    Equalizer,
}

/// One row of the delta bar, before it becomes a plot curve.
/// One chip in the target strip: which curve it stands for, what it is called,
/// whether it is in play, and what to say when it is hovered.
///
/// Built out of state rather than read from it while drawing, because toggling
/// a chip needs the borrow the drawing loop is holding.
struct TargetChip {
    index: usize,
    label: String,
    selected: bool,
    tooltip: String,
}

struct DeviationCurve {
    label: String,
    points: Vec<FRPoint>,
    color: Color32,
    dashed: bool,
}

struct DisplayCurve {
    /// The measurement group this curve belongs to: the group itself for a
    /// measurement, its parent for an EQ curve.
    group_index: usize,
    source: CurveSource,
    label: String,
    points: Vec<FRPoint>,
    target: bool,
}

/// The panels sharing the library column. The reference tool's left column is
/// a tab column rather than a permanent sidebar, so the measurement list and
/// the equalizer take one slot between them instead of the equalizer needing
/// a window of its own.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum LibraryTab {
    /// The brand list, which narrows the model list to one brand.
    Brands,
    /// The measurement and target checkboxes.
    #[default]
    Models,
    /// The parametric equalizer for the selected curve.
    Equalizer,
}

/// One row of a curve's equalizer: a filter and whether the row is switched
/// on. The reference tool keeps a disabled row in place rather than deleting
/// it, so a filter can be toggled back on without re-entering it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct EqRow {
    #[serde(flatten)]
    filter: equalizer::Filter,
    enabled: bool,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
enum ChannelMode {
    Left,
    Right,
    #[default]
    Both,
    Average,
}

impl ChannelMode {
    /// How the mode reads in a sentence and in a file name.
    fn label(self) -> &'static str {
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
    fn accepts(self, channel: &str) -> bool {
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
    /// Whether the deviation window is open. The deviation is a second reading
    /// of the same selection rather than a view of it, so it lives in its own
    /// window and is off by default.
    show_delta: bool,
    label_positions: BTreeMap<String, [f32; 2]>,
    label_layout: squiglab::graph::LabelLayout,
    tutorial_segment: Option<usize>,
    y_scale: f32,
    curve_color_offsets: BTreeMap<String, usize>,
    /// Every curve's equalizer, keyed by group key. An equalizer belongs to
    /// the curve, so it survives the curve being switched off and comes back
    /// with it, as `p.eq` does in the reference tool.
    eq_filters: BTreeMap<String, Vec<EqRow>>,
    /// Selected delta targets, by name. A delta target is a rig correction
    /// rather than a curve to compare, so it carries none of the per-curve
    /// state a `SavedGroup` does.
    active_deltas: Vec<String>,
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
            label_layout: squiglab::graph::LabelLayout::default(),
            tutorial_segment: None,
            y_scale: 1.0,
            curve_color_offsets: BTreeMap::new(),
            eq_filters: BTreeMap::new(),
            active_deltas: Vec::new(),
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

/// Progressive fallback for the toolbar as the viewport narrows.
///
/// The reference tool keeps its toolbar on one row and drops controls as width
/// falls rather than wrapping them onto extra rows. These tiers do the same:
/// each step moves the next cluster into the `⋯` overflow menu instead of
/// hiding it, so every control stays reachable at every width.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ToolbarDensity {
    /// Every control sits inline in the row.
    Full,
    /// The wordmark drops and the action cluster becomes the overflow menu.
    Compact,
    /// Processing controls, view toggles, and the theme join the menu as well.
    Narrow,
}

impl ToolbarDensity {
    /// Width the full row needs, estimated from the inline width of every
    /// cluster at the app's own type scale: the row measures ~1350px with the
    /// wordmark and all five action buttons inline, which is why the default
    /// 1440px window is the only width that shows them all.
    const FULL_WIDTH: f32 = 1340.0;
    /// Width that fits once the wordmark and the action cluster drop out: the
    /// remaining processing controls, view toggles, theme, and panel toggles
    /// measure ~1030px together.
    const COMPACT_WIDTH: f32 = 1000.0;

    fn for_width(width: f32) -> Self {
        if width >= Self::FULL_WIDTH {
            Self::Full
        } else if width >= Self::COMPACT_WIDTH {
            Self::Compact
        } else {
            Self::Narrow
        }
    }

    /// Whether the wordmark is drawn beside the controls. The window title
    /// already names the application, so it is the first thing to go.
    fn shows_wordmark(self) -> bool {
        matches!(self, Self::Full)
    }

    /// Whether the processing and view-toggle controls stay inline instead of
    /// moving into the overflow menu.
    fn shows_processing(self) -> bool {
        !matches!(self, Self::Narrow)
    }

    /// Whether the theme toggle stays inline.
    fn shows_theme(self) -> bool {
        !matches!(self, Self::Narrow)
    }
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
    /// Rig-compensation curves, loaded from the delta directory. Empty when
    /// there is none, which is the normal case: they are optional.
    delta_targets: Vec<DeltaTarget>,
    /// Which delta targets are correcting the readings. Kept separate from
    /// `active` because a delta target is not a curve on the graph.
    active_deltas: BTreeSet<usize>,
    /// Why the delta directory could not be read, if it could not. Optional
    /// data failing to load must not take the measurements down with it.
    delta_error: Option<String>,
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
    /// Whether the deviation window is open. The deviation is a second reading
    /// of the same selection rather than a view of it, so it lives in its own
    /// window and is off by default.
    show_delta: bool,
    label_positions: BTreeMap<String, [f32; 2]>,
    label_layout: squiglab::graph::LabelLayout,
    tutorial_segment: Option<usize>,
    y_scale: f32,
    curve_color_offsets: BTreeMap<String, usize>,
    /// Per-curve equalizer filter rows, keyed by measurement group index.
    eq_filters: BTreeMap<usize, Vec<EqRow>>,
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
    /// Where the last curve export went, or why it did not.
    curve_export_message: Option<String>,
    curve_export_error: Option<String>,
    shared_state: Option<String>,
    share_window_open: bool,
    share_input: String,
    share_message: Option<String>,
    debug_hooks: bool,
    rig_description: Option<String>,
    /// Which panel the library column is showing.
    library_tab: LibraryTab,
    /// The brand the model list is narrowed to, chosen in the brands tab.
    brand_filter: Option<String>,
    /// The measurement whose equalizer the panel is editing. Session state:
    /// the panel re-derives a sensible curve rather than restoring a choice.
    eq_phone: Option<usize>,
    /// The target AutoEQ fits against.
    eq_target: Option<usize>,
    /// How many filters Auto EQ may produce.
    eq_max_filters: usize,
    eq_import_open: bool,
    eq_import_path: String,
    eq_import_error: Option<String>,
    eq_export_message: Option<String>,
    eq_export_error: Option<String>,
    /// The playing sound, if it is playing. Stopped and dropped by
    /// [`stop_sound`].
    player: Option<squiglab::audio::Player>,
    /// Which of the Compare A | B settings the controls are editing.
    sound_slot: usize,
    /// The two settings Compare A | B switches between. They live with the
    /// session rather than in preferences: a sound setting is something the
    /// user is auditioning now, not a view to restore.
    sound_settings: [squiglab::audio::Settings; 2],
    /// A message from the sound player, shown in the panel.
    sound_message: Option<String>,
    /// The measurement grid uploads are resampled onto, captured at load so an
    /// upload lands on the same frequencies as the database.
    upload_grid: Vec<f32>,
    /// The names of the curves the user uploaded, so they are excluded from
    /// shared state no matter which brand they were filed under.
    uploaded_names: BTreeSet<String>,
    /// Whether the equalizer's "How to use" window is open. Session state: the
    /// reference shows it to a first-time visitor, and this offers it again
    /// from the panel's help control.
    onboarding_open: bool,
    /// Which of the four paths is open in that window.
    onboarding_section: usize,
    upload_open: bool,
    upload_path: String,
    upload_as_target: bool,
    upload_error: Option<String>,
    upload_message: Option<String>,
    /// Set while the selected curve still needs scrolling into view in the
    /// library. Session-only: it is an action to take once, not a preference.
    library_reveal_pending: bool,
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
            delta_targets: Vec::new(),
            active_deltas: BTreeSet::new(),
            delta_error: None,
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
            show_delta: false,
            label_positions: BTreeMap::new(),
            label_layout: squiglab::graph::LabelLayout::default(),
            tutorial_segment: None,
            y_scale: 1.0,
            curve_color_offsets: BTreeMap::new(),
            eq_filters: BTreeMap::new(),
            rig_description: None,
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
            curve_export_message: None,
            curve_export_error: None,
            shared_state: None,
            share_window_open: false,
            share_input: String::new(),
            share_message: None,
            debug_hooks: false,
            library_tab: LibraryTab::Models,
            brand_filter: None,
            eq_phone: None,
            eq_target: None,
            eq_max_filters: DEFAULT_EQ_MAX_FILTERS,
            eq_import_open: false,
            eq_import_path: String::new(),
            eq_import_error: None,
            eq_export_message: None,
            eq_export_error: None,
            player: None,
            sound_slot: 0,
            sound_settings: [squiglab::audio::Settings::default(); 2],
            sound_message: None,
            upload_grid: Vec::new(),
            uploaded_names: BTreeSet::new(),
            onboarding_open: false,
            onboarding_section: 0,
            upload_open: false,
            upload_path: String::new(),
            upload_as_target: false,
            upload_error: None,
            upload_message: None,
            library_reveal_pending: false,
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
        let config_path = app_config_path()?;
        let config = initialize_app_config_from(&config_path)?;
        validate_app_config(&config)?;
        self.smooth_level = config.defaults.smoothing_octaves;
        self.rig_description = config.rig_description.clone();

        let database_dir = resolve_measurement_dir(
            config.measurements.database_dir.as_deref(),
            Path::new("data"),
            &config_path,
        );
        let (mut curves, grid) = data::load_data(&database_dir)?;
        self.upload_grid = grid;
        self.uploaded_names.clear();
        let mut phone_books = vec![data::load_phone_book(
            &database_dir.join("phone_book.json"),
        )?];

        let headphone_database_dir = resolve_measurement_dir(
            config.measurements.headphone_database_dir.as_deref(),
            Path::new("data_hp"),
            &config_path,
        );
        if config.measurements.headphone_database_dir.is_some() || headphone_database_dir.is_dir() {
            let (headphone_curves, _) = data::load_data(&headphone_database_dir)?;
            curves.extend(headphone_curves);
            let phone_book_path = headphone_database_dir.join("phone_book.json");
            if phone_book_path.is_file() {
                phone_books.push(data::load_phone_book(&phone_book_path)?);
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
        let delta_dir = resolve_measurement_dir(
            config.measurements.delta_dir.as_deref(),
            Path::new("data_deltas"),
            &config_path,
        );
        self.delta_targets = if delta_dir.is_dir() {
            match delta::load_delta_targets(&delta_dir, &self.upload_grid) {
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

        if self.debug_hooks {
            eprintln!(
                "SquigLab loaded {} curves across {} measurement groups.",
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
        // The library list scrolls independently, so a session restored with a
        // curve selected far down it must scroll that row into view rather
        // than opening at the top of a list of thousands.
        self.library_reveal_pending = !self.active.is_empty() || !self.active_targets.is_empty();
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
            || !preferences
                .eq_filters
                .values()
                .all(|rows| eq_rows_are_valid(rows))
        {
            self.settings_error =
                Some("Unable to load preferences: invalid settings values".into());
            return false;
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
        true
    }

    fn preferences(&self) -> Preferences {
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
            squiglab::color::get_curve_color_with_offset(
                index,
                self.color_offset.wrapping_add(personal_offset),
            )
        }
    }

    /// Whether a group is a curve the user uploaded rather than one from the
    /// reference database. An upload is recorded by name when it is accepted,
    /// because a group can be rebuilt or renamed and the shared state has to
    /// keep excluding it either way.
    fn is_uploaded(&self, index: usize) -> bool {
        let group = &self.groups[index];
        group.brand == UPLOADED_BRAND || self.uploaded_names.contains(&group.name)
    }

    /// Color for a displayed curve. An EQ curve takes a palette slot spread
    /// away from its parent's, so the two lines are tellable apart while still
    /// coming from the graph's existing palette rather than a new hue.
    fn display_curve_color(&self, curve: &DisplayCurve) -> Color32 {
        match curve.source {
            CurveSource::Measurement => self.curve_color(curve.group_index),
            CurveSource::Equalizer => squiglab::color::get_curve_color_with_offset(
                curve.group_index + EQ_CURVE_COLOR_SPREAD,
                self.color_offset,
            ),
        }
    }

    /// The processing and view-toggle cluster: normalization target, smoothing,
    /// and the two view toggles. Drawn inline on wide viewports and inside the
    /// overflow menu on narrow ones, so the two layouts cannot drift apart.
    fn toolbar_processing(&mut self, ui: &mut Ui) {
        use squiglab::ui as cgui;
        // The segmented control and its unit suffix already name what this is,
        // so no cluster label is needed here; the space buys room for the
        // toolbar to stay on one row.
        cgui::segmented(
            ui,
            &[(true, "phon"), (false, "Hz")],
            &mut self.phon_normalization,
            Some("Normalize against a loudness reference (phon) or a reference frequency (Hz)"),
        );
        if self.phon_normalization {
            ui.add(
                egui::DragValue::new(&mut self.target_phon)
                    .range(0.0..=100.0)
                    .speed(1.0)
                    .suffix(" phon"),
            )
            .on_hover_text("Loudness level the curve is normalized to");
        } else {
            ui.add(
                egui::DragValue::new(&mut self.target_hz)
                    .range(20.0..=20000.0)
                    .speed(5.0)
                    .suffix(" Hz"),
            )
            .on_hover_text("Frequency the curve is normalized to");
        }
        ui.add(
            egui::Slider::new(&mut self.smooth_level, 0..=12)
                .suffix(" /oct")
                .text(""),
        )
        .on_hover_text("Fractional-octave smoothing applied to every curve");
        ui.checkbox(&mut self.show_labels, "Labels")
            .on_hover_text("Curve labels are auto-spaced; drag a label to position it.");
        if self.show_labels
            && cgui::action_button(
                ui,
                self.label_layout.short_name(),
                "Group labels in a corner like squig.link, or keep each beside its curve",
            )
        {
            self.label_layout = self.label_layout.next();
        }
        ui.checkbox(&mut self.inspect_mode, "Inspect")
            .on_hover_text("Read the exact dB value under the cursor at a frequency.");
        #[cfg(debug_assertions)]
        if self.debug_hooks {
            ui.checkbox(&mut self.debug_hooks, "Debug")
                .on_hover_text("Show egui widget diagnostics when hovering controls.");
        }
    }

    /// The toolbar's secondary actions: recolouring, exporting, sharing, and the
    /// frequency-guide toggle. Also shared between the inline row and the
    /// overflow menu.
    fn toolbar_actions(&mut self, ui: &mut Ui) {
        use squiglab::ui as cgui;
        if cgui::action_button(ui, "Recolor", "Cycle the measurement curve colors.") {
            self.color_offset = self.color_offset.wrapping_add(1);
        }
        if cgui::action_button(ui, "PNG", "Save a PNG of the current application window.") {
            self.screenshot_pending = true;
            self.screenshot_error = None;
        }
        if cgui::action_button(
            ui,
            "SVG",
            "Save the current graph as a scalable vector image.",
        ) {
            self.export_svg();
        }
        if cgui::action_button(ui, "Share", "Copy a shareable link to this view.") {
            self.share_window_open = true;
            self.share_message = None;
        }
        if cgui::action_button(
            ui,
            "Upload",
            "Add your own measurement or target from a REW or AudioTool file.",
        ) {
            self.open_upload();
        }
        if cgui::action_button(
            ui,
            if self.tutorial_segment.is_some() {
                "Close guide"
            } else {
                "Guide"
            },
            "Learn how to read a frequency response curve.",
        ) {
            self.tutorial_segment = self.tutorial_segment.map_or(Some(0), |_| None);
        }
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        use squiglab::ui as cgui;
        let previous_processing_settings = (
            self.phon_normalization,
            self.target_phon,
            self.target_hz,
            self.smooth_level,
        );
        egui::Panel::top("toolbar").show(ui, |ui| {
            // One row of clearly separated clusters: view range, then the two
            // processing controls that change what is measured (normalize,
            // smooth), then view toggles, then actions, then panel visibility.
            // Rules between clusters carry the grouping so no control needs a
            // box around it. As the viewport narrows, whole clusters move into
            // the overflow menu instead of wrapping onto extra rows.
            let density = ToolbarDensity::for_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = cgui::Spacing::COMPACT;

                if density.shows_wordmark() {
                    cgui::wordmark(ui);
                    cgui::cluster_rule(ui);
                }

                // ---- View range ----
                cgui::zoom_segmented(ui, &mut self.zoom);
                cgui::cluster_rule(ui);

                // ---- Normalize / smooth / view toggles ----
                if density.shows_processing() {
                    self.toolbar_processing(ui);
                    cgui::cluster_rule(ui);
                }

                // ---- Actions (secondary: quiet text buttons) ----
                if density == ToolbarDensity::Full {
                    self.toolbar_actions(ui);
                } else {
                    // Whatever this density dropped out of the row lives here,
                    // so a narrow viewport hides controls without losing them.
                    ui.menu_button("⋯", |ui| {
                        if !density.shows_processing() {
                            self.toolbar_processing(ui);
                            ui.separator();
                        }
                        self.toolbar_actions(ui);
                        if !density.shows_theme()
                            && cgui::action_button(
                                ui,
                                if self.dark_mode { "Light" } else { "Dark" },
                                "Switch between the dark and light themes",
                            )
                        {
                            self.dark_mode = !self.dark_mode;
                        }
                    })
                    .response
                    .on_hover_text("Controls that do not fit the toolbar at this width.");
                }
                cgui::cluster_rule(ui);

                // ---- Appearance ----
                if density.shows_theme() {
                    if cgui::action_button(
                        ui,
                        if self.dark_mode { "Light" } else { "Dark" },
                        "Switch between the dark and light themes",
                    ) {
                        self.dark_mode = !self.dark_mode;
                    }
                    cgui::cluster_rule(ui);
                }

                // ---- Panel visibility (primary toggles, at the end) ----
                if ui
                    .selectable_label(self.show_measurements, "Library")
                    .on_hover_text("Show or hide the measurement and target library.")
                    .clicked()
                {
                    self.show_measurements = !self.show_measurements;
                }
                // The deviation is a second reading of the same selection, so
                // it is a window rather than a place in the layout: the layout
                // is the curves themselves, and this is a question about them.
                if ui
                    .selectable_label(self.show_delta, "\u{394} Deviation")
                    .on_hover_text("Show how far each headphone sits from the target.")
                    .clicked()
                {
                    self.show_delta = !self.show_delta;
                }
            });
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
                        .hint_text("squiglab://state/v1/..."),
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

    fn selector_panel(&mut self, ui: &mut Ui) {
        if !self.show_measurements {
            return;
        }
        if ui.available_width() < COMPACT_LAYOUT_WIDTH {
            // Compact viewport: the library sits below the graph, matching the
            // reference tool's stacked order, and takes the width it needs.
            egui::Panel::bottom("selector")
                .resizable(true)
                .default_size(280.0)
                .size_range(120.0..=620.0)
                .show(ui, |ui| self.library_column(ui));
        } else {
            egui::Panel::left("selector")
                .resizable(true)
                .default_size(250.0)
                .size_range(190.0..=420.0)
                .show(ui, |ui| self.library_column(ui));
        }
    }

    /// The tab column beside the graph: brands, the model list, and the
    /// equalizer share one slot, matching the reference tool's column.
    fn library_column(&mut self, ui: &mut Ui) {
        let mut tab = self.library_tab;
        squiglab::ui::tab_strip(
            ui,
            &mut tab,
            &[
                ("Brands", LibraryTab::Brands),
                ("Models", LibraryTab::Models),
                ("Equalizer", LibraryTab::Equalizer),
            ],
        );
        self.library_tab = tab;
        match tab {
            LibraryTab::Brands => self.brand_list(ui),
            LibraryTab::Models => self.library_contents(ui),
            LibraryTab::Equalizer => self.equalizer_panel(ui),
        }
    }

    /// Every brand in the database, each showing how many models it has, with
    /// the chosen one narrowing the model list.
    fn brand_list(&mut self, ui: &mut Ui) {
        use squiglab::ui as cgui;
        squiglab::ui::section_header(ui, "Brands", None);
        ui.add_space(cgui::Spacing::TIGHT);
        let counts = self.brand_model_counts();
        egui::ScrollArea::vertical()
            .id_salt("brands")
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = cgui::Spacing::TIGHT;
                if self.brand_filter.is_some() && ui.selectable_label(false, "All brands").clicked()
                {
                    self.brand_filter = None;
                }
                for (brand, count) in counts {
                    let selected = self.brand_filter.as_deref() == Some(brand.as_str());
                    let label = RichText::new(format!("{brand} ({count})"))
                        .strong()
                        .size(squiglab::theme::TypeStep::Body.size());
                    if ui.selectable_label(selected, label).clicked() {
                        // Choosing the brand already showing clears the filter.
                        self.brand_filter = (!selected).then(|| brand.clone());
                        self.library_tab = LibraryTab::Models;
                    }
                }
            });
    }

    /// How many models each brand has, unfiltered by the search box so the
    /// counts do not change while typing.
    fn brand_model_counts(&self) -> Vec<(String, usize)> {
        let mut counts = BTreeMap::<String, usize>::new();
        for group in &self.groups {
            if group.target {
                continue;
            }
            counts.entry(brand_name(group)).or_default();
            *counts.get_mut(&brand_name(group)).expect("just inserted") += 1;
        }
        counts.into_iter().collect()
    }

    /// The measurement whose equalizer the panel is editing: the one chosen in
    /// its dropdown, or the first active measurement when nothing is chosen.
    fn eq_phone(&self) -> Option<usize> {
        self.eq_phone
            .filter(|&index| self.active.contains(&index) && !self.groups[index].target)
            .or_else(|| {
                self.active
                    .iter()
                    .copied()
                    .find(|&index| !self.groups[index].target)
            })
    }

    /// The curve Auto EQ fits against: the chosen target, else the first
    /// active target, else the second active measurement, which is the
    /// reference tool's fallback.
    fn eq_target(&self) -> Option<usize> {
        self.eq_target
            .filter(|&index| self.active_targets.contains(&index))
            .or_else(|| self.active_targets.iter().copied().next())
            .or_else(|| self.active.iter().copied().nth(1))
    }

    /// The measurements the panel offers in its dropdown.
    fn active_measurements(&self) -> Vec<usize> {
        self.active
            .iter()
            .copied()
            .filter(|&index| !self.groups[index].target)
            .collect()
    }

    /// The enabled filters of a curve, which is what the exports and the
    /// response calculation take.
    fn enabled_filters(&self, index: usize) -> Vec<equalizer::Filter> {
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

    /// A measurement's normalized both-channel average: the curve Auto EQ fits
    /// filters to, on the same basis the normalization uses, so the filters fit
    /// the curve as it is drawn.
    fn normalized_average_points(
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

    /// A curve on the f64 grid the DSP works in.
    fn response_points(points: &[FRPoint]) -> Vec<equalizer::ResponsePoint> {
        points
            .iter()
            .map(|point| equalizer::ResponsePoint::new(point.hz as f64, point.db as f64))
            .collect()
    }

    /// Fit filters to the edited measurement against the chosen target.
    fn run_auto_eq(&mut self) {
        let (Some(phone), Some(target)) = (self.eq_phone(), self.eq_target()) else {
            self.eq_export_error =
                Some("Auto EQ needs a measurement to fit and a target to fit against".to_owned());
            return;
        };
        let source = match self.normalized_average_points(phone) {
            Ok(points) => Self::response_points(&points),
            Err(error) => {
                self.eq_export_error = Some(format!("Auto EQ: {error}"));
                return;
            }
        };
        let target_curve = match self.normalized_average_points(target) {
            Ok(points) => Self::response_points(&points),
            Err(error) => {
                self.eq_export_error = Some(format!("Auto EQ target: {error}"));
                return;
            }
        };
        let fitted = equalizer::autoeq(&source, &target_curve, self.eq_max_filters);
        self.eq_export_error = None;
        self.eq_export_message = Some(format!(
            "Auto EQ fitted {} filters against {}.",
            fitted.len(),
            self.groups[target].name
        ));
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
    }

    /// The edited curve as an Equalizer APO configuration, named after the
    /// curve as the reference names it. None when no curve is being edited.
    ///
    /// The preamp is the peak boost the filters ask for, computed between the
    /// curve and its equalized response, so the exported curve never boosts.
    fn parametric_export(&self) -> Option<(String, String)> {
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

    /// The edited curve as a Wavelet GraphicEQ configuration, on the same
    /// filter set the parametric export writes.
    fn graphic_eq_export(&self) -> Option<(String, String)> {
        let phone = self.eq_phone()?;
        Some((
            format!("{} Wavelet.txt", self.groups[phone].name),
            equalizer::export_graphic_eq(
                &self.enabled_filters(phone),
                equalizer::DEFAULT_SAMPLE_RATE,
            ),
        ))
    }

    /// Report an export in the panel, or the error that stopped it.
    fn report_export(&mut self, exported: Option<(String, String)>, what: &str) {
        let Some((name, contents)) = exported else {
            self.eq_export_error = Some(format!("{what} export needs a measurement first."));
            return;
        };
        match save_export(&name, &contents) {
            Ok(path) => {
                self.eq_export_error = None;
                self.eq_export_message = Some(format!("{what} exported to {}", path.display()));
            }
            Err(error) => {
                self.eq_export_message = None;
                self.eq_export_error = Some(format!("{what} export failed: {error:#}"));
            }
        }
    }

    /// Read an Equalizer APO configuration from disk into the edited curve.
    fn import_parametric(&mut self) {
        let Some(phone) = self.eq_phone() else {
            self.eq_import_error = Some("Select a measurement to import into first.".to_owned());
            return;
        };
        let text = match std::fs::read_to_string(&self.eq_import_path) {
            Ok(text) => text,
            Err(error) => {
                self.eq_import_error = Some(format!("Unable to read the file: {error:#}"));
                return;
            }
        };
        match equalizer::import_parametric(&text) {
            Ok(imported) => {
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
                self.eq_import_error = None;
                self.eq_export_message = Some(format!("Imported {count} filters."));
            }
            Err(error) => {
                self.eq_import_error = Some(format!("Unable to import the file: {error:#}"));
            }
        }
    }

    /// Read an uploaded file into the database of curves, as a measurement or as
    /// a target.
    ///
    /// An uploaded measurement lands in a synthetic "Uploaded" brand, so it is
    /// never confused with the reference database's, and its curves are not
    /// written into shared links: a measurement from someone else's rig is not
    /// the thing the link was sharing. A target upload is named "<name> Target"
    /// and dropped if the database already has that target, which is how the
    /// reference deduplicates uploads against its configured targets.
    fn import_upload(&mut self) {
        let file_name = match std::path::Path::new(&self.upload_path)
            .file_stem()
            .and_then(|stem| stem.to_str())
        {
            Some(name) if !name.is_empty() => name.to_owned(),
            _ => {
                self.upload_error = Some("That path has no file name in it.".to_owned());
                return;
            }
        };
        let contents = match std::fs::read_to_string(&self.upload_path) {
            Ok(contents) => contents,
            Err(error) => {
                self.upload_error = Some(format!("Unable to read the file: {error:#}"));
                return;
            }
        };
        let target = self.upload_as_target;
        let uploaded = match data::parse_upload(&file_name, &contents, &self.upload_grid, target) {
            Ok(uploaded) => uploaded,
            Err(error) => {
                self.upload_error = Some(format!("{file_name} was not accepted: {error}"));
                return;
            }
        };
        if target {
            // A target the database already carries would draw twice, so the
            // upload is refused rather than shadowing it.
            if self
                .groups
                .iter()
                .any(|group| group.target && group.name == file_name)
            {
                self.upload_error =
                    Some(format!("{file_name} is already a target in this database."));
                return;
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
        if !target && right_only {
            self.curves.push(data::CurveInfo {
                model,
                variant: "L".to_owned(),
                points: self
                    .upload_grid
                    .iter()
                    .map(|frequency| FRPoint {
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
        self.rebuild_uploaded_group(&file_name, target, &indices);
        self.upload_error = None;
        self.upload_message = Some(if target {
            format!("Added {file_name} as a target.")
        } else {
            format!("Added {file_name} under Uploaded.")
        });
        self.dirty = true;
    }

    /// Rebuild the group an uploaded curve belongs to, so a second upload of
    /// the same name joins the first rather than making a second group.
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

    /// The equalizer's "How to use" window: the four paths the reference's
    /// onboarding modal offers, in the same order and with the same intent.
    ///
    /// The difference is that each path here performs what it describes rather
    /// than describing it. A modal that only talks about Auto EQ is worse
    /// guidance than the button that runs it, and the reference's own copy
    /// ("What do you want to do?") is a question the app can answer by
    /// pressing a button.
    fn onboarding_window(&mut self, ctx: &Context) {
        if !self.onboarding_open {
            return;
        }
        let mut open = true;
        let mut finished = false;
        let palette = squiglab::theme::Palette::resolve(self.dark_mode);
        egui::Window::new("How to use the equalizer")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("How to use the equalizer")
                        .font(squiglab::theme::font_bold(squiglab::theme::TypeStep::Heading))
                        .color(palette.text_primary),
                );
                ui.add_space(squiglab::ui::Spacing::COMPACT);
                ui.label(
                    RichText::new(
                        "The equalizer customises the sound of any headphone. What do you want to do?",
                    )
                    .color(palette.text_secondary),
                );
                ui.add_space(squiglab::ui::Spacing::COMPACT);

                // Each path is collapsible, as the reference's are, so the
                // window can be read without committing to anything.
                let sections: [(&str, &str, &str); 4] = [
                    (
                        "Beginner",
                        "Fit a correction to a target response.",
                        "Choose a target in the dropdown above and press Auto EQ. It fits bands to move the measurement as close to that target as the bands allow, and you can adjust any of them afterwards.",
                    ),
                    (
                        "Intermediate",
                        "Fit a correction to another headphone.",
                        "Choose a second measurement as the target instead. Auto EQ will match this headphone to that one, which is how you hear one pair's sound in another.",
                    ),
                    (
                        "Advanced",
                        "Make manual corrections yourself.",
                        "Every band is editable: its type, frequency, gain, and Q, and each can be switched off without losing it. A few wide bands usually sound better than many narrow ones.",
                    ),
                    (
                        "Export",
                        "Take the bands to your system.",
                        "Export writes an Equalizer APO configuration for a parametric EQ, or the Wavelet GraphicEQ format. Both carry a preamp, so the result never boosts.",
                    ),
                ];
                for (index, (title, summary, detail)) in sections.into_iter().enumerate() {
                    let header = egui::CollapsingHeader::new(
                        RichText::new(format!("{title}: {summary}")).strong(),
                    )
                    .default_open(self.onboarding_section == index)
                    .id_salt(("onboarding", index))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(detail).color(palette.text_secondary));
                        match index {
                            // Auto EQ to the chosen target, falling back to the
                            // reference's first active target.
                            0 | 1 => {
                                if ui.button("Auto EQ now").clicked() {
                                    self.run_auto_eq();
                                    finished = true;
                                }
                            }
                            2 => {
                                if ui.button("Show the bands").clicked() {
                                    finished = true;
                                }
                            }
                            _ => {
                                if ui.button("Export now").clicked() {
                                    let exported = self.parametric_export();
                                    self.report_export(exported, "Parametric EQ");
                                    finished = true;
                                }
                            }
                        }
                    });
                    if header.header_response.clicked() {
                        self.onboarding_section = index;
                    }
                }
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Done").clicked() {
                            finished = true;
                        }
                    });
                });
            });
        self.onboarding_open = open && !finished;
    }

    /// The upload window. The reference asks for a file; egui has no file
    /// picker without a new dependency, so the path is typed, as it is for
    /// importing a parametric configuration.
    fn upload_window(&mut self, ctx: &Context) {
        use squiglab::ui as cgui;
        if !self.upload_open {
            return;
        }
        let mut open = true;
        let mut cancelled = false;
        let palette = squiglab::theme::Palette::resolve(self.dark_mode);
        egui::Window::new("Upload measurement")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_width(460.0)
            .show(ctx, |ui| {
                // The reference states this in the upload itself rather than as
                // a dismissible banner, because a measurement taken on another
                // rig is not comparable with the database's and nothing in the
                // curve will show that.
                ui.label(
                    RichText::new(
                        "Measurements from another rig are not comparable with this database.",
                    )
                    .color(palette.warning)
                    .small(),
                );
                ui.add_space(cgui::Spacing::TIGHT);
                ui.label("Path to a REW or AudioTool export:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.upload_path)
                        .hint_text("/path/to/measurement.txt"),
                );
                ui.checkbox(
                    &mut self.upload_as_target,
                    "Upload as a target instead of a measurement",
                )
                .on_hover_text(
                    "A target is a curve to correct towards, not a measurement of a device.",
                );
                ui.horizontal(|ui| {
                    if ui.button("Upload").clicked() {
                        self.import_upload();
                    }
                    if ui.button("Cancel").clicked() {
                        cancelled = true;
                    }
                });
                if let Some(error) = &self.upload_error {
                    ui.colored_label(palette.error, error);
                }
                if let Some(message) = &self.upload_message {
                    ui.label(message);
                }
            });
        self.upload_open = open && !cancelled;
    }

    /// Open the upload window from the toolbar's overflow menu.
    fn open_upload(&mut self) {
        self.upload_open = true;
        self.upload_error = None;
        self.upload_message = None;
    }

    /// The sound tools: a tone generator and pink noise, with the Compare A | B
    /// toggle the reference puts above them.
    ///
    /// The player is created on the first play and dropped on stop, so nothing
    /// is held open while the sound is not wanted.
    fn sound_tools(&mut self, ui: &mut Ui) {
        use squiglab::audio::Source;
        use squiglab::ui as cgui;
        let palette = squiglab::theme::Palette::resolve(ui.visuals().dark_mode);
        ui.add_space(cgui::Spacing::RELAXED);
        cgui::section_header(ui, "Sound Tools", None);
        ui.add_space(cgui::Spacing::TIGHT);
        ui.horizontal(|ui| {
            cgui::segmented(
                ui,
                &[(0usize, "Compare A"), (1usize, "Compare B")],
                &mut self.sound_slot,
                Some("Switch between the two sound settings"),
            );
            let playing = self.player.is_some();
            if cgui::action_button(
                ui,
                if playing { "Stop" } else { "Play" },
                "Play or stop the tone generator",
            ) {
                if playing {
                    self.stop_sound();
                } else {
                    self.start_sound();
                }
            }
        });
        ui.add_space(cgui::Spacing::TIGHT);

        // Edited on a copy, so the panel does not hold a borrow across the
        // player's own state.
        let mut settings = self.sound_settings[self.sound_slot];
        let mut choice = match settings.source {
            Source::Tone { .. } => 0,
            Source::PinkNoise => 1,
            Source::WhiteNoise => 2,
        };
        if cgui::segmented(
            ui,
            &[(0usize, "Tone"), (1usize, "Pink"), (2usize, "White")],
            &mut choice,
            Some("The generator to audition"),
        ) {
            settings.source = match (choice, settings.source) {
                // Keep the frequency the tone was left at when switching back.
                (0, Source::Tone { freq }) => Source::Tone { freq },
                (0, _) => Source::Tone { freq: 1000.0 },
                (1, _) => Source::PinkNoise,
                _ => Source::WhiteNoise,
            };
        }
        if let Source::Tone { freq } = &mut settings.source {
            // A log slider, because pitch is heard geometrically and a linear
            // one would crowd every low frequency into a corner.
            ui.add(
                egui::Slider::new(freq, 20.0..=20_000.0)
                    .logarithmic(true)
                    .suffix(" Hz")
                    .text("Tone"),
            );
        }
        ui.add(egui::Slider::new(&mut settings.volume, 0.0..=1.0).text("Volume"));
        self.sound_settings[self.sound_slot] = settings;
        // A playing tone follows the controls: the pitch glides to a new
        // frequency rather than jumping, which is the reference's behaviour
        // when the slider is dragged.
        if let Some(player) = &self.player {
            player.set_settings(settings);
        }
        if let Some(message) = self.player.as_mut().and_then(Player::take_message) {
            self.sound_message = Some(message);
        }
        if let Some(message) = &self.sound_message {
            ui.colored_label(palette.text_secondary, message);
        }
    }

    /// Start the sound, reporting why not if the audio path is unavailable.
    fn start_sound(&mut self) {
        let settings = self.sound_settings[self.sound_slot];
        match Player::start(settings) {
            Ok(player) => {
                self.player = Some(player);
                self.sound_message = Some(format!("Playing {}.", settings.source.name()));
            }
            Err(error) => {
                self.sound_message = Some(error);
            }
        }
    }

    /// Stop the sound. Dropping the player closes the stream and joins the
    /// thread that was writing to it.
    fn stop_sound(&mut self) {
        self.player = None;
        self.sound_message = Some("Stopped.".to_owned());
    }

    /// The parametric equalizer panel: the curve being edited, its filter
    /// table, Auto EQ, and the two export formats, laid out as the reference
    /// arranges them.
    fn equalizer_panel(&mut self, ui: &mut Ui) {
        use squiglab::ui as cgui;
        let palette = squiglab::theme::Palette::resolve(ui.visuals().dark_mode);

        // --- title, reset, and the Auto EQ budget ---
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Parametric Equalizer")
                    .font(squiglab::theme::font_bold(
                        squiglab::theme::TypeStep::Subtitle,
                    ))
                    .color(palette.text_primary),
            );
            // The reference's title carries a "?" that opens its guidance; the
            // same control opens the guidance here.
            if ui
                .add(egui::Button::new(
                    RichText::new("?").color(palette.text_secondary),
                ))
                .on_hover_text("How to use the equalizer")
                .clicked()
            {
                self.onboarding_open = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.menu_button("Settings", |ui| {
                    ui.label(
                        RichText::new("Auto EQ")
                            .size(squiglab::theme::TypeStep::Caption.size())
                            .color(palette.text_secondary),
                    );
                    let mut budget = self.eq_max_filters as f64;
                    if ui
                        .add(
                            egui::Slider::new(&mut budget, 1.0..=MAX_EQ_ROWS as f64)
                                .text("filters"),
                        )
                        .changed()
                    {
                        self.eq_max_filters = budget.clamp(1.0, MAX_EQ_ROWS as f64) as usize;
                    }
                });
                if cgui::action_button(ui, "Reset", "Clear this curve's filters.") {
                    if let Some(phone) = self.eq_phone() {
                        self.eq_filters.insert(phone, Vec::new());
                        self.dirty = true;
                    }
                }
            });
        });

        let Some(phone) = self.eq_phone() else {
            ui.add_space(cgui::Spacing::TIGHT);
            ui.label("Select a measurement to give it an equalizer.");
            return;
        };
        // A curve that has never been edited starts with the default bands. A
        // curve whose rows were emptied keeps them empty: the empty list is the
        // record that the user cleared it.
        if let std::collections::btree_map::Entry::Vacant(slot) = self.eq_filters.entry(phone) {
            slot.insert(default_eq_filters());
            self.dirty = true;
        }

        // --- the curve being edited, the target, and Auto EQ ---
        ui.add_space(cgui::Spacing::TIGHT);
        let measurements = self.active_measurements();
        let names: Vec<String> = measurements
            .iter()
            .map(|&index| self.groups[index].name.clone())
            .collect();
        let mut selected_phone = Some(phone);
        egui::ComboBox::from_id_salt("eq_phone")
            .selected_text(
                measurements
                    .iter()
                    .position(|&index| Some(index) == selected_phone)
                    .map(|position| names[position].as_str())
                    .unwrap_or("Measurement"),
            )
            .show_ui(ui, |ui| {
                for (index, name) in measurements.iter().zip(&names) {
                    ui.selectable_value(&mut selected_phone, Some(*index), name);
                }
            });
        if selected_phone != Some(phone) {
            self.eq_phone = selected_phone;
        }
        let targets: Vec<usize> = self.active_targets.iter().copied().collect();
        let mut selected_target = self.eq_target();
        let target_label = selected_target
            .map(|index| self.groups[index].name.clone())
            .unwrap_or_else(|| "None".to_owned());
        egui::ComboBox::from_id_salt("eq_target")
            .selected_text(target_label.as_str())
            .show_ui(ui, |ui| {
                for index in &targets {
                    ui.selectable_value(
                        &mut selected_target,
                        Some(*index),
                        &self.groups[*index].name,
                    );
                }
            });
        if selected_target != self.eq_target() {
            self.eq_target = selected_target;
        }
        if ui.button("Auto EQ").clicked() {
            self.run_auto_eq();
        }

        // --- the filter table ---
        ui.add_space(cgui::Spacing::COMPACT);
        let header = |ui: &mut Ui, text: &str| {
            ui.label(
                RichText::new(text)
                    .size(squiglab::theme::TypeStep::Micro.size())
                    .color(palette.text_tertiary),
            );
        };
        egui::Grid::new("eq_filters")
            .num_columns(5)
            .spacing([cgui::Spacing::TIGHT, cgui::Spacing::TIGHT])
            .show(ui, |ui| {
                header(ui, "Type");
                header(ui, "Frequency");
                header(ui, "Gain");
                header(ui, "Q");
                ui.allocate_space(egui::vec2(20.0, 0.0));
                ui.end_row();
            });

        let mut rows = self.eq_filters.get(&phone).cloned().unwrap_or_default();
        let mut removals = Vec::new();
        let mut changed = false;
        let mut sort_requested = false;
        egui::Grid::new("eq_rows")
            .num_columns(5)
            .spacing([cgui::Spacing::TIGHT, cgui::Spacing::TIGHT])
            .show(ui, |ui| {
                for (row_index, row) in rows.iter_mut().enumerate() {
                    let mut enabled = row.enabled;
                    if ui
                        .checkbox(&mut enabled, "")
                        .on_hover_text("Include this band in the response and exports")
                        .changed()
                    {
                        row.enabled = enabled;
                        changed = true;
                    }
                    egui::ComboBox::from_id_salt(("eq_type", row_index))
                        .selected_text(row.filter.filter_type.short_name())
                        .show_ui(ui, |ui| {
                            for filter_type in [
                                equalizer::FilterType::Peaking,
                                equalizer::FilterType::LowShelf,
                                equalizer::FilterType::HighShelf,
                            ] {
                                ui.selectable_value(
                                    &mut row.filter.filter_type,
                                    filter_type,
                                    filter_type.short_name(),
                                );
                            }
                        });
                    let mut frequency = row.filter.freq;
                    let mut gain = row.filter.gain;
                    let mut q = row.filter.q;
                    if ui
                        .add(
                            egui::DragValue::new(&mut frequency)
                                .range(20.0..=20_000.0)
                                .speed(10.0)
                                .suffix(" Hz"),
                        )
                        .changed()
                    {
                        row.filter.freq = frequency;
                        changed = true;
                    }
                    if ui
                        .add(
                            egui::DragValue::new(&mut gain)
                                .range(-40.0..=40.0)
                                .speed(0.1)
                                .suffix(" dB"),
                        )
                        .changed()
                    {
                        row.filter.gain = gain;
                        changed = true;
                    }
                    if ui
                        .add(egui::DragValue::new(&mut q).range(0.1..=20.0).speed(0.1))
                        .changed()
                    {
                        row.filter.q = q;
                        changed = true;
                    }
                    if ui
                        .button("\u{2715}")
                        .on_hover_text("Remove this band")
                        .clicked()
                    {
                        removals.push(row_index);
                    }
                    ui.end_row();
                }
            });
        for row_index in removals.into_iter().rev() {
            rows.remove(row_index);
            changed = true;
        }
        if changed {
            self.eq_filters.insert(phone, rows);
            self.dirty = true;
        }

        // --- row controls ---
        ui.add_space(cgui::Spacing::TIGHT);
        ui.horizontal(|ui| {
            if ui
                .button("+")
                .on_hover_text("Add a band after the last one")
                .clicked()
            {
                let mut rows = self.eq_filters.get(&phone).cloned().unwrap_or_default();
                rows.push(EqRow {
                    filter: equalizer::Filter::new(
                        equalizer::FilterType::Peaking,
                        1000.0,
                        1.0,
                        0.0,
                    ),
                    enabled: true,
                });
                self.eq_filters.insert(phone, rows);
                self.dirty = true;
            }
            if ui
                .button("Sort")
                .on_hover_text("Order the bands by frequency")
                .clicked()
            {
                sort_requested = true;
            }
        });
        if sort_requested {
            if let Some(mut rows) = self.eq_filters.get(&phone).cloned() {
                rows.sort_by(|left, right| left.filter.freq.total_cmp(&right.filter.freq));
                self.eq_filters.insert(phone, rows);
                self.dirty = true;
            }
        }

        // --- the two export formats ---
        ui.add_space(cgui::Spacing::TIGHT);
        ui.horizontal(|ui| {
            if cgui::action_button(ui, "Import", "Load an Equalizer APO file.") {
                self.eq_import_open = true;
                self.eq_import_error = None;
            }
            if cgui::action_button(ui, "Export", "Save an Equalizer APO file.") {
                let exported = self.parametric_export();
                self.report_export(exported, "Parametric EQ");
            }
        });
        if ui
            .add_sized(
                [ui.available_width(), 24.0],
                egui::Button::new("Export Graphic EQ (Wavelet)"),
            )
            .clicked()
        {
            let exported = self.graphic_eq_export();
            self.report_export(exported, "Graphic EQ");
        }
        if let Some(message) = &self.eq_export_message {
            ui.label(message);
        }
        if let Some(error) = &self.eq_export_error {
            ui.colored_label(palette.error, error);
        }

        self.sound_tools(ui);
    }

    /// The Equalizer APO import window. The reference opens a file picker;
    /// egui has none without a new dependency, so the path is typed instead.
    fn equalizer_import_window(&mut self, ctx: &Context) {
        if !self.eq_import_open {
            return;
        }
        let mut open = true;
        let mut cancelled = false;
        egui::Window::new("Import parametric EQ")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.label("Path to an Equalizer APO configuration:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.eq_import_path)
                        .hint_text("/path/to/Model Filters.txt"),
                );
                ui.horizontal(|ui| {
                    if ui.button("Import").clicked() {
                        self.import_parametric();
                    }
                    if ui.button("Cancel").clicked() {
                        cancelled = true;
                    }
                });
                if let Some(error) = &self.eq_import_error {
                    ui.colored_label(
                        squiglab::theme::Palette::resolve(ui.visuals().dark_mode).error,
                        error,
                    );
                }
            });
        self.eq_import_open = open && !cancelled;
    }

    /// The library body, shared by both viewports so the two layouts cannot
    /// drift apart.
    fn library_contents(&mut self, ui: &mut Ui) {
        use squiglab::ui as cgui;
        let palette = squiglab::theme::Palette::resolve(ui.visuals().dark_mode);
        squiglab::ui::section_header(
            ui,
            "Library",
            Some(&format!("{}/{}", self.active.len(), MAX_ACTIVE_CURVES)),
        );
        ui.add_space(cgui::Spacing::TIGHT);
        ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text("Search measurements and targets")
                .font(squiglab::theme::font(squiglab::theme::TypeStep::Body))
                .text_color(palette.text_primary),
        );
        ui.add_space(cgui::Spacing::TIGHT);

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
            let brand = brand_name(group);
            let narrowed = self
                .brand_filter
                .as_ref()
                .is_none_or(|chosen| chosen == &brand);
            if narrowed && matches_search(&format!("{brand} {}", group.name), &self.search) {
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
        let mut section = LibrarySection::default();
        egui::ScrollArea::vertical()
            .id_salt("library")
            .show(ui, |ui| {
                cgui::section_header(ui, "Measurements", None);
                ui.add_space(cgui::Spacing::MICRO);
                for (brand, models) in measurements {
                    self.brand_section(ui, "library_brand", &brand, &models, false, &mut section);
                }
                ui.add_space(cgui::Spacing::TIGHT);
                ui.separator();
                cgui::section_header(ui, "Targets", Some(&self.active_targets.len().to_string()));
                ui.add_space(cgui::Spacing::MICRO);
                for (category, models) in targets {
                    self.brand_section(
                        ui,
                        "library_target",
                        &category,
                        &models,
                        true,
                        &mut section,
                    );
                }
                ui.add_space(cgui::Spacing::TIGHT);
                ui.separator();
                self.delta_library_section(ui, &mut section);
            });
        for index in section.toggles {
            self.toggle_curve(index);
        }
        for index in section.delta_toggles {
            self.toggle_delta(index);
        }
        // Scroll the first selected row into view once: a session restored
        // with a curve far down a list of thousands would otherwise open at
        // the top with its selection off screen.
        if self.library_reveal_pending {
            self.library_reveal_pending = false;
            if let Some(rect) = section.reveal_rect {
                ui.scroll_to_rect(rect, None);
            }
        }
    }

    /// The delta targets in the library.
    ///
    /// They sit apart from the targets they correct against, because they are
    /// not curves to compare: selecting one says the readings were taken on a
    /// rig that needs correcting, and nothing else. They never spend one of the
    /// eight measurement slots, for the same reason targets do not.
    fn delta_library_section(&mut self, ui: &mut Ui, section: &mut LibrarySection) {
        use squiglab::ui as cgui;
        cgui::section_header(
            ui,
            "Delta targets",
            (!self.active_deltas.is_empty()).then_some(&self.active_deltas.len().to_string()),
        );
        ui.add_space(cgui::Spacing::MICRO);
        if let Some(error) = &self.delta_error {
            ui.colored_label(Color32::LIGHT_RED, error);
            return;
        }
        if self.delta_targets.is_empty() {
            ui.label(
                RichText::new("No rig corrections in the delta directory.")
                    .size(squiglab::theme::TypeStep::Body.size()),
            );
            return;
        }
        for index in 0..self.delta_targets.len() {
            let target = &self.delta_targets[index];
            let mut selected = self.active_deltas.contains(&index);
            let row = ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = cgui::Spacing::COMPACT;
                let response = ui.checkbox(
                    &mut selected,
                    RichText::new(&target.name).size(squiglab::theme::TypeStep::Body.size()),
                );
                // A placeholder corrects nothing, so saying so next to the name
                // is the difference between a stub and a rig someone trusts.
                if target.is_placeholder() {
                    ui.colored_label(
                        squiglab::theme::Palette::resolve(ui.visuals().dark_mode).warning,
                        "placeholder",
                    )
                    .on_hover_text(
                        "Built from placeholder calibrations, not measurements. It corrects nothing.",
                    );
                }
                response
            });
            if let Some(note) = &target.note {
                row.response.on_hover_text(note);
            }
            if row.inner.changed() {
                section.delta_toggles.push(index);
            }
            ui.spacing_mut().item_spacing.y = cgui::Spacing::MICRO;
        }
    }

    /// Turn a rig correction on or off.
    fn toggle_delta(&mut self, index: usize) {
        if self.active_deltas.contains(&index) {
            self.active_deltas.remove(&index);
        } else {
            self.active_deltas.insert(index);
        }
        self.dirty = true;
    }

    /// One brand of the library: a collapsing header and its checkboxes.
    ///
    /// The header's persistent id is derived from the brand name, so its open
    /// state survives the search filter changing the counts in its title, and a
    /// brand holding a selected curve opens by default so the selection is not
    /// hidden inside a collapsed brand.
    fn brand_section(
        &mut self,
        ui: &mut Ui,
        id_salt: &str,
        name: &str,
        models: &[(usize, String)],
        target: bool,
        section: &mut LibrarySection,
    ) {
        use squiglab::ui as cgui;
        let contains_active = models
            .iter()
            .any(|(index, _)| self.active.contains(index) || self.active_targets.contains(index));
        let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            egui::Id::new((id_salt, name)),
            contains_active,
        );
        if self.library_reveal_pending && contains_active {
            state.set_open(true);
        }
        let header_response = state.show_header(ui, |ui| {
            ui.label(RichText::new(format!("{name} ({})", models.len())).strong());
        });
        header_response.body(|ui| {
            ui.spacing_mut().item_spacing.y = cgui::Spacing::TIGHT;
            for (index, model_name) in models {
                let mut selected = if target {
                    self.active_targets.contains(index)
                } else {
                    self.active.contains(index)
                };
                // Targets do not spend a measurement slot, so they stay
                // checkable when the slots are full; measurements do not.
                let enabled = if target {
                    !self.pinned.contains(index) || !selected
                } else {
                    !self.pinned.contains(index)
                        && (selected || self.active.len() < MAX_ACTIVE_CURVES)
                };
                let mut response = ui.add_enabled(
                    enabled,
                    egui::Checkbox::new(
                        &mut selected,
                        RichText::new(model_name).size(squiglab::theme::TypeStep::Body.size()),
                    ),
                );
                if target {
                    response =
                        response.on_hover_text("Target curves do not use a measurement slot.");
                }
                if selected && section.reveal_rect.is_none() {
                    section.reveal_rect = Some(response.rect);
                }
                if response.changed() {
                    section.toggles.push(*index);
                }
            }
        });
    }

    fn manager_panel(&mut self, ui: &mut Ui) {
        use squiglab::ui as cgui;
        // Unconditional. The manager is the only place the selection is
        // accounted for: which headphones, which targets, and which rig
        // corrections are in force. It is also what fills the space the graph
        // leaves below its plot, so hiding it left that space dead grey with
        // nothing in it, which read as the app having failed to draw.
        // The graph no longer fills the window, so the manager can reclaim the
        // space it freed: one row per active curve, no scrolling, and stop
        // growing once every curve is visible. The panel stays resizable, but
        // never below what its contents need; see the minimum below.
        // A selected rig correction occupies a row here like any other curve:
        // it is something the graph is currently being corrected by, and it has
        // to be visible and removable in the same place as everything else it
        // is doing.
        let curve_count = self.active.len() + self.active_targets.len() + self.active_deltas.len();
        // Size the panel from the row height this viewport will actually use,
        // so every selected curve is visible without scrolling inside a panel
        // that is too short to hold it.
        let (default_height, min_height) = manager_heights(ui.available_width(), curve_count);
        // The manager sits under the graph and spans the full width, matching
        // the reference tool. A row has the whole width to work with, so curve
        // identity sits on the left and every control for that curve lines up on
        // the right without the row ever needing to wrap.
        egui::Panel::bottom("curve_manager")
            .resizable(true)
            .default_size(default_height)
            .size_range(min_height..=MANAGER_MAX_HEIGHT)
            .show(ui, |ui| {
                let count =
                    self.active.len() + self.active_targets.len() + self.active_deltas.len();
                squiglab::ui::section_header(
                    ui,
                    "Active curves",
                    (count > 0).then_some(&count.to_string()),
                );
                ui.add_space(cgui::Spacing::TIGHT);
                if count == 0 {
                    ui.label("Select up to eight measurements.");
                    return;
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut removals = Vec::new();
                    let mut delta_removals = Vec::new();
                    // A row too narrow for identity and controls side by side
                    // stacks them instead of clipping the controls off the edge.
                    let stack_rows = ui.available_width() < MANAGER_ROW_STACK_WIDTH;
                    for index in self
                        .active
                        .iter()
                        .chain(self.active_targets.iter())
                        .copied()
                        .collect::<Vec<_>>()
                    {
                        let group = self.groups[index].clone();
                        self.manager_row(ui, index, &group, stack_rows, &mut removals);
                        ui.separator();
                    }
                    // Rig corrections come last: they are corrections to what
                    // is above them, not independent curves to read against.
                    for index in self.active_deltas.iter().copied().collect::<Vec<_>>() {
                        let target = self.delta_targets[index].clone();
                        self.manager_delta_row(ui, index, &target, stack_rows, &mut delta_removals);
                        ui.separator();
                    }
                    for index in removals {
                        self.toggle_curve(index);
                    }
                    for index in delta_removals {
                        self.toggle_delta(index);
                    }
                });
            });
    }

    /// One rig-correction row in the manager.
    ///
    /// A delta target is not a curve, so it is offered none of the per-curve
    /// controls: no channel mode, no baseline, no offset. What it does have is
    /// the pair of rigs it stands between, which is the claim it makes and the
    /// one worth being able to read at a glance.
    fn manager_delta_row(
        &mut self,
        ui: &mut Ui,
        index: usize,
        target: &DeltaTarget,
        stack_controls: bool,
        removals: &mut Vec<usize>,
    ) {
        use squiglab::ui as cgui;
        let palette = squiglab::theme::Palette::resolve(ui.visuals().dark_mode);
        let identity = |ui: &mut Ui| {
            ui.label(
                egui::RichText::new(&target.name)
                    .strong()
                    .size(squiglab::theme::TypeStep::Body.size())
                    .color(palette.text_secondary),
            );
            ui.label(
                egui::RichText::new(format!("{} → {}", target.clone_rig, target.original_rig))
                    .size(squiglab::theme::TypeStep::Micro.size())
                    .color(palette.text_tertiary),
            );
            if target.is_placeholder() {
                ui.colored_label(palette.warning, "placeholder")
                    .on_hover_text(
                    "Built from placeholder calibrations, not measurements. It corrects nothing.",
                );
            }
        };
        let controls = |ui: &mut Ui| {
            ui.spacing_mut().item_spacing.x = cgui::Spacing::COMPACT;
            if cgui::action_button(ui, "Remove", "Stop applying this rig correction.") {
                removals.push(index);
            }
        };
        if stack_controls {
            ui.vertical(|ui| {
                ui.horizontal(identity);
                ui.horizontal_wrapped(controls);
            });
        } else {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = cgui::Spacing::COMPACT;
                identity(ui);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), controls);
            });
        }
    }

    /// One curve row in the manager.
    ///
    /// Wide viewports keep the row on one line — identity on the left, controls
    /// right-aligned against it — which is both the reference's shape and what
    /// lets every selected curve be visible at once. Only a viewport too narrow
    /// to hold that line stacks the controls onto a second one.
    fn manager_row(
        &mut self,
        ui: &mut Ui,
        index: usize,
        group: &MeasurementGroup,
        stack_controls: bool,
        removals: &mut Vec<usize>,
    ) {
        if stack_controls {
            ui.vertical(|ui| {
                ui.horizontal(|ui| self.manager_row_identity(ui, index, group));
                // Wrapped, so a long control list flows rather than clipping.
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = squiglab::ui::Spacing::COMPACT;
                    self.manager_row_controls(ui, index, group, false, removals);
                });
            });
        } else {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = squiglab::ui::Spacing::COMPACT;
                self.manager_row_identity(ui, index, group);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = squiglab::ui::Spacing::COMPACT;
                    self.manager_row_controls(ui, index, group, true, removals);
                });
            });
        }
    }

    /// A row's identity: its colour swatch, its name, and the variant chooser.
    fn manager_row_identity(&mut self, ui: &mut Ui, index: usize, group: &MeasurementGroup) {
        use squiglab::ui as cgui;
        let palette = squiglab::theme::Palette::resolve(ui.visuals().dark_mode);
        let color = self.curve_color(index);
        // A filled disc rather than a square: the reference tool marks a curve
        // with one, and a disc reads as a swatch of ink rather than a chip of UI.
        let (swatch, _) = ui.allocate_exact_size(egui::vec2(11.0, 11.0), egui::Sense::hover());
        ui.painter().circle_filled(swatch.center(), 5.5, color);
        if group.target {
            // A target says so, because the row is the reference the other rows
            // are read against rather than a curve in its own right, and
            // colouring it like one invites the opposite reading.
            ui.label(
                egui::RichText::new(format!("Target: {}", group.name))
                    .strong()
                    .size(squiglab::theme::TypeStep::Body.size())
                    .color(palette.text_secondary),
            );
        } else {
            ui.label(
                egui::RichText::new(&group.name)
                    .strong()
                    .size(squiglab::theme::TypeStep::Body.size())
                    .color(color),
            );
        }
        if !group.brand.is_empty() {
            ui.label(
                egui::RichText::new(&group.brand)
                    .size(squiglab::theme::TypeStep::Micro.size())
                    .color(palette.text_tertiary),
            );
        }

        // Variant chooser, when the model has more than one. A leading-plus mark
        // rather than a combo box, so it reads as adding to the row rather than
        // as a form control sitting in one.
        if group.variants.len() > 1 {
            let mut selected_variant = self
                .variant_choices
                .get(&index)
                .copied()
                .unwrap_or(0)
                .min(group.variants.len() - 1);
            if cgui::icon_button_tinted(
                ui,
                cgui::Icon::Plus,
                &format!(
                    "Using {} of {} variants. Click for the others.",
                    group.variants[selected_variant].name,
                    group.variants.len(),
                ),
                Some(palette.text_tertiary),
            )
            .clicked()
            {
                egui::ComboBox::from_id_salt(("variant", index))
                    .selected_text(&group.variants[selected_variant].name)
                    .show_ui(ui, |ui| {
                        for (variant_index, variant) in group.variants.iter().enumerate() {
                            ui.selectable_value(
                                &mut selected_variant,
                                variant_index,
                                &variant.name,
                            );
                        }
                    });
                if self.variant_choices.get(&index).copied().unwrap_or(0) != selected_variant {
                    self.variant_choices.insert(index, selected_variant);
                    self.dirty = true;
                }
            }
        }
    }

    /// A row's preview: the curve itself, drawn small.
    ///
    /// The fastest way to tell which headphone a row is when the name is not
    /// familiar, and the reason the reference tool puts one beside each row. A
    /// target is drawn as a reference line instead, because a target is the
    /// thing the others are read against and has no shape of its own to show.
    fn curve_preview(
        &self,
        ui: &mut Ui,
        group_index: usize,
        color: egui::Color32,
    ) -> egui::Response {
        let palette = squiglab::theme::Palette::resolve(ui.visuals().dark_mode);
        let is_target = self.groups[group_index].target;
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(PREVIEW_WIDTH, PREVIEW_HEIGHT),
            egui::Sense::hover(),
        );
        if is_target {
            let midpoint = rect.center().y;
            ui.painter().hline(
                rect.left()..=rect.right(),
                midpoint,
                egui::Stroke::new(1.0, palette.text_tertiary),
            );
            return response.on_hover_text(format!(
                "{}: the reference the others are read against.",
                self.groups[group_index].name
            ));
        }
        let Some(curve) = self
            .display_curves
            .iter()
            .find(|curve| curve.group_index == group_index)
        else {
            return response;
        };
        let (hz_min, hz_max) = self.zoom.x_range();
        let visible: Vec<&FRPoint> = curve
            .points
            .iter()
            .filter(|point| point.hz >= hz_min && point.hz <= hz_max)
            .collect();
        if visible.len() < 2 {
            return response;
        }
        let (min_db, max_db) = visible
            .iter()
            .map(|point| point.db)
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), db| {
                (min.min(db), max.max(db))
            });
        // A curve exactly flat would otherwise collapse its own scale to zero.
        let span = (max_db - min_db).max(1e-3);
        let map = |point: &FRPoint| {
            let phase = (point.hz.ln() - hz_min.ln()) / (hz_max.ln() - hz_min.ln());
            let height = (point.db - min_db) / span;
            // Drawn in the top four fifths, so the curve does not sit on the
            // row's baseline and look like an underline.
            let y = rect.top() + 0.09 * rect.height() + (1.0 - height) * 0.82 * rect.height();
            egui::pos2(rect.left() + phase * rect.width(), y)
        };
        for pair in visible.windows(2) {
            ui.painter()
                .line_segment([map(pair[0]), map(pair[1])], egui::Stroke::new(1.2, color));
        }
        response.on_hover_text(format!(
            "{} shown from {} Hz to {} Hz",
            curve.label, hz_min as u32, hz_max as u32
        ))
    }

    /// A row's controls: the offset box, and one mark per command.
    ///
    /// Drawn in the order the reference tool draws them, so a row of these reads
    /// the same way wherever it appears. A right-aligned cluster is laid out
    /// right-to-left, which reverses them, and the caller knows which it asked
    /// for.
    fn manager_row_controls(
        &mut self,
        ui: &mut Ui,
        index: usize,
        group: &MeasurementGroup,
        right_to_left: bool,
        removals: &mut Vec<usize>,
    ) {
        use squiglab::ui as cgui;
        let palette = squiglab::theme::Palette::resolve(ui.visuals().dark_mode);
        let pinned = self.pinned.contains(&index);
        let hidden = self.hidden.contains(&index);
        let baseline = self.baseline == Some(index);
        let mut order = RowControl::ORDER.to_vec();
        if right_to_left {
            order.reverse();
        }
        for control in order {
            match control {
                RowControl::Offset => {
                    let offset = self.offsets.entry(index).or_default();
                    if ui
                        .add(egui::DragValue::new(offset).speed(0.1).suffix(" dB"))
                        .changed()
                    {
                        self.dirty = true;
                    }
                }
                RowControl::Smoothing => {
                    if cgui::icon_button_enabled(
                        ui,
                        cgui::Icon::Wave,
                        RowControl::Smoothing.tooltip(),
                        None,
                        !group.target,
                    )
                    .clicked()
                    {
                        self.smooth_level = next_smoothing_level(self.smooth_level);
                        self.dirty = true;
                    }
                }
                RowControl::Hide => {
                    if cgui::icon_button_tinted(
                        ui,
                        if hidden {
                            cgui::Icon::EyeHidden
                        } else {
                            cgui::Icon::Eye
                        },
                        RowControl::Hide.tooltip(),
                        hidden.then_some(palette.accent),
                    )
                    .clicked()
                    {
                        if hidden {
                            self.hidden.remove(&index);
                        } else {
                            self.hidden.insert(index);
                        }
                        self.dirty = true;
                    }
                }
                RowControl::Pin => {
                    if cgui::icon_button_tinted(
                        ui,
                        cgui::Icon::Pin,
                        RowControl::Pin.tooltip(),
                        pinned.then_some(palette.accent),
                    )
                    .clicked()
                    {
                        if pinned {
                            self.pinned.remove(&index);
                        } else {
                            self.pinned.insert(index);
                        }
                    }
                }
                RowControl::Baseline => {
                    if cgui::icon_button_tinted(
                        ui,
                        cgui::Icon::Baseline,
                        RowControl::Baseline.tooltip(),
                        baseline.then_some(palette.accent),
                    )
                    .clicked()
                    {
                        self.baseline = if baseline { None } else { Some(index) };
                        self.dirty = true;
                    }
                }
                RowControl::Download => {
                    if cgui::icon_button(ui, cgui::Icon::Download, RowControl::Download.tooltip())
                        .clicked()
                    {
                        self.export_curve_text(index);
                    }
                }
                RowControl::Remove => {
                    // A pin keeps a curve put, so the mark that removes it goes
                    // dead rather than removing the one the pin was protecting.
                    if cgui::icon_button_enabled(
                        ui,
                        cgui::Icon::Close,
                        if pinned {
                            "Unpin this curve before removing it."
                        } else {
                            RowControl::Remove.tooltip()
                        },
                        None,
                        !pinned,
                    )
                    .clicked()
                    {
                        removals.push(index);
                    }
                }
            }
        }

        // The channel switch, then the curve it is switching. Together, because
        // reading which channel is showing without seeing its shape is how you
        // end up comparing a left ear to a right one.
        if !group.target {
            let mut channel_mode = self.channel_modes.get(&index).copied().unwrap_or_default();
            let mut modes = [
                (ChannelMode::Left, "L"),
                (ChannelMode::Right, "R"),
                (ChannelMode::Both, "Both"),
                (ChannelMode::Average, "Avg"),
            ];
            if right_to_left {
                modes.reverse();
            }
            for (mode, label) in modes {
                if ui
                    .selectable_label(
                        channel_mode == mode,
                        egui::RichText::new(label).size(squiglab::theme::TypeStep::Micro.size()),
                    )
                    .on_hover_text("Which channel of this curve to show.")
                    .clicked()
                    && channel_mode != mode
                {
                    channel_mode = mode;
                    self.channel_modes.insert(index, mode);
                    self.dirty = true;
                }
            }
            self.curve_preview(ui, index, self.curve_color(index));
            let variant_index = self
                .variant_choices
                .get(&index)
                .copied()
                .unwrap_or(0)
                .min(group.variants.len() - 1);
            if group_has_imbalance(&self.curves, &group.variants[variant_index].curve_indices) {
                ui.colored_label(palette.warning, "!").on_hover_text(
                    "Left and right channels differ beyond the imbalance threshold.",
                );
            }
        }
    }

    /// Write one curve's raw measurement out as a `.txt` file.
    ///
    /// The raw measurement, not the processed one. Smoothing, normalisation,
    /// the manual offset and any equaliser are all views of the data, and a
    /// view exported as a measurement is indistinguishable from the
    /// measurement if it ever comes back through this app or another one.
    ///
    /// The channel the row is showing is the one written, because that is the
    /// curve the mark was pressed on, and the file says which it is.
    fn export_curve_text(&mut self, group_index: usize) {
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
                self.curve_export_message = None;
                self.curve_export_error = Some(format!("Unable to export {}: {error}", group.name));
                return;
            }
        };
        let Some((_, points)) = channels
            .into_iter()
            .find(|(name, _)| channel_mode.accepts(name.as_str()))
        else {
            self.curve_export_message = None;
            self.curve_export_error = Some(format!(
                "{} has no {} data to export.",
                group.name,
                channel_mode.label()
            ));
            return;
        };
        let path = download_dir().join(curve_file_name(group, variant, channel_mode));
        let body = curve_text(&points);
        match std::fs::write(&path, body) {
            Ok(()) => {
                self.curve_export_error = None;
                self.curve_export_message =
                    Some(format!("{} exported to {}", group.name, path.display()));
            }
            Err(error) => {
                self.curve_export_message = None;
                self.curve_export_error = Some(format!("Unable to export {path:?}: {error}"));
            }
        }
    }

    /// The deviations the delta bar draws.
    ///
    /// Each measurement is measured against the selected target, which is the
    /// comparison the graph cannot show: on the graph the two sit at absolute
    /// levels and the distance between them depends on the axis. When a rig
    /// correction is selected the measurement is corrected first, and the
    /// uncorrected deviation is kept alongside it, dimmed, so the distance
    /// between the two lines is what the correction actually bought.
    ///
    /// Returns nothing without a target: a deviation from nothing is not a
    /// deviation, and a bar with no curves would only take room from the graph.
    fn delta_bar_deviations(&self) -> Vec<DeviationCurve> {
        let Some(target_group) = self.active_targets.iter().copied().next() else {
            return Vec::new();
        };
        let Some(target) = self
            .display_curves
            .iter()
            .find(|curve| curve.target && curve.group_index == target_group)
        else {
            return Vec::new();
        };
        // Selected corrections compose: each removes its own rig's error, so
        // two of them remove two rigs' errors. A rig with no calibration is
        // not corrected at all, which is what leaving one out means.
        let corrections: Vec<&[FRPoint]> = self
            .active_deltas
            .iter()
            .map(|&index| self.delta_targets[index].points.as_slice())
            .collect();
        let correction_at = |hz: f32| -> f32 {
            corrections
                .iter()
                .filter_map(|curve| processing::interpolate_db(curve, hz))
                .sum()
        };
        let sample = |curve: &[FRPoint]| -> Vec<FRPoint> {
            self.upload_grid
                .iter()
                .map(|&hz| FRPoint {
                    hz,
                    // The grid is shared with the database, so a curve that
                    // starts or ends outside it is held at its edge rather
                    // than dropped, which would notch the bar at both ends.
                    db: processing::interpolate_db(curve, hz).unwrap_or(0.0),
                })
                .collect()
        };
        let target_points = sample(&target.points);

        let mut deviations = Vec::new();
        for curve in &self.display_curves {
            if curve.target || curve.source != CurveSource::Measurement {
                continue;
            }
            let measured = sample(&curve.points);
            let corrected: Vec<FRPoint> = measured
                .iter()
                .zip(&target_points)
                .map(|(measured, target)| FRPoint {
                    hz: measured.hz,
                    db: measured.db - target.db - correction_at(measured.hz),
                })
                .collect();
            let color = self.display_curve_color(curve);
            if corrections.is_empty() {
                deviations.push(DeviationCurve {
                    label: curve.label.clone(),
                    points: corrected,
                    color,
                    dashed: false,
                });
            } else {
                deviations.push(DeviationCurve {
                    label: format!("{} (rig-corrected)", curve.label),
                    points: corrected,
                    color,
                    dashed: false,
                });
                deviations.push(DeviationCurve {
                    label: format!("{} (uncorrected)", curve.label),
                    points: measured
                        .iter()
                        .zip(&target_points)
                        .map(|(measured, target)| FRPoint {
                            hz: measured.hz,
                            db: measured.db - target.db,
                        })
                        .collect(),
                    color: Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 90),
                    dashed: false,
                });
            }
        }

        // The corrections themselves, dashed: the bar shows what they are doing
        // to the measurements, and this is the curve doing it.
        for &index in &self.active_deltas {
            let correction = &self.delta_targets[index];
            deviations.push(DeviationCurve {
                label: correction.name.clone(),
                points: correction.points.clone(),
                color: squiglab::theme::Palette::resolve(true).text_secondary,
                dashed: true,
            });
        }
        deviations
    }

    /// What the delta bar's deviations are measured against.
    fn delta_bar_caption(&self) -> String {
        let mut caption = String::new();
        if let Some(index) = self.active_targets.iter().copied().next() {
            caption = format!("deviation from {}", self.groups[index].name);
        }
        let selected: Vec<&str> = self
            .active_deltas
            .iter()
            .map(|&index| self.delta_targets[index].name.as_str())
            .collect();
        if !selected.is_empty() {
            caption.push_str(" \u{b7} rig-corrected by ");
            caption.push_str(&selected.join(", "));
        }
        caption
    }

    /// The deviation window.
    ///
    /// A window rather than a panel, because this is not a view of the selection
    /// so much as a question about it. The manager's job is to say what is in
    /// play; this asks how far it is from the target, which is a different act
    /// with a different centre of attention. It also wants its own proportions:
    /// a strip squeezed under the graph shares the window with the curves it is
    /// about, and a window can be moved aside and looked at next to them.
    ///
    /// With nothing selected it says so instead of opening an empty plot, so
    /// the window is never an explanation-free void.
    fn delta_window(&mut self, ctx: &Context) {
        if !self.show_delta {
            return;
        }
        let mut open = self.show_delta;
        let caption = self.delta_bar_caption();
        let deviations = self.delta_bar_deviations();
        let bar_curves: Vec<PlotCurve<'_>> = deviations
            .iter()
            .map(|deviation| PlotCurve {
                label: &deviation.label,
                label_key: &deviation.label,
                points: &deviation.points,
                color: deviation.color,
                dashed: deviation.dashed,
                target: true,
            })
            .collect();
        let (productive, notice) = if bar_curves.is_empty() {
            (false, self.delta_bar_caption().is_empty())
        } else {
            (true, false)
        };
        egui::Window::new("Deviation from target")
            .open(&mut open)
            .default_size(egui::vec2(760.0, DeltaBar::HEIGHT + 96.0))
            .show(ctx, |ui| {
                if productive {
                    DeltaBar {
                        zoom: self.zoom,
                        caption,
                    }
                    .show(ui, &bar_curves);
                    return;
                }
                // Nothing to plot, so explain what is missing rather than
                // showing an empty plot the user has to interpret.
                let message = if notice {
                    "Select a target to measure against."
                } else {
                    "Select a target and at least one measurement."
                };
                ui.label(RichText::new(message).size(squiglab::theme::TypeStep::Body.size()));
            });
        self.show_delta = open;
    }

    /// The strip of targets between the graph and the manager.
    ///
    /// What is being read against is a different question from how far the
    /// headphones are from it, and on the reference tool the two get their own
    /// bands for that reason. The strip answers the first: which reference
    /// targets, and which rig corrections. The manager below it answers the
    /// second, one row per curve.
    ///
    /// Both lists scroll sideways rather than wrapping, so the strip stays one
    /// row however many targets are selected, and a wide one costs nothing.
    /// The status line, at the very bottom.
    ///
    /// A bottom panel like the rest, so it is laid out with them and not after
    /// the central one. Drawn last in that group, which puts it under the strip
    /// and the manager: the manager's rows are the selection you are working
    /// with, and the status line is about the app rather than about them.
    fn status_panel(&mut self, ui: &mut Ui) {
        egui::Panel::bottom("status")
            .resizable(false)
            .default_size(24.0)
            .show(ui, |ui| {
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
                        || self.curve_export_error.is_some()
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
                if let Some(error) = &self.curve_export_error {
                    ui.colored_label(Color32::LIGHT_RED, error);
                }
                if let Some(message) = &self.curve_export_message {
                    ui.label(message);
                }
            });
    }

    /// The rig corrections, as chips.
    ///
    /// Built out of state rather than read from it as the loop draws, because
    /// toggling one needs the borrow the drawing loop is holding. Each carries
    /// its own placeholder warning: a chip is the only place a rig correction
    /// can say it corrects nothing, and a chip nobody distrusted is worse than
    /// no chip at all.
    fn delta_chips(&self) -> Vec<TargetChip> {
        self.delta_targets
            .iter()
            .enumerate()
            .map(|(index, target)| TargetChip {
                index,
                label: format!("\u{394} {}", target.name),
                selected: self.active_deltas.contains(&index),
                tooltip: if target.is_placeholder() {
                    format!(
                        "{}\n\nBuilt from placeholder calibrations, not measurements. It corrects nothing.",
                        target.note.as_deref().unwrap_or("This rig correction.")
                    )
                } else {
                    target.note.clone().unwrap_or_default()
                },
            })
            .collect()
    }

    /// The reference targets, as chips.
    fn target_chips(&self) -> Vec<TargetChip> {
        self.groups
            .iter()
            .enumerate()
            .filter(|(_, group)| group.target)
            .map(|(index, group)| TargetChip {
                index,
                label: group.name.clone(),
                selected: self.active_targets.contains(&index),
                tooltip: group.brand.clone(),
            })
            .collect()
    }

    /// The strip of targets between the graph and the manager.
    ///
    /// What is being read against is a different question from how far the
    /// headphones are from it, and on the reference tool the two get their own
    /// bands for that reason. The strip answers the first — which reference
    /// targets, and which rig corrections. The manager below it answers the
    /// second, one row per curve.
    ///
    /// Both lists scroll sideways rather than wrapping, so the strip stays one
    /// row however many targets are in play, and a wide one costs nothing.
    fn target_strip(&mut self, ui: &mut Ui) {
        use squiglab::ui as cgui;
        let palette = squiglab::theme::Palette::resolve(ui.visuals().dark_mode);
        let delta_open = self.show_delta;
        egui::Panel::bottom("target_strip")
            .resizable(false)
            .exact_size(TARGET_STRIP_HEIGHT)
            .frame(
                egui::Frame::default()
                    .fill(palette.surface_panel)
                    .inner_margin(egui::Margin::symmetric(cgui::Spacing::COMPACT as i8, 0)),
            )
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = cgui::Spacing::COMPACT;

                    // The deviation window's switch, here rather than in the
                    // toolbar because it is a question about the targets named
                    // beside it.
                    let glyph_color = if delta_open {
                        palette.on_accent
                    } else {
                        palette.text_secondary
                    };
                    if cgui::pill(
                        ui,
                        "DELTA",
                        delta_open,
                        "Open the window showing how far each curve is from the target.",
                    )
                    .clicked()
                    {
                        self.show_delta = !self.show_delta;
                    }
                    ui.label(
                        egui::RichText::new("\u{25b2}")
                            .font(squiglab::theme::font(squiglab::theme::TypeStep::Caption))
                            .color(glyph_color),
                    );

                    // Rig corrections. Capped, because the reference targets
                    // need the rest of the strip: a long list of corrections
                    // that took everything would leave the targets — the thing
                    // the readings are measured against — with nowhere to be.
                    if !self.delta_targets.is_empty() {
                        let cap = ui.available_width() * 0.45;
                        egui::ScrollArea::horizontal()
                            .id_salt("delta_chips")
                            .auto_shrink([true, false])
                            .max_width(cap)
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.x = cgui::Spacing::COMPACT;
                                for chip in self.delta_chips() {
                                    if cgui::pill(ui, &chip.label, chip.selected, chip.tooltip)
                                        .clicked()
                                    {
                                        self.toggle_delta(chip.index);
                                    }
                                }
                            });
                    }
                    if let Some(error) = &self.delta_error {
                        ui.colored_label(Color32::LIGHT_RED, error);
                    }

                    cgui::cluster_rule(ui);

                    // A target is what everything else is read against, so it
                    // says so rather than sharing a heading with the
                    // corrections.
                    cgui::cluster_label(ui, "Reference targets");
                    // Scrolled sideways, never wrapped. There are more targets
                    // than any window is wide, and a target that ran off the
                    // edge would be one the user cannot select — which reads as
                    // the target not existing.
                    egui::ScrollArea::horizontal()
                        .id_salt("target_chips")
                        .auto_shrink([true, false])
                        .max_width(ui.available_width())
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.x = cgui::Spacing::COMPACT;
                            for chip in self.target_chips() {
                                if cgui::pill(ui, &chip.label, chip.selected, chip.tooltip)
                                    .clicked()
                                {
                                    self.toggle_curve(chip.index);
                                }
                            }
                        });
                });
            });
    }

    fn central_panel(&mut self, ui: &mut Ui) {
        egui::CentralPanel::default().show(ui, |ui| {
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
                    color: self.display_curve_color(curve),
                    dashed: curve.target,
                    target: curve.target,
                })
                .collect();
            let interaction = GraphCanvas {
                zoom: self.zoom,
                smooth_level: self.smooth_level,
                show_labels: self.show_labels,
                inspect_mode: self.inspect_mode,
                label_positions: &self.label_positions,
                label_layout: self.label_layout,
                tutorial_band: self.tutorial_segment.map(|index| {
                    let segment = &TUTORIAL_SEGMENTS[index];
                    (segment.title, segment.range_hz)
                }),
                bands: &FREQUENCY_BANDS,
                rig_description: self.rig_description.as_deref(),
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
                color: self.display_curve_color(curve),
                dashed: curve.target,
                target: curve.target,
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
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        #[cfg(debug_assertions)]
        ctx.set_debug_on_hover(self.debug_hooks);
        squiglab::theme::apply(&ctx, self.dark_mode);
        if self.screenshot_pending {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
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
        self.toolbar(ui);
        self.delta_window(&ctx);
        self.share_window(&ctx);
        self.equalizer_import_window(&ctx);
        self.upload_window(&ctx);
        self.onboarding_window(&ctx);
        self.tutorial_window(&ctx);
        // The reference tool has two viewports: below ~1024px content width the
        // graph stacks above the tab column, and above it the tabs sit beside the
        // graph. `selector_panel` branches on available width to pick between
        // them, which is still the full width here because nothing laid out
        // above this point narrows it.
        //
        // Every side, top and bottom panel is laid out before the central one,
        // and that order is not cosmetic. A `CentralPanel` uses up the rect it
        // was given, so a bottom panel shown after it is measured against a rect
        // with no height left and is clamped to zero: it would draw nothing at
        // all, with the space below the graph left empty.
        //
        // The manager goes ahead of the library, not merely ahead of the graph,
        // so that it spans the full width and the library sits beside the graph
        // above it. The other way round, the library would own the full height
        // and the manager would be squeezed into the column beside it.
        self.status_panel(ui);
        self.manager_panel(ui);
        self.target_strip(ui);
        self.selector_panel(ui);
        self.central_panel(ui);
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
    }
}

/// What rendering one brand section of the library found: the checkboxes the
/// user toggled, and the rect of the first selected row if the brand holds one.
#[derive(Default)]
struct LibrarySection {
    toggles: Vec<usize>,
    /// Delta targets toggled this frame, held apart from `toggles` because an
    /// index into the groups is not an index into the delta targets.
    delta_toggles: Vec<usize>,
    reveal_rect: Option<egui::Rect>,
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

/// Label for an EQ curve. The reference tool names it "<phone> EQ"; deriving
/// it from the parent curve's label keeps the pair readable whatever the
/// model's variant is named, and names it per channel when the parent shows
/// more than one.
fn eq_curve_label(parent_label: &str) -> String {
    format!("{parent_label} EQ")
}

/// Whether stored equalizer rows are usable: a bounded number of finite rows
/// with a frequency, Q, and gain. Anything else is refused rather than
/// partly trusted, so a corrupt preferences file cannot poison the graph.
fn eq_rows_are_valid(rows: &[EqRow]) -> bool {
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

/// Whether a row's filter would contribute nothing, the same condition the
/// DSP uses to drop inert filters before calculating.
fn eq_row_is_inert(row: &EqRow) -> bool {
    let filter = row.filter;
    filter.freq == 0.0 || filter.q == 0.0 || filter.gain == 0.0
}

fn curve_label(model: &str, variant: &str, channel_name: String) -> String {
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

/// The brand a measurement is listed under, including the brand's suffix when
/// it has one.
fn brand_name(group: &MeasurementGroup) -> String {
    group
        .brand_suffix
        .as_ref()
        .map(|suffix| format!("{} {suffix}", group.brand))
        .unwrap_or_else(|| group.brand.clone())
}

/// Write an export beside the user's pictures, where the PNG and SVG exports
/// go, under the file name the reference format specifies.
fn save_export(file_name: &str, contents: &str) -> anyhow::Result<PathBuf> {
    let directory = dirs::picture_dir().ok_or_else(|| {
        anyhow::anyhow!("the operating system did not provide a pictures directory")
    })?;
    std::fs::create_dir_all(&directory)?;
    let path = directory.join(file_name);
    std::fs::write(&path, contents)?;
    Ok(path)
}

fn group_key(group: &MeasurementGroup) -> String {
    format!("{}\u{1f}{}", group.brand, group.name)
}

/// Root of the application's configuration directory.
///
/// Honours `SQUIGLAB_CONFIG_DIR` when set so tests can point the application at a
/// scratch directory. Without this, tests that load measurements read whichever
/// preferences file happens to exist on the machine running them, so their
/// result depends on the developer's own app state rather than the test.
fn config_root() -> anyhow::Result<PathBuf> {
    if let Some(override_directory) = std::env::var_os("SQUIGLAB_CONFIG_DIR") {
        if !override_directory.is_empty() {
            return Ok(PathBuf::from(override_directory));
        }
    }
    dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("the operating system did not provide a config directory"))
}

fn preferences_path() -> anyhow::Result<PathBuf> {
    let config_dir = config_root()?;
    Ok(config_dir
        .join(CONFIG_DIRECTORY_NAME)
        .join("preferences.json"))
}

fn app_config_path() -> anyhow::Result<PathBuf> {
    let config_dir = config_root()?;
    Ok(config_dir.join(CONFIG_DIRECTORY_NAME).join("config.toml"))
}

/// Config directories used before the rename, newest first. Preferences are
/// only read from these; writes always go to the current location, so the first
/// successful launch migrates a returning user's settings forward.
const LEGACY_CONFIG_DIRECTORIES: [&str; 2] = ["cringraph", "CrinGraph-native"];

fn legacy_preferences_paths() -> anyhow::Result<Vec<PathBuf>> {
    let config_dir = config_root()?;
    Ok(LEGACY_CONFIG_DIRECTORIES
        .iter()
        .map(|directory| config_dir.join(directory).join("preferences.json"))
        .collect())
}

fn read_app_config_from(path: &Path) -> anyhow::Result<AppConfig> {
    match std::fs::read_to_string(path) {
        Ok(contents) => Ok(toml::from_str(&contents)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(AppConfig::default()),
        Err(error) => Err(error.into()),
    }
}

fn initialize_app_config_from(path: &Path) -> anyhow::Result<AppConfig> {
    match std::fs::read_to_string(path) {
        Ok(contents) => Ok(toml::from_str(&contents)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let config = default_app_config()?;
            let directory = path
                .parent()
                .ok_or_else(|| anyhow::anyhow!("configuration path has no parent directory"))?;
            std::fs::create_dir_all(directory)?;
            let contents = toml::to_string_pretty(&config)?;
            let mut file = match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
            {
                Ok(file) => file,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    return read_app_config_from(path);
                }
                Err(error) => return Err(error.into()),
            };
            use std::io::Write;
            file.write_all(contents.as_bytes())?;
            file.sync_all()?;
            Ok(config)
        }
        Err(error) => Err(error.into()),
    }
}

fn default_app_config() -> anyhow::Result<AppConfig> {
    let working_directory = std::env::current_dir()?;
    let headphone_database_dir = working_directory.join("data_hp");
    let delta_dir = working_directory.join("data_deltas");
    Ok(AppConfig {
        measurements: MeasurementConfig {
            database_dir: Some(working_directory.join("data")),
            headphone_database_dir: headphone_database_dir
                .is_dir()
                .then_some(headphone_database_dir),
            delta_dir: delta_dir.is_dir().then_some(delta_dir),
        },
        defaults: DefaultConfig::default(),
        rig_description: None,
    })
}

fn validate_app_config(config: &AppConfig) -> anyhow::Result<()> {
    if config.defaults.smoothing_octaves > MAX_SMOOTH_LEVEL {
        anyhow::bail!("configured smoothing_octaves must be between 0 and {MAX_SMOOTH_LEVEL}");
    }
    Ok(())
}

fn resolve_measurement_dir(
    configured: Option<&Path>,
    fallback: &Path,
    config_path: &Path,
) -> PathBuf {
    let Some(path) = configured else {
        return fallback.to_path_buf();
    };
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        config_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(path)
    }
}

fn read_preferences() -> anyhow::Result<Option<Preferences>> {
    if let Some(preferences) = read_preferences_from(&preferences_path()?)? {
        return Ok(Some(preferences));
    }
    for path in legacy_preferences_paths()? {
        if let Some(preferences) = read_preferences_from(&path)? {
            return Ok(Some(preferences));
        }
    }
    Ok(None)
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
        .or_else(|| uri.strip_prefix(LEGACY_SHARE_URI_PREFIX))
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
    let path = directory.join(format!("squiglab-{timestamp}.png"));
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
    let path = directory.join(format!("squiglab-{timestamp}.svg"));
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
            point.db -= processing::interpolate_db(&baseline, point.hz)
                .ok_or_else(|| anyhow::anyhow!("unable to sample the baseline curve"))?;
        }
    }
    Ok(())
}

fn target_category(target: &str) -> &'static str {
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

/// Where an exported curve goes: the download directory, because it is data the
/// user asked for rather than something the app produced on its own.
fn download_dir() -> PathBuf {
    dirs::download_dir().unwrap_or_else(std::env::temp_dir)
}

/// One curve's raw points as text.
///
/// Comma-separated, so a spreadsheet opens it directly. A header line would
/// break every naive parser that reads it, so there is none.
fn curve_text(points: &[FRPoint]) -> String {
    points
        .iter()
        .map(|point| format!("{:.6},{:.6}\n", point.hz, point.db))
        .collect()
}

/// What to call an exported curve: the curve, the variant, and the channel,
/// because two rows can differ on any of the three.
fn curve_file_name(
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
        initialize_app_config_from, manager_heights, matches_search, prepare_channels,
        read_app_config_from, read_preferences_from, resolve_measurement_dir, target_category,
        validate_app_config, write_preferences_to, AppConfig, ChannelMode, CrinGraphApp,
        CurveSource, DisplayCurve, MeasurementGroup, MeasurementVariant, Preferences, SavedGroup,
        MAX_EQ_ROWS, MAX_SHARE_URI_LENGTH, MAX_SMOOTH_LEVEL, TUTORIAL_SEGMENTS,
    };
    use squiglab::data::{CurveInfo, FRPoint};
    use squiglab::delta::DeltaTarget;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    /// Serialises tests that redirect the config directory, since the override is
    /// process-global and tests run on parallel threads.
    static CONFIG_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Run `body` with the application pointed at a scratch config directory, so
    /// a test is not reading whichever preferences happen to exist on the machine
    /// running it. Restores the previous setting afterwards.
    fn with_isolated_config<T>(body: impl FnOnce() -> T) -> T {
        let _guard = CONFIG_DIR_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let directory =
            std::env::temp_dir().join(format!("squiglab-test-config-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("create scratch config directory");
        let previous = std::env::var_os("SQUIGLAB_CONFIG_DIR");
        std::env::set_var("SQUIGLAB_CONFIG_DIR", &directory);
        let result = body();
        match previous {
            Some(value) => std::env::set_var("SQUIGLAB_CONFIG_DIR", value),
            None => std::env::remove_var("SQUIGLAB_CONFIG_DIR"),
        }
        let _ = std::fs::remove_dir_all(&directory);
        result
    }
    use squiglab::graph::ZoomRegion;
    use std::path::Path;

    #[test]
    fn default_smoothing_matches_the_original_graph_setting() {
        assert_eq!(Preferences::default().smooth_level, 5);
        assert_eq!(CrinGraphApp::default().smooth_level, 5);
    }

    #[test]
    fn curve_labels_do_not_repeat_the_model_name() {
        assert_eq!(super::curve_label("GAR5", "GarG5", String::new()), "GarG5");
        assert_eq!(
            super::curve_label("GAR5", "GarG5", "R".to_owned()),
            "GarG5 (R)"
        );
        // A model whose single variant carries its own name is not echoed twice.
        assert_eq!(super::curve_label("GAR5", "GAR5", String::new()), "GAR5");
        assert_eq!(
            super::curve_label("HTH67", "HTH67", "R".to_owned()),
            "HTH67 (R)"
        );
        // With no variant named, the model name stands on its own.
        assert_eq!(
            super::curve_label("SoftCheeks", "", "Avg".to_owned()),
            "SoftCheeks (Avg)"
        );
    }

    #[test]
    fn windows_safe_ief_harman_filename_retains_target_category() {
        assert_eq!(target_category("IEF Harman"), "Preference targets");
    }

    #[test]
    fn app_config_sets_measurement_directories_and_default_smoothing() {
        let directory = std::env::temp_dir().join(format!(
            "squiglab-config-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("config.toml");
        std::fs::write(
            &path,
            r#"
[measurements]
database_dir = "measurements"
headphone_database_dir = "headphones"

[defaults]
smoothing_octaves = 3
"#,
        )
        .unwrap();

        let config = read_app_config_from(&path).unwrap();
        assert_eq!(config.defaults.smoothing_octaves, 3);
        assert_eq!(
            resolve_measurement_dir(
                config.measurements.database_dir.as_deref(),
                Path::new("data"),
                &path
            ),
            directory.join("measurements")
        );
        assert_eq!(
            resolve_measurement_dir(
                config.measurements.headphone_database_dir.as_deref(),
                Path::new("data_hp"),
                &path
            ),
            directory.join("headphones")
        );
        let absolute_path = directory.join("external-headphones");
        assert_eq!(
            resolve_measurement_dir(Some(&absolute_path), Path::new("data_hp"), &path),
            absolute_path
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn platform_config_examples_are_valid_toml_schemas() {
        let examples = [
            "config-examples/windows/config.toml",
            "config-examples/linux/config.toml",
            "config-examples/macos/config.toml",
        ];
        for example in examples {
            let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(example);
            let config = read_app_config_from(&path).unwrap();
            validate_app_config(&config).unwrap();
            assert!(config.measurements.database_dir.is_some());
            assert!(config.measurements.headphone_database_dir.is_some());
            assert_eq!(config.defaults.smoothing_octaves, 5);
        }
    }

    #[test]
    fn missing_app_config_uses_defaults_and_out_of_range_smoothing_is_rejected() {
        let directory = std::env::temp_dir().join(format!(
            "squiglab-config-default-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("missing.toml");
        let config = read_app_config_from(&path).unwrap();
        assert_eq!(config, AppConfig::default());
        assert_eq!(config.defaults.smoothing_octaves, 5);

        std::fs::write(
            &path,
            format!("[defaults]\nsmoothing_octaves = {}\n", MAX_SMOOTH_LEVEL + 1),
        )
        .unwrap();
        let config = read_app_config_from(&path).unwrap();
        assert!(config.defaults.smoothing_octaves > MAX_SMOOTH_LEVEL);
        assert!(validate_app_config(&config).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn first_run_creates_config_with_sample_paths_and_preserves_existing_config() {
        let directory = std::env::temp_dir().join(format!(
            "squiglab-config-create-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = directory.join("config.toml");
        let config = initialize_app_config_from(&path).unwrap();
        let generated_contents = std::fs::read_to_string(&path).unwrap();
        let current_directory = std::env::current_dir().unwrap();
        assert_eq!(
            config.measurements.database_dir,
            Some(current_directory.join("data"))
        );
        assert!(generated_contents.contains("database_dir"));
        assert!(generated_contents.contains("smoothing_octaves = 5"));

        let second_read = initialize_app_config_from(&path).unwrap();
        assert_eq!(second_read, config);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), generated_contents);

        let user_config = "[defaults]\nsmoothing_octaves = 2\n";
        std::fs::write(&path, user_config).unwrap();
        let user_config_parsed = initialize_app_config_from(&path).unwrap();
        assert_eq!(user_config_parsed.defaults.smoothing_octaves, 2);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), user_config);
        std::fs::remove_dir_all(directory).unwrap();
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
    fn an_equalizer_becomes_a_child_curve_of_its_measurement() {
        let curves = vec![
            test_curve("Model", "L", 50.0),
            test_curve("Model", "R", 50.0),
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
        app.eq_filters.insert(
            0,
            vec![super::EqRow {
                filter: squiglab::equalizer::Filter::new(
                    squiglab::equalizer::FilterType::Peaking,
                    500.0,
                    1.2,
                    -3.5,
                ),
                enabled: true,
            }],
        );
        app.rebuild_display_curves();
        assert!(app.processing_error.is_none());
        // One EQ child per channel beside the parent's channel.
        let eq_children: Vec<_> = app
            .display_curves
            .iter()
            .filter(|curve| curve.source == CurveSource::Equalizer)
            .collect();
        assert_eq!(eq_children.len(), 2);
        assert_eq!(eq_children[0].label, "Default (L) EQ");
        assert_eq!(eq_children[1].label, "Default (R) EQ");
        // The child is back-linked to the parent group so the renderer can
        // resolve its color and normalization from it.
        assert!(eq_children.iter().all(|child| child.group_index == 0));
        assert!(eq_children.iter().all(|child| !child.target));
        // The parent's normalization propagates: the child is the parent's
        // curve plus the filter's own response. The -3.5 dB cut is deepest at
        // the filter's 500 Hz centre and vanishes away from it.
        for child in &eq_children {
            let parent = app
                .display_curves
                .iter()
                .find(|curve| {
                    child.label.starts_with(&curve.label) && curve.label.len() < child.label.len()
                })
                .expect("the child names its parent");
            assert_eq!(parent.source, CurveSource::Measurement);
            let deepest = parent
                .points
                .iter()
                .zip(child.points.iter())
                .map(|(parent_point, child_point)| {
                    assert_eq!(parent_point.hz, child_point.hz);
                    parent_point.db - child_point.db
                })
                .fold(0.0_f32, f32::max);
            assert!(
                (deepest - 3.5).abs() < 0.05,
                "the deepest cut was {deepest:.2} dB, not 3.5"
            );
            // Two octaves away, at either end of the curve, the cut is only a
            // fraction of a decibel: a Q of 1.2 is a wide filter, not a notch.
            for (parent_point, child_point) in [
                parent.points.first().unwrap(),
                parent.points.last().unwrap(),
            ]
            .into_iter()
            .zip([child.points.first().unwrap(), child.points.last().unwrap()])
            {
                let away = parent_point.db - child_point.db;
                assert!(
                    away.abs() < 0.5,
                    "the cut reached {} Hz by {away:.2} dB",
                    parent_point.hz
                );
            }
        }
        // The two curves do not share a color.
        let parent = app
            .display_curves
            .iter()
            .find(|curve| curve.source == CurveSource::Measurement)
            .unwrap();
        assert_ne!(
            app.display_curve_color(parent),
            app.display_curve_color(eq_children[0])
        );
    }

    #[test]
    fn a_disabled_or_inert_filter_leaves_the_curve_alone() {
        let curves = vec![test_curve("Model", "L", 50.0)];
        let group = super::MeasurementGroup {
            brand: "Brand".to_owned(),
            brand_suffix: None,
            name: "Model".to_owned(),
            variants: vec![super::MeasurementVariant {
                name: "Default".to_owned(),
                curve_indices: vec![0],
            }],
            target: false,
        };
        let mut app = CrinGraphApp {
            curves,
            groups: vec![group],
            active: [0].into_iter().collect(),
            ..CrinGraphApp::default()
        };
        let row = |enabled: bool, gain: f64| super::EqRow {
            filter: squiglab::equalizer::Filter::new(
                squiglab::equalizer::FilterType::Peaking,
                500.0,
                1.2,
                gain,
            ),
            enabled,
        };
        app.eq_filters.insert(0, vec![row(false, -3.5)]);
        app.rebuild_display_curves();
        assert!(app
            .display_curves
            .iter()
            .all(|curve| curve.source == CurveSource::Measurement));
        // A zero gain is inert: it would not be drawn, so no child curve.
        app.eq_filters.insert(0, vec![row(true, 0.0)]);
        app.rebuild_display_curves();
        assert!(app
            .display_curves
            .iter()
            .all(|curve| curve.source == CurveSource::Measurement));
    }

    #[test]
    fn equalizers_round_trip_through_preferences_and_share_links() {
        with_isolated_config(|| {
            let mut app = CrinGraphApp::default();
            app.load_measurements().unwrap();
            let key = super::group_key(&app.groups[app.active.iter().next().copied().unwrap()]);
            let rows = vec![super::EqRow {
                filter: squiglab::equalizer::Filter::new(
                    squiglab::equalizer::FilterType::LowShelf,
                    105.0,
                    0.7,
                    6.0,
                ),
                enabled: true,
            }];
            app.eq_filters
                .insert(app.active.iter().next().copied().unwrap(), rows.clone());
            let preferences = app.preferences();
            assert_eq!(preferences.eq_filters.get(&key), Some(&rows));

            let mut restored = CrinGraphApp {
                curves: app.curves.clone(),
                groups: app.groups.clone(),
                ..CrinGraphApp::default()
            };
            assert!(restored.apply_preferences(preferences));
            assert_eq!(restored.eq_filters.values().next(), Some(&rows));
            restored.dirty = true;
            restored.rebuild_display_curves();
            assert!(restored
                .display_curves
                .iter()
                .any(|curve| curve.source == CurveSource::Equalizer));
        });
    }

    #[test]
    fn an_invalid_equalizer_is_refused_rather_than_drawn() {
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
            ..CrinGraphApp::default()
        };
        let key = super::group_key(&app.groups[0]);
        let row = |freq: f64| super::EqRow {
            filter: squiglab::equalizer::Filter::new(
                squiglab::equalizer::FilterType::Peaking,
                freq,
                1.0,
                -3.0,
            ),
            enabled: true,
        };
        for rows in [
            vec![],
            vec![row(f64::NAN)],
            vec![row(-10.0)],
            vec![row(30_000.0)],
            vec![row(500.0); MAX_EQ_ROWS + 1],
        ] {
            let preferences = Preferences {
                eq_filters: [(key.clone(), rows.clone())].into_iter().collect(),
                ..Preferences::default()
            };
            assert!(!app.apply_preferences(preferences), "{rows:?}");
        }
        // A valid equalizer is accepted.
        let preferences = Preferences {
            eq_filters: [(key, vec![row(500.0)])].into_iter().collect(),
            ..Preferences::default()
        };
        assert!(app.apply_preferences(preferences));
        assert_eq!(app.eq_filters.get(&0).map(Vec::len), Some(1));
    }

    fn group_named(app: &CrinGraphApp, name: &str) -> usize {
        app.groups
            .iter()
            .position(|group| group.name == name)
            .unwrap_or_else(|| panic!("no group named {name}"))
    }

    #[test]
    fn the_equalizer_panel_resolves_the_curve_and_target_it_edits() {
        with_isolated_config(|| {
            let mut app = CrinGraphApp::default();
            app.load_measurements().unwrap();
            // With one measurement active and no target, the panel edits it
            // and falls back to the reference's second-measurement rule for
            // the target, which does not exist yet.
            let phone = group_named(&app, "HTH67");
            app.active = [phone].into_iter().collect();
            app.active_targets.clear();
            assert_eq!(app.eq_phone(), Some(phone));
            assert_eq!(app.eq_target(), None);

            // A second active measurement becomes the fallback target, in the
            // same display order the groups are listed in.
            let other = app
                .groups
                .iter()
                .enumerate()
                .map(|(index, _)| index)
                .filter(|&index| index != phone && !app.groups[index].target)
                .nth(1)
                .expect("more than one measurement to fall back to");
            app.active.insert(other);
            let mut active_in_order: Vec<_> = app.active.iter().copied().collect();
            active_in_order.sort_unstable();
            assert_eq!(app.eq_target(), active_in_order.get(1).copied());

            // An active target wins over the measurement fallback, and the
            // first active target wins over an unset choice.
            let target = group_named(&app, "Harman");
            app.active_targets = [target].into_iter().collect();
            assert_eq!(app.eq_target(), Some(target));
            app.eq_target = None;
            assert_eq!(app.eq_target(), Some(target));

            // An explicit choice is honoured while the curve stays active, and
            // forgotten when the curve is switched off.
            app.eq_phone = Some(other);
            assert_eq!(app.eq_phone(), Some(other));
            app.active = [phone].into_iter().collect();
            assert_eq!(app.eq_phone(), Some(phone));
        });
    }

    #[test]
    fn auto_eq_moves_a_measurement_towards_its_target() {
        with_isolated_config(|| {
            let mut app = CrinGraphApp::default();
            app.load_measurements().unwrap();
            let phone = group_named(&app, "HTH67");
            let target = group_named(&app, "Harman");
            app.active = [phone].into_iter().collect();
            app.active_targets = [target].into_iter().collect();
            let before = app.normalized_average_points(phone).unwrap();
            let target_points = app.normalized_average_points(target).unwrap();
            let distance = |curve: &[FRPoint]| {
                let mut total = 0.0;
                for (point, goal) in curve.iter().zip(&target_points) {
                    total += (point.db - goal.db).abs();
                }
                total / curve.len() as f32
            };
            let distance_before = distance(&before);

            app.run_auto_eq();
            assert!(app.eq_export_error.is_none(), "{:?}", app.eq_export_error);
            let rows = app.eq_filters.get(&phone).expect("Auto EQ fitted filters");
            assert!(!rows.is_empty());
            assert!(rows.iter().all(|row| row.enabled));
            assert!(rows.len() <= super::DEFAULT_EQ_MAX_FILTERS);

            // The fitted filters are what the graph and the exports use.
            app.rebuild_display_curves();
            let equalized: Vec<FRPoint> = app
                .display_curves
                .iter()
                .find(|curve| curve.source == CurveSource::Equalizer && curve.group_index == phone)
                .expect("the equalizer drew a child curve")
                .points
                .clone();
            assert!(
                distance(&equalized) < distance_before,
                "Auto EQ left the curve {} dB from the target, was {}",
                distance(&equalized),
                distance_before
            );
            // AutoEQ strips the ports to the optimizer's ranges, which the
            // panel shows without further rounding.
            let filters = app.enabled_filters(phone);
            assert!(filters.iter().all(|filter| {
                (20.0..=15_000.0).contains(&filter.freq)
                    && (0.5..=2.0).contains(&filter.q)
                    && (-12.0..=12.0).contains(&filter.gain)
            }));
        });
    }

    #[test]
    fn the_equalizer_exports_name_the_curve_and_agree_on_its_filters() {
        with_isolated_config(|| {
            let mut app = CrinGraphApp::default();
            app.load_measurements().unwrap();
            let phone = group_named(&app, "HTH67");
            app.active = [phone].into_iter().collect();
            // Give the curve the default bands the panel would have installed.
            app.eq_filters.insert(phone, super::default_eq_filters());

            let (name, parametric) = app.parametric_export().expect("a curve is being edited");
            assert_eq!(name, "HTH67 Filters.txt");
            assert!(parametric.starts_with("Preamp: "));
            assert!(parametric.contains("Filter 1: ON LSC "));
            // The preamp keeps the exported curve from boosting.
            let preamp: f64 = parametric
                .strip_prefix("Preamp: ")
                .and_then(|rest| rest.split(' ').next())
                .and_then(|value| value.parse().ok())
                .expect("the preamp is a number");
            assert!(preamp <= 0.0, "a preamp above zero would boost");

            let (name, graphic) = app.graphic_eq_export().expect("a curve is being edited");
            assert_eq!(name, "HTH67 Wavelet.txt");
            assert!(graphic.starts_with("GraphicEQ: 20 "));

            // Both exports describe the same enabled filters: importing the
            // parametric text back gives the same set the panel holds.
            let imported = squiglab::equalizer::import_parametric(&parametric).unwrap();
            let enabled: Vec<_> = imported
                .filters
                .iter()
                .zip(&imported.enabled)
                .filter(|(_, &enabled)| enabled)
                .map(|(filter, _)| *filter)
                .collect();
            assert_eq!(enabled, app.enabled_filters(phone));
        });
    }

    #[test]
    fn importing_an_apo_file_replaces_the_curves_filters() {
        with_isolated_config(|| {
            let mut app = CrinGraphApp::default();
            app.load_measurements().unwrap();
            let phone = group_named(&app, "HTH67");
            app.active = [phone].into_iter().collect();

            let path =
                std::env::temp_dir().join(format!("squiglab-eq-import-{}.txt", std::process::id()));
            // The configuration captured from the live site, which is the
            // authority on the format.
            std::fs::write(
                &path,
                "Preamp: 0.0002 dB\r\nFilter 1: ON PK Fc 500 Hz Gain -3.5 dB Q 1.2\r\n",
            )
            .unwrap();
            app.eq_import_path = path.to_string_lossy().into_owned();
            app.import_parametric();
            assert!(app.eq_import_error.is_none(), "{:?}", app.eq_import_error);

            let rows = app.eq_filters.get(&phone).expect("the import landed");
            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0].filter,
                squiglab::equalizer::Filter::new(
                    squiglab::equalizer::FilterType::Peaking,
                    500.0,
                    1.2,
                    -3.5
                )
            );
            assert!(rows[0].enabled);

            // A file that is not an Equalizer APO configuration is refused and
            // the curve keeps what it had.
            std::fs::write(&path, "Filter 1: ON LPQ Fc 500 Hz Gain -3 dB Q 1\r\n").unwrap();
            app.import_parametric();
            assert!(app.eq_import_error.is_some());
            assert_eq!(app.eq_filters.get(&phone).map(Vec::len), Some(1));
            std::fs::remove_file(path).unwrap();
        });
    }

    #[test]
    fn a_curve_keeps_its_cleared_filters_but_a_new_curve_gets_defaults() {
        let curves = vec![test_curve("Model", "L", 50.0)];
        let group = super::MeasurementGroup {
            brand: "Brand".to_owned(),
            brand_suffix: None,
            name: "Model".to_owned(),
            variants: vec![super::MeasurementVariant {
                name: "Default".to_owned(),
                curve_indices: vec![0],
            }],
            target: false,
        };
        let mut app = CrinGraphApp {
            curves,
            groups: vec![group],
            active: [0].into_iter().collect(),
            ..CrinGraphApp::default()
        };
        // Nothing installed yet: the panel's default bands are what a first
        // visit would see.
        assert!(app.eq_filters.is_empty());
        let defaults = super::default_eq_filters();
        assert_eq!(defaults.len(), 5, "the reference shows five default bands");
        app.eq_filters.insert(0, defaults);
        assert_eq!(app.enabled_filters(0).len(), 5);

        // Clearing a curve is remembered: an empty list is not "never edited".
        app.eq_filters.insert(0, Vec::new());
        assert!(app.enabled_filters(0).is_empty());
        assert!(app.eq_filters.contains_key(&0));
        // And the curve still draws its normalized response with no EQ child.
        app.rebuild_display_curves();
        assert!(app
            .display_curves
            .iter()
            .all(|curve| curve.source == CurveSource::Measurement));
    }

    #[test]
    fn the_sound_settings_default_to_a_tone_and_keep_two_slots() {
        let app = CrinGraphApp::default();
        // Both Compare slots start identical, on a mid tone at a low volume.
        assert_eq!(
            app.sound_settings[0],
            squiglab::audio::Settings {
                source: squiglab::audio::Source::Tone { freq: 1000.0 },
                volume: 0.2
            }
        );
        assert_eq!(app.sound_settings[0], app.sound_settings[1]);
        assert_eq!(app.sound_slot, 0);
        assert!(
            app.player.is_none(),
            "nothing is playing on a fresh session"
        );
    }

    #[test]
    fn starting_the_sound_reports_why_it_cannot() {
        // Playback is wired up on Linux through PipeWire; on other platforms the
        // player says so rather than failing silently. Either way the app is
        // left in a coherent state: playing or a message explaining why not.
        let mut app = CrinGraphApp::default();
        app.start_sound();
        if app.player.is_none() {
            assert!(
                app.sound_message.is_some(),
                "a failed start must explain itself"
            );
        } else {
            app.stop_sound();
            assert!(app.player.is_none());
            assert_eq!(app.sound_message.as_deref(), Some("Stopped."));
        }
    }

    /// A REW export with the given number of points, written to a temp file.
    fn write_measurement(name: &str, points: usize) -> PathBuf {
        // The file's name is the measurement's name, which is what the
        // reference tool uses, so the file is named exactly that.
        let directory =
            std::env::temp_dir().join(format!("squiglab-uploads-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join(format!("{name}.txt"));
        let mut text = String::from("Freq\tLevel\n");
        for index in 0..points {
            text.push_str(&format!(
                "{:.3}\t{:.2}\n",
                20.0 + index as f64 * 40.0,
                70.0 - (index % 5) as f64
            ));
        }
        std::fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn an_uploaded_measurement_lands_in_the_uploaded_brand() {
        with_isolated_config(|| {
            let mut app = CrinGraphApp::default();
            app.load_measurements().unwrap();
            let groups_before = app.groups.len();
            let path = write_measurement("My Headphones", 300);
            app.upload_path = path.to_string_lossy().into_owned();
            app.import_upload();
            assert!(app.upload_error.is_none(), "{:?}", app.upload_error);
            assert!(
                app.upload_message
                    .as_deref()
                    .is_some_and(|m| m.contains("Uploaded")),
                "{:?}",
                app.upload_message
            );
            assert_eq!(app.groups.len(), groups_before + 1);

            let group = app
                .groups
                .iter()
                .find(|group| group.name == "My Headphones")
                .expect("the upload is listed");
            assert_eq!(group.brand, super::UPLOADED_BRAND);
            assert!(!group.target);
            assert!(app.is_uploaded(
                app.groups
                    .iter()
                    .position(|g| g.name == "My Headphones")
                    .unwrap()
            ));
            std::fs::remove_file(path).unwrap();
        });
    }

    #[test]
    fn an_uploaded_measurement_is_left_out_of_a_shared_link() {
        with_isolated_config(|| {
            let mut app = CrinGraphApp::default();
            app.load_measurements().unwrap();
            let path = write_measurement("Shared Away", 300);
            app.upload_path = path.to_string_lossy().into_owned();
            app.import_upload();
            assert!(app.upload_error.is_none());
            let index = app
                .groups
                .iter()
                .position(|group| group.name == "Shared Away")
                .unwrap();
            app.active.insert(index);
            app.rebuild_display_curves();
            assert!(app.processing_error.is_none(), "{:?}", app.processing_error);
            // The upload is on the graph, but a link made now does not carry
            // it: the recipient has no such measurement.
            assert!(
                app.display_curves
                    .iter()
                    .any(|curve| curve.group_index == index),
                "the upload did not reach the graph"
            );
            let preferences = app.preferences();
            assert!(
                !preferences
                    .active
                    .iter()
                    .any(|saved| saved.key == super::group_key(&app.groups[index])),
                "an uploaded curve leaked into the shared state"
            );
            let uri = encode_share_uri(&preferences).unwrap();
            let restored = decode_share_uri(&uri).unwrap();
            assert_eq!(restored, preferences);
            std::fs::remove_file(path).unwrap();
        });
    }

    #[test]
    fn a_second_upload_of_the_same_measurement_joins_the_first() {
        with_isolated_config(|| {
            let mut app = CrinGraphApp::default();
            app.load_measurements().unwrap();
            let path = write_measurement("Twice", 300);
            app.upload_path = path.to_string_lossy().into_owned();
            app.import_upload();
            let groups_after_first = app
                .groups
                .iter()
                .filter(|group| group.name == "Twice")
                .count();
            app.import_upload();
            let groups_after_second = app
                .groups
                .iter()
                .filter(|group| group.name == "Twice")
                .count();
            assert_eq!(
                groups_after_first, 1,
                "the first upload should make one group"
            );
            assert_eq!(
                groups_after_second, 1,
                "the second upload made a second group"
            );
            // And the second file replaced the first rather than being stacked
            // beside it: measuring the same model again shows one curve.
            let index = app
                .groups
                .iter()
                .position(|group| group.name == "Twice")
                .unwrap();
            assert_eq!(
                app.groups[index].variants[0].curve_indices.len(),
                1,
                "the second upload was stacked on the first"
            );
            std::fs::remove_file(path).unwrap();
        });
    }

    #[test]
    fn a_right_channel_upload_gains_a_null_left_channel() {
        with_isolated_config(|| {
            let mut app = CrinGraphApp::default();
            app.load_measurements().unwrap();
            let path = write_measurement("Right Only R", 300);
            app.upload_path = path.to_string_lossy().into_owned();
            app.import_upload();
            assert!(app.upload_error.is_none(), "{:?}", app.upload_error);
            let index = app
                .groups
                .iter()
                .position(|group| group.name == "Right Only R")
                .unwrap();
            let indices = &app.groups[index].variants[0].curve_indices;
            assert_eq!(indices.len(), 2, "a right-only upload needs a left channel");
            let right = &app.curves[indices[0]];
            let left = &app.curves[indices[1]];
            assert_eq!(right.variant, "R");
            assert_eq!(left.variant, "L");
            // The null channel is on the same frequencies as the right one, so
            // the two line up rather than one being shifted.
            assert_eq!(right.points.len(), left.points.len());
            for (right_point, left_point) in right.points.iter().zip(&left.points) {
                assert_eq!(right_point.hz, left_point.hz);
            }
            assert!(left.points.iter().all(|point| point.db == 0.0));
            std::fs::remove_file(path).unwrap();
        });
    }

    #[test]
    fn a_target_upload_is_named_flagged_and_deduplicated() {
        with_isolated_config(|| {
            let mut app = CrinGraphApp::default();
            app.load_measurements().unwrap();
            let path = write_measurement("Brand New Target", 300);
            app.upload_path = path.to_string_lossy().into_owned();
            app.upload_as_target = true;
            app.import_upload();
            assert!(app.upload_error.is_none(), "{:?}", app.upload_error);
            // The curve is named with the " Target" suffix the database uses, so
            // the loader's own name splitter treats it as a target.
            assert!(app
                .curves
                .iter()
                .any(|curve| curve.model == "Brand New Target Target"));
            let index = app
                .groups
                .iter()
                .position(|group| group.name == "Brand New Target")
                .expect("the target is listed");
            assert!(app.groups[index].target, "an uploaded target is a target");
            assert!(app.is_uploaded(index));
            assert!(
                app.active_targets.is_empty(),
                "an upload is not selected on the user's behalf"
            );

            // A target the database already has is refused rather than
            // shadowing it.
            app.upload_path = write_measurement("Harman", 300)
                .to_string_lossy()
                .into_owned();
            app.import_upload();
            assert!(
                app.upload_error
                    .as_deref()
                    .is_some_and(|error| error.contains("already a target")),
                "{:?}",
                app.upload_error
            );
            std::fs::remove_file(path).unwrap();
        });
    }

    #[test]
    fn a_short_upload_is_refused_and_nothing_is_added() {
        with_isolated_config(|| {
            let mut app = CrinGraphApp::default();
            app.load_measurements().unwrap();
            let groups_before = app.groups.len();
            let curves_before = app.curves.len();
            let path = write_measurement("Fragment", 50);
            app.upload_path = path.to_string_lossy().into_owned();
            app.import_upload();
            assert!(
                app.upload_error
                    .as_deref()
                    .is_some_and(|error| error.contains("at least 128")),
                "{:?}",
                app.upload_error
            );
            assert_eq!(app.groups.len(), groups_before);
            assert_eq!(app.curves.len(), curves_before);
            std::fs::remove_file(path).unwrap();
        });
    }

    #[test]
    fn a_missing_upload_file_is_reported_not_ignored() {
        with_isolated_config(|| {
            let mut app = CrinGraphApp::default();
            app.load_measurements().unwrap();
            app.upload_path = "/nonexistent/measurement.txt".to_owned();
            app.import_upload();
            assert!(app.upload_error.is_some());
            // And a path with no file name in it is refused before any read.
            app.upload_path = "/".to_owned();
            app.import_upload();
            assert!(app.upload_error.is_some());
        });
    }

    #[test]
    fn the_manager_is_tall_enough_to_show_every_selected_curve() {
        // A manager shorter than its own rows pushes the last ones out of sight
        // and makes the user scroll a sliver, which reads as "my selection is
        // not showing". A wide viewport fits eight curves without scrolling.
        for count in 1..=8 {
            let (default, min) = manager_heights(1440.0, count);
            let needed = super::MANAGER_HEIGHT_BASE + count as f32 * super::MANAGER_ROW_HEIGHT;
            assert!(
                default >= needed,
                "{count} curves need {needed}px but the manager defaults to {default}px"
            );
            assert!(min >= needed, "{count} curves, but the minimum is {min}px");
        }
        // A narrow viewport stacks the controls onto a second line, so the
        // panel has to budget for that too.
        for count in 1..=4 {
            let (default, min) = manager_heights(430.0, count);
            let needed =
                super::MANAGER_HEIGHT_BASE + count as f32 * super::MANAGER_STACKED_ROW_HEIGHT;
            assert!(
                default >= needed && min >= needed,
                "{count} stacked curves need {needed}px, got {default}px / {min}px"
            );
        }
    }

    #[test]
    fn a_manager_that_cannot_fit_its_rows_can_still_be_shrunk_to_scroll() {
        // A selection big enough to exceed the maximum — measurements, targets
        // and corrections together — has to fall back to the header and a single
        // row. If it did not, the panel would be pinned at its maximum with the
        // overflow unreachable.
        let (default, min) = manager_heights(430.0, 10);
        assert_eq!(default, super::MANAGER_MAX_HEIGHT);
        assert!(
            min < default,
            "the manager must be shrinkable to reach rows past its maximum"
        );
        assert!(min >= super::MANAGER_HEIGHT_BASE + super::MANAGER_STACKED_ROW_HEIGHT);
    }

    #[test]
    fn startup_loads_the_configured_measurement_and_targets() {
        with_isolated_config(|| {
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
        });
    }

    #[test]
    fn loading_with_a_selection_asks_the_library_to_reveal_it() {
        with_isolated_config(|| {
            // A first launch selects a default curve, which the library must
            // scroll into view.
            let mut app = CrinGraphApp::default();
            app.load_measurements().unwrap();
            assert!(!app.active.is_empty());
            assert!(app.library_reveal_pending);

            // A restored session that selected nothing has nothing to reveal.
            let path = super::preferences_path().unwrap();
            write_preferences_to(
                &path,
                &Preferences {
                    active: Vec::new(),
                    ..Preferences::default()
                },
            )
            .unwrap();
            let mut restored = CrinGraphApp::default();
            restored.load_measurements().unwrap();
            assert!(restored.active.is_empty());
            assert!(!restored.library_reveal_pending);
        });
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
        let book = squiglab::data::load_phone_book(Path::new("data/phone_book.json")).unwrap();
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
                source: CurveSource::Measurement,
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
                source: CurveSource::Measurement,
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

    /// A flat pair of points at the frequencies the delta bar samples on.
    fn flat_pair(db: f32) -> Vec<FRPoint> {
        [100.0, 1000.0]
            .into_iter()
            .map(|hz| FRPoint { hz, db })
            .collect()
    }

    /// An app with one target at 60 dB, one measurement at 65 dB, and a
    /// three-point shared grid.
    fn bar_app() -> CrinGraphApp {
        let groups = vec![
            MeasurementGroup {
                brand: "Neutral targets".to_owned(),
                brand_suffix: None,
                name: "Harman".to_owned(),
                variants: vec![MeasurementVariant {
                    name: "Target".to_owned(),
                    curve_indices: vec![],
                }],
                target: true,
            },
            MeasurementGroup {
                brand: "Brand".to_owned(),
                brand_suffix: None,
                name: "Headphone".to_owned(),
                variants: vec![MeasurementVariant {
                    name: "Avg".to_owned(),
                    curve_indices: vec![],
                }],
                target: false,
            },
        ];
        let display_curves = vec![
            DisplayCurve {
                source: CurveSource::Measurement,
                group_index: 0,
                label: "Harman".to_owned(),
                points: flat_pair(60.0),
                target: true,
            },
            DisplayCurve {
                source: CurveSource::Measurement,
                group_index: 1,
                label: "Headphone".to_owned(),
                points: flat_pair(65.0),
                target: false,
            },
        ];
        CrinGraphApp {
            upload_grid: vec![100.0, 1000.0],
            groups,
            active_targets: [0].into_iter().collect(),
            active: [1].into_iter().collect(),
            display_curves,
            ..Default::default()
        }
    }

    fn delta_target(name: &str, db: f32) -> DeltaTarget {
        DeltaTarget {
            name: name.to_owned(),
            clone_rig: "clone".to_owned(),
            original_rig: "original".to_owned(),
            synthetic: true,
            note: None,
            points: flat_pair(db),
        }
    }

    #[test]
    fn the_delta_bar_measures_a_headphone_against_the_target() {
        let app = bar_app();
        let deviations = app.delta_bar_deviations();
        // One measurement, no corrections, so one line and nothing else.
        assert_eq!(deviations.len(), 1);
        assert_eq!(deviations[0].label, "Headphone");
        // 65 dB measured against a 60 dB target is 5 dB above it.
        assert!(deviations[0]
            .points
            .iter()
            .all(|point| (point.db - 5.0).abs() < 1e-4));
    }

    #[test]
    fn the_delta_bar_draws_nothing_without_a_target() {
        let mut app = bar_app();
        // A deviation from nothing is not a deviation.
        app.active_targets.clear();
        assert!(app.delta_bar_deviations().is_empty());
    }

    #[test]
    fn a_selected_correction_removes_its_rig_error_from_the_deviation() {
        let mut app = bar_app();
        // A rig that reads 2 dB high makes a 65 dB headphone look like 65 dB
        // when the true figure, on the target's rig, would be 63 dB: 3 dB
        // above target rather than 5.
        app.delta_targets = vec![delta_target("\u{394} Clone", 2.0)];
        app.active_deltas.insert(0);
        let deviations = app.delta_bar_deviations();
        let corrected = deviations
            .iter()
            .find(|curve| curve.label.contains("rig-corrected"))
            .expect("the corrected line");
        let uncorrected = deviations
            .iter()
            .find(|curve| curve.label.contains("uncorrected"))
            .expect("the uncorrected line");
        assert!(corrected.points.iter().all(|p| (p.db - 3.0).abs() < 1e-4));
        assert!(uncorrected.points.iter().all(|p| (p.db - 5.0).abs() < 1e-4));
        // The gap between the two lines is exactly the rig correction, which is
        // the whole reason both are drawn.
        for (corrected, uncorrected) in corrected.points.iter().zip(&uncorrected.points) {
            assert!(((corrected.db - uncorrected.db) + 2.0).abs() < 1e-4);
        }
    }

    #[test]
    fn a_selected_correction_is_drawn_on_the_bar_as_well() {
        let mut app = bar_app();
        app.delta_targets = vec![delta_target("\u{394} Clone", 2.0)];
        app.active_deltas.insert(0);
        let deviations = app.delta_bar_deviations();
        let correction = deviations
            .iter()
            .find(|curve| curve.label == "\u{394} Clone")
            .expect("the correction is drawn so its own shape is visible");
        assert!(correction.dashed);
        assert!(correction.points.iter().all(|p| (p.db - 2.0).abs() < 1e-4));
    }

    #[test]
    fn two_selected_corrections_compose() {
        let mut app = bar_app();
        app.delta_targets = vec![delta_target("A", 2.0), delta_target("B", 1.0)];
        app.active_deltas = [0, 1].into_iter().collect();
        let deviations = app.delta_bar_deviations();
        let corrected = deviations
            .iter()
            .find(|curve| curve.label.contains("rig-corrected"))
            .expect("the corrected line");
        // 65 - 60 - 2 - 1
        assert!(corrected.points.iter().all(|p| (p.db - 2.0).abs() < 1e-4));
    }

    #[test]
    fn the_delta_bar_caption_names_the_target_and_the_corrections() {
        let mut app = bar_app();
        assert_eq!(app.delta_bar_caption(), "deviation from Harman");
        app.delta_targets = vec![delta_target("\u{394} Clone", 2.0)];
        app.active_deltas.insert(0);
        assert_eq!(
            app.delta_bar_caption(),
            "deviation from Harman \u{b7} rig-corrected by \u{394} Clone"
        );
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
        assert_eq!(
            squiglab::processing::interpolate_db(&points, 10.0),
            Some(10.0)
        );
        assert_eq!(
            squiglab::processing::interpolate_db(&points, 10000.0),
            Some(20.0)
        );
        assert!(
            (squiglab::processing::interpolate_db(&points, 316.22778).unwrap() - 15.0).abs()
                < 0.001
        );
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
            squiglab::color::get_curve_color_with_offset(0, 5)
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

    /// An app with one target, one headphone, and one rig correction.
    fn chip_app() -> CrinGraphApp {
        let mut app = bar_app();
        app.delta_targets = vec![DeltaTarget {
            name: "5128DF".to_owned(),
            clone_rig: "clone".to_owned(),
            original_rig: "original".to_owned(),
            synthetic: false,
            note: Some("calibrated 2026-01-14".to_owned()),
            points: vec![FRPoint {
                hz: 1000.0,
                db: 0.5,
            }],
        }];
        app
    }

    #[test]
    fn a_rig_correction_chip_says_what_it_corrects_what_it_measured() {
        let app = chip_app();
        let chips = app.delta_chips();
        assert_eq!(chips.len(), 1);
        // Named with a delta because it is a difference between two rigs.
        assert_eq!(chips[0].label, "\u{394} 5128DF");
        assert!(!chips[0].selected);
        assert_eq!(chips[0].tooltip, "calibrated 2026-01-14");
    }

    #[test]
    fn a_selected_correction_chip_says_it_is_selected() {
        let mut app = chip_app();
        app.toggle_delta(0);
        let chips = app.delta_chips();
        assert_eq!(chips.len(), 1);
        assert!(chips[0].selected);
        // And its own state round-trips through the toggle.
        assert_eq!(app.active_deltas.len(), 1);
        app.toggle_delta(0);
        assert!(!app.delta_chips()[0].selected);
    }

    #[test]
    fn a_placeholder_correction_warns_that_it_corrects_nothing() {
        let mut app = chip_app();
        app.delta_targets[0].synthetic = true;
        let chips = app.delta_chips();
        // A chip is the only place a rig correction can carry a warning, and a
        // chip nobody distrusted is worse than no chip at all.
        assert!(
            chips[0].tooltip.contains("placeholder"),
            "the tooltip said {:?}",
            chips[0].tooltip
        );
        // It keeps whatever note it had, so the reason is not lost.
        assert!(chips[0].tooltip.contains("calibrated 2026-01-14"));
    }

    #[test]
    fn a_reference_target_chip_carries_the_target_it_is_one_of() {
        let app = chip_app();
        let chips = app.target_chips();
        // Only targets: a chip strip of everything would not say what is being
        // read against.
        assert_eq!(chips.len(), 1);
        assert!(!chips[0].label.contains("Headphone"));
    }

    #[test]
    fn chips_stay_in_the_order_the_strip_lays_them_out() {
        // Two rig corrections and two targets: the chips have to come back in a
        // stable order or the strip would reshuffle under the pointer.
        let mut app = chip_app();
        app.delta_targets.push(DeltaTarget {
            name: "711 clone".to_owned(),
            clone_rig: "clone".to_owned(),
            original_rig: "original".to_owned(),
            synthetic: false,
            note: None,
            points: Vec::new(),
        });
        let labels: Vec<String> = app
            .delta_chips()
            .into_iter()
            .map(|chip| chip.label)
            .collect();
        assert_eq!(labels, vec!["\u{394} 5128DF", "\u{394} 711 clone"]);
    }

    /// A curve, a variant and a mode, to name and format.
    fn export_group(target: bool) -> (MeasurementGroup, MeasurementVariant) {
        (
            MeasurementGroup {
                brand: "Neutral targets".to_owned(),
                brand_suffix: None,
                name: "Harman".to_owned(),
                variants: Vec::new(),
                target,
            },
            MeasurementVariant {
                name: "L".to_owned(),
                curve_indices: Vec::new(),
            },
        )
    }

    #[test]
    fn an_exported_curve_says_what_it_is_and_which_channel() {
        let (group, variant) = export_group(false);
        assert_eq!(
            super::curve_file_name(&group, &variant, ChannelMode::Left),
            "Harman-measurement-L-Left.txt"
        );
        let (target, variant) = export_group(true);
        assert_eq!(
            super::curve_file_name(&target, &variant, ChannelMode::Average),
            "Harman-target-L-Average.txt"
        );
    }

    #[test]
    fn a_name_a_filesystem_cannot_hold_is_folded_to_something_it_can() {
        // Slashes and colons are legal in a label and illegal in a file name,
        // and a name that reduced to nothing but separators would collide.
        let (group, variant) = export_group(false);
        let group = MeasurementGroup {
            name: "A/B:C*D".to_owned(),
            ..group
        };
        let named = super::curve_file_name(&group, &variant, ChannelMode::Both);
        assert!(
            !named.contains('/') && !named.contains(':'),
            "{named} is not a file name"
        );
        assert!(named.ends_with(".txt"));
    }

    #[test]
    fn an_exported_curve_is_its_own_digits_and_nothing_else() {
        let text = super::curve_text(&[
            FRPoint { hz: 20.0, db: 0.0 },
            FRPoint {
                hz: 1000.5,
                db: -3.25,
            },
        ]);
        // No header: a header is a line every naive parser has to reject.
        assert_eq!(text.lines().count(), 2);
        assert_eq!(text.lines().next().unwrap(), "20.000000,0.000000");
        assert_eq!(text.lines().nth(1).unwrap(), "1000.500000,-3.250000");
    }

    #[test]
    fn an_exported_curve_comes_back_identical() {
        // The whole point of the export: read it back and get what was written.
        let points = [
            FRPoint { hz: 20.0, db: 1.5 },
            FRPoint {
                hz: 500.0,
                db: -2.25,
            },
            FRPoint {
                hz: 10000.0,
                db: 0.125,
            },
        ];
        let text = super::curve_text(&points);
        let reparsed: Vec<FRPoint> = text
            .lines()
            .filter_map(squiglab::data::parse_point)
            .collect();
        assert_eq!(reparsed.len(), points.len());
        for (written, read) in points.iter().zip(&reparsed) {
            assert!(
                (written.hz - read.hz).abs() < 1e-6,
                "{} vs {}",
                written.hz,
                read.hz
            );
            assert!(
                (written.db - read.db).abs() < 1e-6,
                "{} vs {}",
                written.db,
                read.db
            );
        }
    }

    #[test]
    fn smoothing_is_cycled_through_the_levels_worth_stopping_at() {
        // The row's smoothing mark cycles rather than slides, so the cycle has
        // to cover the levels worth stopping at and come back where it started.
        let mut level = 0;
        let mut visited = Vec::new();
        loop {
            level = super::next_smoothing_level(level);
            assert!(level <= super::MAX_SMOOTH_LEVEL, "{level} is off the end");
            visited.push(level);
            if level == 0 {
                break;
            }
        }
        // Round values, plus 0, which is the measurement as it was taken.
        assert_eq!(visited, vec![1, 2, 5, 10, super::MAX_SMOOTH_LEVEL, 0]);
    }

    #[test]
    fn a_channel_mode_knows_which_prepared_channel_is_its_own() {
        // `Both` and `Average` are the only modes whose channel carries the
        // mode's name, so a mode has to match by prefix rather than by name.
        assert!(ChannelMode::Left.accepts("L"));
        assert!(ChannelMode::Right.accepts("R"));
        // A curve with one unlabelled channel is both channels, and its average.
        assert!(ChannelMode::Both.accepts(""));
        assert!(ChannelMode::Both.accepts("L"));
        assert!(ChannelMode::Average.accepts("Average"));
        assert!(ChannelMode::Average.accepts("avg"));
        assert!(!ChannelMode::Average.accepts("L"));
    }

    #[test]
    fn preferences_round_trip_and_missing_or_malformed_files_are_handled() {
        let directory =
            std::env::temp_dir().join(format!("squiglab-preferences-{}", std::process::id()));
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
            show_delta: true,
            label_positions: [("Brand · Model".to_owned(), [0.4, 0.6])]
                .into_iter()
                .collect(),
            label_layout: squiglab::graph::LabelLayout::CornerTopLeft,
            tutorial_segment: Some(3),
            y_scale: 1.75,
            curve_color_offsets: [("Brand\u{1f}Model".into(), 2)].into_iter().collect(),
            eq_filters: BTreeMap::new(),
            active_deltas: vec!["Δ Placeholder clone rig".into()],
        };
        write_preferences_to(&path, &preferences).unwrap();
        assert_eq!(read_preferences_from(&path).unwrap(), Some(preferences));

        std::fs::write(&path, b"{not valid json").unwrap();
        assert!(read_preferences_from(&path).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn preferences_written_before_the_manager_toggle_was_removed_still_load() {
        // The manager is no longer toggleable, so `show_manager` is gone from
        // the saved settings. Every install that ran before this still has it
        // written to disk, and `Preferences` deliberately does not deny unknown
        // fields: adding that would turn a removed setting into a startup
        // failure for anyone who had ever run the app.
        let directory =
            std::env::temp_dir().join(format!("squiglab-legacy-{}", std::process::id()));
        let path = directory.join("preferences.toml");
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            &path,
            r#"{"show_manager": false, "show_measurements": true}"#,
        )
        .unwrap();
        let preferences = read_preferences_from(&path).unwrap().unwrap();
        assert!(preferences.show_measurements);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn png_export_writes_the_captured_rgba_image() {
        let directory =
            std::env::temp_dir().join(format!("squiglab-export-{}", std::process::id()));
        let path = directory.join("capture.png");
        let _ = std::fs::remove_dir_all(&directory);
        let image = egui::ColorImage {
            size: [2, 1],
            source_size: egui::vec2(2.0, 1.0),
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
        // Links issued under the previous name still decode, and re-encode to
        // the current scheme.
        let legacy = uri.replacen(super::SHARE_URI_PREFIX, super::LEGACY_SHARE_URI_PREFIX, 1);
        assert!(legacy.starts_with(super::LEGACY_SHARE_URI_PREFIX));
        assert_eq!(decode_share_uri(&legacy).unwrap(), preferences);
        assert!(decode_share_uri("squiglab://state/v1/0").is_err());
        assert!(decode_share_uri("squiglab://state/v1/zz").is_err());
        assert!(decode_share_uri("cringraph://state/v1/zz").is_err());
        // An unrelated scheme that merely looks similar is still rejected.
        assert!(decode_share_uri("squiglabx://state/v1/").is_err());
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
