//! The graph, in the GPUI interface.
//!
//! A first port of the reference's plot: the curves that are on the graph,
//! drawn on a logarithmic frequency axis with a linear decibel axis. The
//! finer parts (grid, labels, inspector, equalizer editor) follow in later
//! milestones; this module is the seam at which they will hang.

use gpui::{Bounds, Pixels, Window, px};
use squiglab_core::d3::math::{Point as CorePoint, Rect as CoreRect};
use squiglab_core::graph::{Transform, scaled_y_range};
use squiglab_core::session::Session;

/// A snapshot of the session data the graph needs, so it can be moved into
/// a canvas paint closure.
#[derive(Clone)]
pub struct GraphData {
    pub curves: Vec<CurveSnapshot>,
    pub zoom_x: (f32, f32),
    pub y_scale: f32,
    pub color_offset: usize,
}

#[derive(Clone)]
pub struct CurveSnapshot {
    pub points: Vec<(f32, f32)>,
    pub index: usize,
    pub target: bool,
}

impl GraphData {
    pub fn from_session(session: &Session) -> Self {
        let mut curves = Vec::new();
        for (index, c) in session.display_curves.iter().enumerate() {
            curves.push(CurveSnapshot {
                points: c.points.iter().map(|p| (p.hz, p.db)).collect(),
                index,
                target: c.target,
            });
        }
        Self {
            curves,
            zoom_x: session.zoom.x_range(),
            y_scale: session.y_scale,
            color_offset: session.color_offset,
        }
    }
}

/// Draw the graph into the space the layout gives it.
pub fn show(data: &GraphData, bounds: Bounds<Pixels>, window: &mut Window) {
    if data.curves.is_empty() {
        return;
    }

    let width_px: f32 = bounds.size.width.into();
    let height_px: f32 = bounds.size.height.into();
    let origin_x: f32 = bounds.origin.x.into();
    let origin_y: f32 = bounds.origin.y.into();
    let unit = width_px / 800.0;
    let plot = CoreRect::from_min_max(
        CorePoint::new(origin_x + 15.0 * unit, origin_y + 10.0 * unit),
        CorePoint::new(
            origin_x + width_px - 15.0 * unit,
            origin_y + height_px - 36.0 * unit,
        ),
    );

    let mut mn = f32::INFINITY;
    let mut mx = f32::NEG_INFINITY;
    for c in &data.curves {
        for (_hz, db) in &c.points {
            mn = mn.min(*db);
            mx = mx.max(*db);
        }
    }
    if !mn.is_finite() {
        mn = 40.0;
    }
    if !mx.is_finite() {
        mx = 80.0;
    }
    let (minimum, maximum) = scaled_y_range((mn, mx), data.y_scale);
    let transform = Transform::new(plot, data.zoom_x, (minimum, maximum));

    for curve in &data.curves {
        if curve.points.is_empty() {
            continue;
        }
        let color = squiglab_core::color::get_curve_color_with_offset(curve.index, data.color_offset);
        let mut builder = gpui::PathBuilder::default();
        let mut opts = gpui::StrokeOptions::default();
        opts.line_width = 2.0;
        builder.style = gpui::PathStyle::Stroke(opts);
        builder.move_to(gpui::point(
            px(transform.hz_to_x(curve.points[0].0)),
            px(transform.db_to_y(curve.points[0].1)),
        ));
        for (hz, db) in &curve.points[1..] {
            builder.line_to(gpui::point(px(transform.hz_to_x(*hz)), px(transform.db_to_y(*db))));
        }
        if let Ok(path) = builder.build() {
            let color = gpui::rgb(((color.r as u32) << 16) | ((color.g as u32) << 8) | (color.b as u32));
            window.paint_path(path, color);
        }
    }
}