//! The graph's vocabulary: what a curve is, how far the graph reaches, and
//! how its labels are arranged.
//!
//! These are rules of the program rather than of the picture. The interface
//! draws them; the core decides them, and the interface can be rewritten
//! without any of this changing.
use crate::color::Rgb;
use crate::data::FRPoint;
use emath::{Pos2, Rect, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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

/// How many labels are grouped into one corner before the rest follow their own
/// curves. The reference groups about five.
pub const MAX_GROUPED_LABELS: usize = 6;

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

/// A named band of the frequency axis.
///
/// The reference names the bands of the axis, and the names are part of its
/// vocabulary rather than of any one picture of it, so they live here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrequencyBand {
    /// The name the reference gives the band.
    pub title: &'static str,
    /// The frequencies the band covers.
    pub range_hz: (f32, f32),
}

/// The bands of the frequency axis, in order.
pub const FREQUENCY_BANDS: [FrequencyBand; 7] = [
    FrequencyBand {
        title: "Sub bass",
        range_hz: (20.0, 60.0),
    },
    FrequencyBand {
        title: "Mid bass",
        range_hz: (60.0, 200.0),
    },
    FrequencyBand {
        title: "Lower midrange",
        range_hz: (200.0, 500.0),
    },
    FrequencyBand {
        title: "Upper midrange",
        range_hz: (500.0, 2000.0),
    },
    FrequencyBand {
        title: "Presence region",
        range_hz: (2000.0, 5000.0),
    },
    FrequencyBand {
        title: "Mid treble",
        range_hz: (5000.0, 10000.0),
    },
    FrequencyBand {
        title: "Air",
        range_hz: (10000.0, 20000.0),
    },
];

pub fn y_range(curves: &[PlotCurve<'_>]) -> (f32, f32) {
    // Targets are excluded from the plotted dB range. A target with a steep
    // high-frequency rolloff — the Harman target reaches 14.7 dB at 20 kHz —
    // would otherwise drag the axis down to 10 dB and compress the measurements
    // being compared into the top of the graph, which is the opposite of the
    // comparison this tool exists for. Targets are still drawn; they simply clip
    // at the bottom of the plot, as they do in the reference tool.
    //
    // If only targets are active, fall back to them so the plot is never empty.
    let measuring: Vec<&PlotCurve<'_>> = curves.iter().filter(|curve| !curve.target).collect();
    let curves: &[&PlotCurve<'_>] = if measuring.is_empty() {
        &curves.iter().collect::<Vec<_>>()
    } else {
        &measuring
    };
    let bounds = curves
        .iter()
        .flat_map(|curve| curve.points.iter().map(|point| point.db))
        .filter(|value| value.is_finite())
        .fold(None, |bounds: Option<(f32, f32)>, value| {
            Some(match bounds {
                Some((min, max)) => (min.min(value), max.max(value)),
                None => (value, value),
            })
        });
    match bounds {
        Some((min, max)) => {
            let padding = ((max - min) * 0.08).max(3.0);
            (
                ((min - padding) / 5.0).floor() * 5.0,
                ((max + padding) / 5.0).ceil() * 5.0,
            )
        }
        None => (20.0, 120.0),
    }
}
pub fn format_frequency(hz: f32) -> String {
    if hz >= 1000.0 {
        format!("{:.2} kHz", hz / 1000.0)
    } else {
        format!("{hz:.0} Hz")
    }
}
pub fn format_axis_frequency(hz: f32) -> String {
    if hz <= 20.0 {
        format!("{hz:.0}Hz")
    } else if hz >= 20000.0 {
        format!("{:.0}kHz", hz / 1000.0)
    } else if hz >= 1000.0 {
        format!("{}k", hz as u32 / 1000)
    } else {
        format!("{hz:.0}")
    }
}
pub fn nearest_point(points: &[FRPoint], frequency: f32) -> Option<&FRPoint> {
    if points.is_empty() {
        return None;
    }
    let upper = points.partition_point(|point| point.hz < frequency);
    match (upper.checked_sub(1), points.get(upper)) {
        (Some(lower), Some(upper_point)) => {
            if (frequency / points[lower].hz).ln().abs() <= (upper_point.hz / frequency).ln().abs()
            {
                Some(&points[lower])
            } else {
                Some(upper_point)
            }
        }
        (Some(lower), None) => Some(&points[lower]),
        (None, Some(upper_point)) => Some(upper_point),
        (None, None) => None,
    }
}
pub fn curve_control_points(points: &[Pos2], natural: bool) -> Vec<[Pos2; 4]> {
    if points.len() < 3 {
        return Vec::new();
    }
    let (first_x, second_x, first_y, second_y) = if natural {
        let (first_x, second_x) =
            natural_control_points(&points.iter().map(|p| p.x).collect::<Vec<_>>());
        let (first_y, second_y) =
            natural_control_points(&points.iter().map(|p| p.y).collect::<Vec<_>>());
        (first_x, second_x, first_y, second_y)
    } else {
        let tension = 1.0 / 12.0;
        let mut first_x = Vec::with_capacity(points.len() - 1);
        let mut second_x = Vec::with_capacity(points.len() - 1);
        let mut first_y = Vec::with_capacity(points.len() - 1);
        let mut second_y = Vec::with_capacity(points.len() - 1);
        for index in 0..points.len() - 1 {
            let start = points[index];
            let end = points[index + 1];
            let previous = points[index.saturating_sub(1)];
            let next = points[(index + 2).min(points.len() - 1)];
            first_x.push(if index == 0 {
                start.x
            } else {
                start.x + tension * (end.x - previous.x)
            });
            first_y.push(if index == 0 {
                start.y
            } else {
                start.y + tension * (end.y - previous.y)
            });
            second_x.push(if index + 1 == points.len() - 1 {
                end.x
            } else {
                end.x + tension * (start.x - next.x)
            });
            second_y.push(if index + 1 == points.len() - 1 {
                end.y
            } else {
                end.y + tension * (start.y - next.y)
            });
        }
        (first_x, second_x, first_y, second_y)
    };

    (0..points.len() - 1)
        .map(|index| {
            [
                points[index],
                Pos2::new(first_x[index], first_y[index]),
                Pos2::new(second_x[index], second_y[index]),
                points[index + 1],
            ]
        })
        .collect()
}
pub fn natural_control_points(values: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let segments = values.len() - 1;
    let mut lower = vec![0.0; segments];
    let mut diagonal = vec![0.0; segments];
    let mut rhs = vec![0.0; segments];
    lower[0] = 0.0;
    diagonal[0] = 2.0;
    rhs[0] = values[0] + 2.0 * values[1];
    for index in 1..segments - 1 {
        lower[index] = 1.0;
        diagonal[index] = 4.0;
        rhs[index] = 4.0 * values[index] + 2.0 * values[index + 1];
    }
    lower[segments - 1] = 2.0;
    diagonal[segments - 1] = 7.0;
    rhs[segments - 1] = 8.0 * values[segments - 1] + values[segments];
    for index in 1..segments {
        let multiplier = lower[index] / diagonal[index - 1];
        diagonal[index] -= multiplier;
        rhs[index] -= multiplier * rhs[index - 1];
    }

    let mut first = vec![0.0; segments];
    let mut second = vec![0.0; segments];
    first[segments - 1] = rhs[segments - 1] / diagonal[segments - 1];
    for index in (0..segments - 1).rev() {
        first[index] = (rhs[index] - first[index + 1]) / diagonal[index];
    }
    second[segments - 1] = (values[segments] + first[segments - 1]) / 2.0;
    for index in 0..segments - 1 {
        second[index] = 2.0 * values[index + 1] - first[index + 1];
    }
    (first, second)
}
pub fn cubic_bezier(start: Pos2, control1: Pos2, control2: Pos2, end: Pos2, t: f32) -> Pos2 {
    let inverse = 1.0 - t;
    start * inverse.powi(3)
        + control1.to_vec2() * (3.0 * inverse.powi(2) * t)
        + control2.to_vec2() * (3.0 * inverse * t.powi(2))
        + end.to_vec2() * t.powi(3)
}

pub fn nice_limit(peak: f32) -> f32 {
    for step in [0.5_f32, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0] {
        if peak <= step {
            return step;
        }
    }
    (peak / 50.0).ceil() * 50.0
}

pub fn tick_db(db: f32) -> String {
    let magnitude = db.abs();
    let sign = if db < 0.0 { "\u{2212}" } else { "+" };
    if magnitude >= 10.0 {
        format!("{sign}{magnitude:.0}")
    } else {
        format!("{sign}{magnitude:.1}")
    }
}

pub fn corner_label_position(
    corner: LabelLayout,
    index: usize,
    size: Vec2,
    plot_rect: Rect,
    line_height: f32,
) -> Pos2 {
    // Inset clears the y-axis labels on the left and the tick text on the
    // right, so the block never sits under either.
    let inset = Vec2::new(10.0, 8.0);
    let line = line_height;
    let row = index.min(MAX_GROUPED_LABELS - 1) as f32;

    match corner {
        // CurveEnd never reaches here; the caller branches on it first.
        LabelLayout::CurveEnd => Pos2::ZERO,
        LabelLayout::CornerBottomLeft => Pos2::new(
            plot_rect.left() + inset.x + 26.0,
            plot_rect.bottom() - inset.y - size.y - row * line,
        ),
        LabelLayout::CornerBottomRight => Pos2::new(
            plot_rect.right() - inset.x - size.x - 26.0,
            plot_rect.bottom() - inset.y - size.y - row * line,
        ),
        LabelLayout::CornerTopLeft => Pos2::new(
            plot_rect.left() + inset.x + 26.0,
            plot_rect.top() + inset.y + 16.0 + row * line,
        ),
    }
}

pub fn place_labels(mut labels: Vec<(String, f32, f32)>, plot_rect: Rect) -> BTreeMap<String, f32> {
    // Keyed by the caller-supplied label key rather than the drawn text: two
    // curves can legitimately carry the same text (the left and right channel
    // of one measurement, for instance), and keying by text would collapse
    // them onto a single slot and let them overlap.
    labels.sort_by(|left, right| left.1.total_cmp(&right.1));
    let mut positioned: Vec<(String, f32, f32)> = Vec::with_capacity(labels.len());
    let mut next_top = plot_rect.top();
    for (key, desired_center, height) in labels {
        let top = (desired_center - height / 2.0).max(next_top);
        positioned.push((key, top, height));
        next_top = top + height + 2.0;
    }
    let mut next_top = plot_rect.bottom();
    for (_, top, height) in positioned.iter_mut().rev() {
        *top = top.min(next_top - *height);
        next_top = *top - 2.0;
    }
    positioned
        .into_iter()
        .map(|(key, top, height)| {
            (
                key,
                top.clamp(
                    plot_rect.top(),
                    (plot_rect.bottom() - height).max(plot_rect.top()),
                ),
            )
        })
        .collect()
}

pub fn clamp_label_position(position: Pos2, size: Vec2, bounds: Rect) -> Pos2 {
    Pos2::new(
        position
            .x
            .clamp(bounds.left(), (bounds.right() - size.x).max(bounds.left())),
        position
            .y
            .clamp(bounds.top(), (bounds.bottom() - size.y).max(bounds.top())),
    )
}

pub fn scaled_y_range((minimum, maximum): (f32, f32), scale: f32) -> (f32, f32) {
    let center = (minimum + maximum) / 2.0;
    let half_range = (maximum - minimum) * scale.clamp(0.35, 4.0) / 2.0;
    (center - half_range, center + half_range)
}

pub fn sample_curve_path(points: &[Pos2], smooth: bool) -> Vec<Pos2> {
    let controls = curve_control_points(points, smooth);
    if controls.is_empty() {
        return points.to_vec();
    }

    let mut sampled = Vec::with_capacity(controls.len() * 8 + 1);
    sampled.push(points[0]);
    for [start, control1, control2, end] in controls {
        for step in 1..=8 {
            sampled.push(cubic_bezier(
                start,
                control1,
                control2,
                end,
                step as f32 / 8.0,
            ));
        }
    }
    sampled
}

pub fn point_segment_distance(point: Pos2, start: Pos2, end: Pos2) -> f32 {
    let segment = end - start;
    let length_squared = segment.length_sq();
    if length_squared == 0.0 {
        return point.distance(start);
    }
    let projection = ((point - start).dot(segment) / length_squared).clamp(0.0, 1.0);
    point.distance(start + segment * projection)
}

pub fn nearest_curve_index(curves: &[Vec<Pos2>], position: Pos2, threshold: f32) -> Option<usize> {
    curves
        .iter()
        .enumerate()
        .filter_map(|(index, points)| {
            let distance = points
                .windows(2)
                .map(|pair| point_segment_distance(position, pair[0], pair[1]))
                .fold(f32::INFINITY, f32::min);
            (distance <= threshold).then_some((index, distance))
        })
        .min_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(index, _)| index)
}
#[derive(Clone, Copy)]
pub struct Transform {
    pub rect: Rect,
    pub x_range: (f32, f32),
    pub y_range: (f32, f32),
}

impl Transform {
    pub fn new(rect: Rect, x_range: (f32, f32), y_range: (f32, f32)) -> Self {
        Self {
            rect,
            x_range,
            y_range,
        }
    }

    pub fn hz_to_x(self, hz: f32) -> f32 {
        let min = self.x_range.0.ln();
        let t = (hz.ln() - min) / (self.x_range.1.ln() - min);
        self.rect.left() + t * self.rect.width()
    }

    pub fn x_at(self, x: f32) -> f32 {
        let t = (x - self.rect.left()) / self.rect.width();
        (self.x_range.0.ln() + t * (self.x_range.1.ln() - self.x_range.0.ln())).exp()
    }

    pub fn db_to_y(self, db: f32) -> f32 {
        self.rect.bottom()
            - (db - self.y_range.0) / (self.y_range.1 - self.y_range.0) * self.rect.height()
    }
}
