//! The graph's vocabulary: what a curve is, how far the graph reaches, and
//! how its labels are arranged.
//!
//! These are rules of the program rather than of the picture. The interface
//! draws them; the core decides them, and the interface can be rewritten
//! without any of this changing.
use crate::color::Rgb;
use crate::data::FRPoint;
use d3::scale::{scale_linear, scale_log, ScaleLinear, ScaleLog};
use d3::shape::{curve_cardinal, curve_natural, Context};
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
    // Run the bounds in f64 (the reference's own number type) and snap to
    // the five-decibel grid the reference's axis uses.
    let bounds = curves
        .iter()
        .flat_map(|curve| curve.points.iter().map(|point| f64::from(point.db)))
        .filter(|value| value.is_finite())
        .fold(None, |bounds: Option<(f64, f64)>, value| {
            Some(match bounds {
                Some((min, max)) => (min.min(value), max.max(value)),
                None => (value, value),
            })
        });
    match bounds {
        Some((min, max)) => {
            let padding = ((max - min) * 0.08).max(3.0);
            (
                (((min - padding) / 5.0).floor() * 5.0) as f32,
                (((max + padding) / 5.0).ceil() * 5.0) as f32,
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
            if d3::ieee754::log(f64::from(frequency) / f64::from(points[lower].hz)).abs()
                <= d3::ieee754::log(f64::from(upper_point.hz) / f64::from(frequency)).abs()
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

    // `graphtool.js` switches between curveNatural and
    // curveCardinal.tension(0.5). Giving d3 a small recording context keeps
    // its endpoint handling and operation order, while the interface remains
    // free to tessellate the resulting Beziers for egui.
    let mut context = BezierControls::default();
    let mut curve = if natural {
        curve_natural()
    } else {
        curve_cardinal(0.5)
    };
    curve.line_start(&mut context);
    for point in points {
        curve.point(&mut context, f64::from(point.x), f64::from(point.y));
    }
    curve.line_end(&mut context);
    context.controls
}

/// The parts of a d3 path that egui's tessellated line needs.
///
/// d3-shape writes through a context instead of returning control points. The
/// graph only asks the two line curves for moves and cubic Beziers, so this
/// recorder deliberately preserves just those operations.
#[derive(Default)]
struct BezierControls {
    current: Option<Pos2>,
    controls: Vec<[Pos2; 4]>,
}

impl Context for BezierControls {
    fn move_to(&mut self, x: f64, y: f64) {
        self.current = Some(Pos2::new(x as f32, y as f32));
    }

    fn line_to(&mut self, x: f64, y: f64) {
        self.current = Some(Pos2::new(x as f32, y as f32));
    }

    fn close_path(&mut self) {}

    fn quadratic_curve_to(&mut self, _x1: f64, _y1: f64, x: f64, y: f64) {
        self.current = Some(Pos2::new(x as f32, y as f32));
    }

    fn bezier_curve_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) {
        let end = Pos2::new(x as f32, y as f32);
        let start = self.current.unwrap_or(end);
        self.controls.push([
            start,
            Pos2::new(x1 as f32, y1 as f32),
            Pos2::new(x2 as f32, y2 as f32),
            end,
        ]);
        self.current = Some(end);
    }

    fn arc(
        &mut self,
        _x: f64,
        _y: f64,
        _r: f64,
        _a0: f64,
        _a1: f64,
        _ccw: bool,
    ) -> Result<(), d3::path::PathError> {
        // Neither d3 curve used above emits arcs. The trait requires this
        // method because it is also shared by the shape generators that do.
        Ok(())
    }

    fn rect(&mut self, x: f64, y: f64, _width: f64, _height: f64) {
        self.current = Some(Pos2::new(x as f32, y as f32));
    }
}

pub fn natural_control_points(values: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let values: Vec<f64> = values.iter().map(|value| f64::from(*value)).collect();
    let (first, second) = d3::shape::curve::control_points(&values);
    (
        first.into_iter().map(|value| value as f32).collect(),
        second.into_iter().map(|value| value as f32).collect(),
    )
}
pub fn cubic_bezier(start: Pos2, control1: Pos2, control2: Pos2, end: Pos2, t: f32) -> Pos2 {
    // The reference evaluates the cubic in the browser's own numbers (IEEE 754
    // binary64 through V8). Run the same arithmetic here, via the port's own
    // ieee754, so the sampled curve agrees with the reference to the last bit.
    let t = f64::from(t);
    let inverse = 1.0 - t;
    let inv2 = d3::ieee754::pow(inverse, 2.0);
    let inv3 = d3::ieee754::pow(inverse, 3.0);
    let t2 = d3::ieee754::pow(t, 2.0);
    let t3 = d3::ieee754::pow(t, 3.0);
    let x = f64::from(start.x) * inv3
        + f64::from(control1.x) * (3.0 * inv2 * t)
        + f64::from(control2.x) * (3.0 * inverse * t2)
        + f64::from(end.x) * t3;
    let y = f64::from(start.y) * inv3
        + f64::from(control1.y) * (3.0 * inv2 * t)
        + f64::from(control2.y) * (3.0 * inverse * t2)
        + f64::from(end.y) * t3;
    Pos2::new(x as f32, y as f32)
}

pub fn nice_limit(peak: f32) -> f32 {
    for step in [0.5_f32, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0] {
        if peak <= step {
            return step;
        }
    }
    (peak / 50.0).ceil() * 50.0
}

/// The figure the reference writes against a tick of the decibel scale.
///
/// The reference writes the plain number — `85`, `80`, `75` — and never a
/// sign, and it never writes a decimal: the scale is marked every five
/// decibels, so every figure on it is a whole one.
pub fn tick_db(db: f32) -> String {
    if db.fract().abs() < f32::EPSILON {
        format!("{db:.0}")
    } else {
        format!("{db:.1}")
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
                (top as f64)
                    .clamp(
                        f64::from(plot_rect.top()),
                        f64::from((plot_rect.bottom() - height).max(plot_rect.top())),
                    ) as f32,
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
    // Evaluate in f64, matching the reference's number type, and clamp the
    // scale factor the same way the reference's slider does.
    let minimum = f64::from(minimum);
    let maximum = f64::from(maximum);
    let scale = f64::from(scale).clamp(0.35, 4.0);
    let center = (minimum + maximum) / 2.0;
    let half_range = (maximum - minimum) * scale / 2.0;
    ((center - half_range) as f32, (center + half_range) as f32)
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
    // Route the square root through the port's ieee754 so the hit-testing
    // distance agrees with the reference's own arithmetic.
    let sx = f64::from(end.x - start.x);
    let sy = f64::from(end.y - start.y);
    let length_squared = sx * sx + sy * sy;
    if length_squared == 0.0 {
        let dx = f64::from(point.x - start.x);
        let dy = f64::from(point.y - start.y);
        return d3::ieee754::sqrt(dx * dx + dy * dy) as f32;
    }
    let px = f64::from(point.x - start.x);
    let py = f64::from(point.y - start.y);
    let projection = (px * sx + py * sy) / length_squared;
    let projection = projection.clamp(0.0, 1.0);
    let rx = f64::from(start.x) + sx * projection;
    let ry = f64::from(start.y) + sy * projection;
    let dx = f64::from(point.x) - rx;
    let dy = f64::from(point.y) - ry;
    d3::ieee754::sqrt(dx * dx + dy * dy) as f32
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
#[derive(Clone)]
pub struct Transform {
    pub rect: Rect,
    pub x_range: (f32, f32),
    pub y_range: (f32, f32),
    x_scale: ScaleLog,
    y_scale: ScaleLinear,
}

impl Transform {
    pub fn new(rect: Rect, x_range: (f32, f32), y_range: (f32, f32)) -> Self {
        let mut x_scale = scale_log();
        x_scale
            .set_domain(&[f64::from(x_range.0), f64::from(x_range.1)])
            .set_range(&[f64::from(rect.left()), f64::from(rect.right())]);
        let mut y_scale = scale_linear();
        y_scale
            .set_domain(&[f64::from(y_range.0), f64::from(y_range.1)])
            .set_range(&[f64::from(rect.bottom()), f64::from(rect.top())]);
        Self {
            rect,
            x_range,
            y_range,
            x_scale,
            y_scale,
        }
    }

    pub fn hz_to_x(&self, hz: f32) -> f32 {
        self.x_scale.call(f64::from(hz)) as f32
    }

    pub fn x_at(&self, x: f32) -> f32 {
        self.x_scale.invert(f64::from(x)) as f32
    }

    pub fn db_to_y(&self, db: f32) -> f32 {
        self.y_scale.call(f64::from(db)) as f32
    }

    /// The level at a place on the plot, the other way about.
    pub fn y_at(&self, y: f32) -> f32 {
        self.y_scale.invert(f64::from(y)) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::{curve_control_points, Transform};
    use emath::{Pos2, Rect};

    fn assert_near(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= 0.001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn transform_uses_the_reference_d3_scales() {
        let transform = Transform::new(
            Rect::from_min_max(Pos2::new(15.0, 10.0), Pos2::new(785.0, 370.0)),
            (20.0, 20_000.0),
            (30.0, 90.0),
        );

        assert_near(transform.hz_to_x(20.0), 15.0);
        assert_near(transform.hz_to_x(200.0), 271.666_66);
        assert_near(transform.hz_to_x(2_000.0), 528.333_3);
        assert_near(transform.hz_to_x(20_000.0), 785.0);
        assert_near(transform.x_at(528.333_3), 2_000.0);
        assert_near(transform.db_to_y(30.0), 370.0);
        assert_near(transform.db_to_y(60.0), 190.0);
        assert_near(transform.db_to_y(90.0), 10.0);
        assert_near(transform.y_at(190.0), 60.0);
    }

    #[test]
    fn unsmoothed_curve_uses_d3_cardinal_tension_one_half() {
        let points = [
            Pos2::new(0.0, 0.0),
            Pos2::new(6.0, 12.0),
            Pos2::new(14.0, 4.0),
            Pos2::new(22.0, 10.0),
        ];

        let controls = curve_control_points(&points, false);
        assert_eq!(controls.len(), 3);
        assert_near(controls[0][1].x, 0.0);
        assert_near(controls[0][2].x, 4.833_333_5);
        assert_near(controls[1][1].y, 12.333_333);
        assert_near(controls[1][2].x, 12.666_667);
        assert_near(controls[2][1].x, 15.333_333);
        assert_near(controls[2][2].y, 10.0);
    }
}
