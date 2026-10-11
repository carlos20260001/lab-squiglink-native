use std::default::Default;

/// App state and persistence for CrinGraph.
///
/// Holds the application state including loaded data, view configuration,
/// UI state, and color management.
#[derive(Default)]
pub struct AppState {
    /// Loaded measurement curves
    pub curves: Vec<crate::data::CurveInfo>,
    /// Shared frequency grid (20Hz - 20kHz, 490 points)
    pub f_values: Option<Vec<f32>>,
    /// Current view transform (x range in Hz, y range in dB)
    pub view_transform: ViewTransform,
    /// Active phone selections
    pub active_phones: Vec<crate::data::PhoneInfo>,
    /// Dark mode toggle
    pub dark_mode: bool,
    /// Whether labels are shown
    pub show_labels: bool,
    /// Whether inspector mode is active
    pub inspect_mode: bool,
    /// Smoothing level (0 = none, 1-12 = n-octave)
    pub smooth_level: usize,
    /// Normalization mode (0 = dB/phon, 1 = Hz)
    pub norm_sel: bool,
    /// Target phon level (when norm_sel=0)
    pub norm_phon: f32,
    /// Target frequency Hz (when norm_sel=1)
    pub norm_fr: f32,
    /// Zoom region state
    pub zoom_full: bool,
    pub zoom_bass: bool,
    pub zoom_mids: bool,
    pub zoom_treble: bool,
    /// Loaded phone book
    pub phon_book: Option<Vec<crate::data::BrandInfo>>,
}

/// View transform for the graph
#[derive(Clone, Default)]
pub struct ViewTransform {
    /// Hz range shown (typically 20.0 - 20000.0)
    pub x_range: (f32, f32),
    /// dB range shown (typically 30.0 - 90.0)
    pub y_range: (f32, f32),
}