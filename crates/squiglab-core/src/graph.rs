//! The graph's vocabulary: what a curve is, how far the graph reaches, and
//! how its labels are arranged.
//!
//! These are rules of the program rather than of the picture. The interface
//! draws them; the core decides them, and the interface can be rewritten
//! without any of this changing.

use crate::color::Rgb;
use crate::data::FRPoint;
use serde::{Deserialize, Serialize};

/// How much of the frequency axis the graph shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum ZoomRegion {
    #[default]
    Full,
    Bass,
    Mids,
    Treble,
}

impl ZoomRegion {
    /// The frequency range the region covers, in hertz.
    pub fn x_range(self) -> (f32, f32) {
        match self {
            Self::Full => (20.0, 20000.0),
            Self::Bass => (20.0, 400.0),
            Self::Mids => (100.0, 4000.0),
            Self::Treble => (1000.0, 20000.0),
        }
    }
}

/// One curve on the graph: its name, the points it is drawn from, and how it
/// is drawn.
#[derive(Clone, Copy)]
pub struct PlotCurve<'a> {
    pub label: &'a str,
    pub label_key: &'a str,
    pub points: &'a [FRPoint],
    pub color: Rgb,
    pub dashed: bool,
    /// Whether this curve is a reference target rather than a measurement.
    /// Targets are excluded from the plotted dB range.
    pub target: bool,
}

/// Where the curve labels are put.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum LabelLayout {
    CurveEnd,
    #[default]
    CornerBottomLeft,
    CornerBottomRight,
    CornerTopLeft,
}

impl LabelLayout {
    /// Label for the toolbar toggle, cycling through the layouts.
    pub fn next(self) -> Self {
        match self {
            LabelLayout::CornerBottomLeft => LabelLayout::CornerBottomRight,
            LabelLayout::CornerBottomRight => LabelLayout::CornerTopLeft,
            LabelLayout::CornerTopLeft => LabelLayout::CurveEnd,
            LabelLayout::CurveEnd => LabelLayout::CornerBottomLeft,
        }
    }

    /// Short name for a control.
    pub fn short_name(self) -> &'static str {
        match self {
            LabelLayout::CornerBottomLeft => "Labels: corner",
            LabelLayout::CornerBottomRight => "Labels: corner",
            LabelLayout::CornerTopLeft => "Labels: corner",
            LabelLayout::CurveEnd => "Labels: curve",
        }
    }

    /// Which corner, for the tooltip.
    pub fn corner_name(self) -> &'static str {
        match self {
            LabelLayout::CornerBottomLeft => "bottom-left",
            LabelLayout::CornerBottomRight => "bottom-right",
            LabelLayout::CornerTopLeft => "top-left",
            LabelLayout::CurveEnd => "",
        }
    }
}
