//! The graph.
//!
//! The geometry is the core's: where a frequency sits on the axis, how a
//! curve is smoothed, where a label goes. This is only the drawing of it, and
//! the layout rule the reference lays the graph out by: the graph is at most
//! 2.2 units across for every unit of the window's height, less the chrome
//! around it, and it is centred in its column.

use std::sync::Arc;

use egui::{Align2, Color32, FontId, Galley, Pos2, Rect, Sense, Stroke, Vec2};

use squiglab_core::graph::{
    format_axis_frequency, format_frequency, nice_limit, place_labels, scaled_y_range, tick_db,
    y_range, PlotCurve, Transform, FREQUENCY_BANDS,
};

use crate::app::App;
use crate::theme::{self, metrics, TypeStep};

/// The frequencies the axis is marked at, taken from the reference's own
/// `d3.merge([1,2,3].map(e => xvals.map(m => m * Math.pow(10, e)))).concat([20000])`
/// with `xvals = [2,3,4,5,6,8,10,15]`, so the native's grid agrees with the
/// reference's down to the last tick.
fn x_tick_frequencies() -> Vec<f32> {
    let xvals = [2.0_f64, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0, 15.0];
    let mut lists: Vec<Vec<f64>> = Vec::new();
    for e in [1_i32, 2, 3] {
        let e = e as f64;
        lists.push(
            xvals
                .iter()
                .map(|m| m * d3::ieee754::pow(10.0, e))
                .collect(),
        );
    }
    let mut ticks = d3::array::merge(&lists);
    ticks.push(20000.0);
    ticks.into_iter().map(|v| v as f32).collect()
}

/// The graph, in the middle of the window.
///
/// Rewrite entry point: the rendering is being ported to d3-driven graphics
/// piece by piece. Each section below is replaced in turn; until then it
/// delegates to the legacy implementation so the app keeps working.
pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let palette = theme::current_palette();
    let window_height = ui.ctx().content_rect().height();
    let by_height =
        (window_height - metrics::GRAPH_CHROME_HEIGHT).max(120.0) * metrics::GRAPH_ASPECT;
    let height = (ui.available_height() - 40.0)
        .min(by_height / metrics::GRAPH_ASPECT)
        .max(160.0);
    let width = (height * metrics::PLOT_ASPECT_RATIO).min(ui.available_width());
    let height = width / metrics::PLOT_ASPECT_RATIO;

    let available = ui.available_width();
    let ctx = ui.ctx().clone();
    ui.horizontal(|ui| {
        ui.add_space((available - width).max(0.0) / 2.0);
        let (response, painter) =
            ui.allocate_painter(Vec2::new(width, height), Sense::click_and_drag());
        let rect = response.rect;
        let mut path = d3::path::Path::default();
        path.rect(
            f64::from(rect.left()),
            f64::from(rect.top()),
            f64::from(rect.width()),
            f64::from(rect.height()),
        );
        let shapes = svg_tessellate::tessellate(
            &path.to_string(),
            None,
            Some(palette.graph),
            svg_tessellate::Options::default(),
        );
        for shape in shapes {
            painter.add(shape);
        }
        let plot = plot_rect(rect);
        // dB scale drag handle first, so the axis labels draw above it.
        scale(app, &painter, rect, &response);
        let curves = plot_curves(app);
        let (minimum, maximum) = match app.session.graph_range {
            Some(range) => range,
            None => {
                let (minimum, maximum) = y_range(&curves);
                scaled_y_range((minimum, maximum), app.session.y_scale)
            }
        };
        let transform = transform_for(app, plot);

        // axes (d3-backed)
        grid(&painter, rect, plot, &transform, minimum, maximum);
        // curves, labels, inspector, rig line, watermark
        render_body(
            &painter,
            app,
            rect,
            plot,
            &transform,
            &curves,
            response.hover_pos(),
        );
        // equalizer editor
        eq_editor(app, &painter, plot, &transform, &response, &ctx);
    });
}

/// The plot's own area, inside the box.
///
/// The reference lays its graph out in an 800-by-360 viewBox with fifteen
/// units of padding at the sides, ten at the top and thirty-six at the
/// bottom, and the box is drawn at the window's scale. The same fractions
/// of this box are used here, so the plot sits where the reference's does.
fn plot_rect(rect: Rect) -> Rect {
    let unit = rect.width() / 800.0;
    Rect::from_min_max(
        Pos2::new(rect.left() + 15.0 * unit, rect.top() + 10.0 * unit),
        Pos2::new(rect.right() - 15.0 * unit, rect.bottom() - 36.0 * unit),
    )
}

/// The way from the plot back to the curves.
fn transform_for(app: &App, plot: Rect) -> Transform {
    let curves = plot_curves(app);
    let (minimum, maximum) = match app.session.graph_range {
        Some(range) => range,
        None => {
            let (minimum, maximum) = y_range(&curves);
            scaled_y_range((minimum, maximum), app.session.y_scale)
        }
    };
    let zoom = app.session.zoom;
    Transform::new(plot, zoom.x_range(), (minimum, maximum))
}

/// The scale down the left of the graph, and the handles that move it.
///
/// The reference's scale is dragged: the handle in the middle moves the
/// range up and down, and the two ends change how far it reaches. A
/// double-click puts it back where the curves say it should be.
fn scale(app: &mut App, painter: &egui::Painter, rect: Rect, response: &egui::Response) {
    let palette = theme::current_palette();
    let plot = plot_rect(rect);
    if plot.width() <= 0.0 || plot.height() <= 0.0 {
        return;
    }
    let (minimum, maximum) = match app.session.graph_range {
        Some(range) => range,
        None => {
            let curves = plot_curves(app);
            let (minimum, maximum) = y_range(&curves);
            scaled_y_range((minimum, maximum), app.session.y_scale)
        }
    };
    let centre_db = (minimum + maximum) / 2.0;
    let half_db = (maximum - minimum) / 2.0;

    // Where the reference puts the handle: its origin sits nine units left
    // of the plot's edge, on the 60 dB line, and it reaches ten dB either
    // way. Its own path is in viewBox units, scaled by the graph's unit.
    let transform = Transform::new(plot, app.session.zoom.x_range(), (minimum, maximum));
    let unit = rect.width() / 800.0;
    let origin_x = rect.left() + 6.0 * unit; // pad.l − 9 = 6
    let origin_y = transform.db_to_y(60.0);
    let half_px = transform.db_to_y(60.0) - transform.db_to_y(70.0); // > 0

    scale_shape(painter, origin_x, origin_y, half_px, unit, &palette);

    // Dragging the middle mark moves the range; dragging an end widens it.
    // The scale is only taken hold of at the scale itself: the rest of the
    // graph belongs to the equalizer, as it does in the reference.
    let on_the_scale = response
        .hover_pos()
        .is_some_and(|point| (point.x - origin_x).abs() <= 12.0 * unit);
    if response.drag_started() && on_the_scale {
        // Where the pointer took hold, which is what tells the mark from the
        // ends once the pointer has moved.
        app.scale_grab = response.hover_pos();
    }
    if response.dragged() && app.scale_grab.is_some() {
        let delta = response.drag_delta().y;
        if delta != 0.0 {
            let per_pixel = (maximum - minimum) / plot.height();
            let range;
            // The end the pointer took hold of: the one above the middle
            // or the one below it, which reach in opposite directions.
            let end = app
                .scale_grab
                .filter(|grab| (grab.y - origin_y).abs() > half_px.abs() * 0.6);
            if let Some(grab) = end {
                // The pointer is on an end: the scale reaches further or less.
                let sign = if grab.y < origin_y { -1.0 } else { 1.0 };
                let new_half = (half_db + sign * delta * per_pixel).abs().max(1.0);
                range = (centre_db - new_half, centre_db + new_half);
            } else {
                // The pointer is on the mark: the whole range moves.
                let shift = -delta * per_pixel;
                range = (minimum + shift, maximum + shift);
            }
            app.session.graph_range = Some(range);
        }
    }
    if response.double_clicked() {
        app.session.graph_range = None;
    }
}

/// The reference's scale handle: the two mirrored paths and the three
/// grips, drawn at the size the graph's unit gives.
///
/// The path is the reference's own, in viewBox units: a spine at x=9.9
/// running from y=−1 to y=dB.H, a foot at the far end, and a curve back
/// to x=−1.8. dB.H is the ten dB from 60 to 70, which the graph's unit
/// turns into pixels. The grips are the three short marks at the middle,
/// cut in the graph's own colour.
fn scale_shape(
    painter: &egui::Painter,
    origin_x: f32,
    origin_y: f32,
    half_px: f32,
    unit: f32,
    palette: &theme::Palette,
) {
    // The reference's own path, drawn once for each sign: one half above
    // the middle, one below it. dB.H is the half-height in pixels; the
    // rest are fixed offsets the reference writes into its path.
    for sign in [1.0, -1.0] {
        let mut path = d3::path::Path::default();
        let mut x = origin_x + 9.9 * unit;
        let mut y = origin_y - 1.0 * unit * sign;
        path.move_to(f64::from(x), f64::from(y));
        // The spine, down to the far end of the handle.
        y = origin_y + half_px * sign;
        path.line_to(f64::from(x), f64::from(y));
        // The foot: a step, a rise, a fall, a run, and a small drop.
        x -= 1.0 * unit;
        path.line_to(f64::from(x), f64::from(y));
        x -= 1.0 * unit;
        y -= 1.5 * unit * sign;
        path.line_to(f64::from(x), f64::from(y));
        x -= 2.1 * unit;
        y += 2.0 * unit * sign;
        path.line_to(f64::from(x), f64::from(y));
        x -= 5.6 * unit;
        path.line_to(f64::from(x), f64::from(y));
        y -= 1.5 * unit * sign;
        path.line_to(f64::from(x), f64::from(y));
        // The reference's own curve back toward the spine.
        let cx = x + 7.0 * unit;
        let cy = y + 2.0 * unit * sign;
        x += 8.0 * unit;
        y -= 7.0 * unit * sign;
        path.quadratic_curve_to(f64::from(cx), f64::from(cy), f64::from(x), f64::from(y));
        // Up the inner edge to the bracket, then the curve to the far side.
        y = origin_y + 29.0 * unit * sign;
        path.line_to(f64::from(x), f64::from(y));
        let c1x = x + 1.0 * unit;
        let c1y = y - 16.0 * unit * sign;
        let c2x = x - 10.0 * unit;
        let c2y = y - 15.0 * unit * sign;
        x -= 10.0 * unit;
        y -= 14.0 * unit * sign;
        path.bezier_curve_to(
            f64::from(c1x),
            f64::from(c1y),
            f64::from(c2x),
            f64::from(c2y),
            f64::from(x),
            f64::from(y),
        );
        // Back to the top of the spine, which closes the shape.
        y = origin_y - 1.0 * unit * sign;
        path.line_to(f64::from(x), f64::from(y));
        path.close_path();

        let shapes = svg_tessellate::tessellate(
            &path.to_string(),
            None,
            Some(palette.border_strong),
            svg_tessellate::Options::default(),
        );
        for shape in shapes {
            painter.add(shape);
        }
    }

    // The three grips at the middle, cut in the graph's own colour. The
    // reference's own heights: ten, seven and thirteen units.
    for (i, d) in [10.0_f32, 7.0, 13.0].iter().enumerate() {
        let grip = Rect::from_min_size(
            Pos2::new(origin_x + i as f32 * 2.8 * unit, origin_y - d * unit),
            Vec2::new(0.8 * unit, 2.0 * d * unit),
        );
        painter.rect_filled(grip, 0.0, palette.graph);
    }
}

/// Draw the graph into a rectangle, with the pointer where it is.
fn render_body(
    painter: &egui::Painter,
    app: &App,
    rect: Rect,
    plot: Rect,
    transform: &Transform,
    curves: &[PlotCurve<'_>],
    pointer: Option<Pos2>,
) {
    band_names(painter, plot, transform);
    curves_into(painter, app, curves, transform);
    if app.session.inspect_mode {
        inspector(painter, app, curves, transform, plot, pointer);
    }
    labels(painter, app, curves, transform, plot);
    rig_line(painter, app, rect);
    watermark(painter, plot);
}

/// The colour a measurement is drawn in.
///
/// The palette is turned in order, so a curve keeps its colour for as long as
/// it is on the graph, and the manager row and the graph agree.
pub fn curve_color(app: &App, group_index: usize) -> squiglab_core::color::Rgb {
    let position = app
        .session
        .display_curves
        .iter()
        .position(|curve| {
            curve.group_index == group_index
                && matches!(
                    curve.source,
                    squiglab_core::session::CurveSource::Measurement
                )
        })
        .unwrap_or(group_index);
    squiglab_core::color::get_curve_color_with_offset(position, app.session.color_offset)
}

/// The curves the graph draws: the measurements and the targets that are on.
fn plot_curves(app: &App) -> Vec<PlotCurve<'_>> {
    let mut curves = Vec::new();
    for (index, curve) in app.session.display_curves.iter().enumerate() {
        let color = match curve.source {
            squiglab_core::session::CurveSource::Equalizer => {
                squiglab_core::color::get_curve_color_with_offset(index, app.session.color_offset)
            }
            _ => squiglab_core::color::get_curve_color_with_offset(index, app.session.color_offset),
        };
        curves.push(PlotCurve {
            label: &curve.label,
            label_key: &curve.label,
            points: &curve.points,
            color,
            dashed: curve.target,
            target: curve.target,
        });
    }
    curves
}

/// The bands of the curve that is being edited, and where they sit.
struct Marker {
    /// The row in the bank.
    row: usize,
    /// The frequency, in hertz.
    hz: f64,
    /// The response there, in decibels.
    db: f64,
}

/// The way a drag on the graph is locked.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// The frequency, and the gain stays where it was.
    Frequency,
    /// The gain, and the frequency stays where it was.
    Gain,
}

/// What the pointer is doing on the graph.
pub enum GraphDrag {
    /// A band is being moved, or a new one is being drawn.
    Band {
        /// The row being moved; none when a new band is being drawn.
        row: Option<usize>,
        /// The way the drag is locked.
        axis: Option<Axis>,
        /// The frequency and the gain the band began at.
        anchor_hz: f64,
        anchor_db: f64,
        /// Where the pointer began.
        origin: Pos2,
    },
    /// A range of frequencies is being brushed, for the tone generator.
    Range {
        /// The frequency the brush began at.
        anchor_hz: f64,
        /// The frequency it has reached.
        last_hz: f64,
    },
}

/// How far the pointer may be from a band and still be on it, in points.
const MARKER_HIT_PX: f32 = 28.0;
/// The radius a band's mark is drawn at, in points.
const MARKER_RADIUS: f32 = 2.5;
/// How far the pointer must move before the drag is locked to one way.
const DRAG_THRESHOLD_PX: f32 = 5.0;

/// The equalizer's own graph: the bands, and what the pointer does to them.
fn eq_editor(
    app: &mut App,
    painter: &egui::Painter,
    plot: Rect,
    transform: &Transform,
    response: &egui::Response,
    ctx: &egui::Context,
) {
    // The reference's graph is an editor only while the equalizer is open.
    if app.tab != crate::app::Tab::Equalizer {
        return;
    }
    let palette = theme::current_palette();
    let markers = eq_markers(app, transform);
    let pointer = response.hover_pos().filter(|point| plot.contains(*point));

    // The range being brushed, which the reference shows on the plot while
    // it is being brushed.
    if let Some(GraphDrag::Range { anchor_hz, last_hz }) = &app.graph_drag {
        let (low, high) = if anchor_hz <= last_hz {
            (*anchor_hz, *last_hz)
        } else {
            (*last_hz, *anchor_hz)
        };
        let left = transform.hz_to_x(low as f32);
        let right = transform.hz_to_x(high as f32);
        let mut path = d3::path::Path::default();
        path.rect(
            f64::from(left),
            f64::from(plot.top()),
            f64::from(right - left),
            f64::from(plot.bottom() - plot.top()),
        );
        let shapes = svg_tessellate::tessellate(
            &path.to_string(),
            None,
            Some(palette.accent.gamma_multiply(0.18)),
            svg_tessellate::Options::default(),
        );
        for shape in shapes {
            painter.add(shape);
        }
    }

    // The marks, drawn as d3 arc paths tessellated by svg-tessellate.
    for marker in &markers {
        let centre = Pos2::new(
            transform.hz_to_x(marker.hz as f32),
            transform.db_to_y(marker.db as f32),
        );
        let hot = pointer.is_some_and(|point| (point - centre).length() <= MARKER_HIT_PX);
        let radius = if hot {
            MARKER_RADIUS * 1.8
        } else {
            MARKER_RADIUS
        };
        let color = theme::color(crate::graph_port::curve_color(
            app,
            app.session.eq_phone.unwrap_or(0),
        ));
        let r = radius as f64;
        let cx = centre.x as f64;
        let cy = centre.y as f64;
        let path_data = format!(
            "M {},{} A {},{} 0 1,1 {},{} A {},{} 0 1,1 {},{} Z",
            cx - r,
            cy,
            r,
            r,
            cx + r,
            cy,
            r,
            r,
            cx - r,
            cy
        );
        let shapes = svg_tessellate::tessellate(
            &path_data,
            Some(Stroke::new(1.4, color)),
            Some(palette.graph),
            svg_tessellate::Options::default(),
        );
        for shape in shapes {
            painter.add(shape);
        }
    }

    // The wheel over a band changes its width, as the reference's does.
    if let Some(pointer) = pointer {
        let wheel = ctx.input(|input| input.smooth_scroll_delta.y);
        if wheel != 0.0 {
            if let Some(marker) = markers.iter().find(|marker| {
                (pointer - marker_centre(marker, transform)).length() <= MARKER_HIT_PX
            }) {
                let current = band_q(app, marker.row);
                let fine = current <= 0.3;
                let step: f32 = if fine { 0.01 } else { 0.1 };
                let sensitivity: f32 = if current >= 2.0 {
                    1.0
                } else {
                    let t = ((current as f32 - 0.1) / (2.0 - 0.1)).clamp(0.0, 1.0);
                    0.22 + 0.78 * t.powf(1.2)
                };
                let notches = (wheel.abs() / 42.0_f32).powf(0.58).round().clamp(1.0, 5.0);
                let change = -wheel.signum() * step * notches * sensitivity;
                crate::equalizer::set_band_q(app, marker.row, current + change as f64);
            }
        }
    }

    // The right button on a band turns its kind round, as the reference's does.
    if ctx.input(|input| input.pointer.button_clicked(egui::PointerButton::Secondary)) {
        if let Some(pointer) = pointer {
            if let Some(marker) = markers.iter().find(|marker| {
                (pointer - marker_centre(marker, transform)).length() <= MARKER_HIT_PX
            }) {
                crate::equalizer::cycle_band_type(app, marker.row);
            }
        }
    }

    // The drag: a band is moved, or drawn, or a range of frequencies is
    // brushed for the tone generator.
    if response.drag_started() {
        if let Some(pointer) = response.interact_pointer_pos() {
            if plot.contains(pointer) {
                let hit = markers.iter().find(|marker| {
                    (pointer - marker_centre(marker, transform)).length() <= MARKER_HIT_PX
                });
                match hit {
                    Some(marker) => {
                        app.graph_drag = Some(GraphDrag::Band {
                            row: Some(marker.row),
                            axis: None,
                            anchor_hz: marker.hz,
                            anchor_db: marker.db,
                            origin: pointer,
                        });
                    }
                    None => {
                        let hz = frequency_at(transform, pointer);
                        if near_the_trace(app, transform, pointer) {
                            // A band is drawn out of the curve, at the
                            // height the pointer is at above it.
                            let gain = gain_under(app, transform, pointer);
                            let row = crate::equalizer::add_band_at(app, hz, gain);
                            app.graph_drag = Some(GraphDrag::Band {
                                row,
                                axis: None,
                                anchor_hz: hz,
                                anchor_db: gain,
                                origin: pointer,
                            });
                        } else {
                            app.graph_drag = Some(GraphDrag::Range {
                                anchor_hz: hz,
                                last_hz: hz,
                            });
                        }
                    }
                }
            }
        }
    }
    if response.dragged() {
        let pointer = response.interact_pointer_pos();
        // The drag is taken out of the program while it is worked on, so that
        // the band can be written back without borrowing it twice.
        match app.graph_drag.take() {
            Some(GraphDrag::Band {
                row,
                axis,
                anchor_hz,
                anchor_db,
                origin,
            }) => {
                let mut axis = axis;
                let (mut hz, mut db) = (anchor_hz, anchor_db);
                if let Some(pointer) = pointer {
                    if axis.is_none() {
                        let dx = (pointer.x - origin.x).abs();
                        let dy = (pointer.y - origin.y).abs();
                        if dx.max(dy) >= DRAG_THRESHOLD_PX {
                            axis = Some(if dx >= dy {
                                Axis::Frequency
                            } else {
                                Axis::Gain
                            });
                        }
                    }
                    if axis == Some(Axis::Frequency) {
                        hz = frequency_at(transform, pointer);
                    }
                    if axis == Some(Axis::Gain) {
                        db = gain_under(app, transform, pointer);
                    }
                    if let Some(row) = row {
                        crate::equalizer::move_band(app, row, hz, db);
                    }
                }
                app.graph_drag = Some(GraphDrag::Band {
                    row,
                    axis,
                    anchor_hz,
                    anchor_db,
                    origin,
                });
            }
            Some(GraphDrag::Range { anchor_hz, last_hz }) => {
                let mut last_hz = last_hz;
                if let Some(pointer) = pointer {
                    last_hz = frequency_at(transform, pointer);
                    let (low, high) = if anchor_hz <= last_hz {
                        (anchor_hz, last_hz)
                    } else {
                        (last_hz, anchor_hz)
                    };
                    app.tone_band = (
                        low.clamp(20.0, 20_000.0) as f32,
                        high.clamp(20.0, 20_000.0) as f32,
                    );
                }
                app.graph_drag = Some(GraphDrag::Range { anchor_hz, last_hz });
            }
            None => {}
        }
    }
    if response.drag_stopped() {
        app.graph_drag = None;
    }
}

/// Where a band's mark sits.
fn marker_centre(marker: &Marker, transform: &Transform) -> Pos2 {
    Pos2::new(
        transform.hz_to_x(marker.hz as f32),
        transform.db_to_y(marker.db as f32),
    )
}

/// The frequency under the pointer.
fn frequency_at(transform: &Transform, pointer: Pos2) -> f64 {
    f64::from(transform.x_at(pointer.x)).clamp(20.0, 20_000.0)
}

/// The level under the pointer, in decibels.
fn level_at(transform: &Transform, pointer: Pos2) -> f64 {
    f64::from(transform.y_at(pointer.y))
}

/// How far above the curve the pointer is, in decibels.
fn gain_under(app: &App, transform: &Transform, pointer: Pos2) -> f64 {
    let level = level_at(transform, pointer);
    let Some(phone) = app.session.eq_phone else {
        return level;
    };
    let Some(curve) = app.session.display_curves.iter().find(|curve| {
        curve.group_index == phone
            && matches!(
                curve.source,
                squiglab_core::session::CurveSource::Measurement
            )
    }) else {
        return level;
    };
    let hz = frequency_at(transform, pointer) as f32;
    let trace = f64::from(db_at(&curve.points, f64::from(hz)));
    (level - trace).clamp(-40.0, 40.0)
}

/// The bands of the curve that is being edited.
fn eq_markers(app: &App, transform: &Transform) -> Vec<Marker> {
    let Some(phone) = app.session.eq_phone else {
        return Vec::new();
    };
    // The measurement the bands sit on, so that a mark is where the curve is.
    let measurement = app
        .session
        .display_curves
        .iter()
        .find(|curve| {
            curve.group_index == phone
                && matches!(
                    curve.source,
                    squiglab_core::session::CurveSource::Measurement
                )
        })
        .map(|curve| &curve.points);
    crate::equalizer::bank_rows(app)
        .into_iter()
        .filter(|(_, row)| row.enabled)
        .map(|(index, band)| {
            let hz = band.filter.freq;
            // The mark sits where the curve is, with the band's own gain
            // added to it: a shelf is drawn as the peak it would be, so that
            // turning a band into a shelf does not move its mark.
            let db = measurement
                .map(|points| db_at(points, hz) as f64 + band.filter.gain)
                .unwrap_or(band.filter.gain);
            Marker { row: index, hz, db }
        })
        .filter(|marker| {
            let x = transform.hz_to_x(marker.hz as f32);
            let y = transform.db_to_y(marker.db as f32);
            x.is_finite() && y.is_finite()
        })
        .collect()
}

/// The level of a curve at a frequency.
fn db_at(points: &[squiglab_core::data::FRPoint], hz: f64) -> f32 {
    if points.is_empty() {
        return 0.0;
    }
    for window in points.windows(2) {
        let (left, right) = (&window[0], &window[1]);
        if hz >= f64::from(left.hz) && hz <= f64::from(right.hz) {
            let span = f64::from(right.hz) - f64::from(left.hz);
            if span <= 0.0 {
                return left.db;
            }
            let place = (hz - f64::from(left.hz)) / span;
            return (f64::from(left.db) + (f64::from(right.db) - f64::from(left.db)) * place)
                as f32;
        }
    }
    if hz < f64::from(points[0].hz) {
        points[0].db
    } else {
        points[points.len() - 1].db
    }
}

/// The width of a band.
fn band_q(app: &App, row: usize) -> f64 {
    crate::equalizer::bank_rows(app)
        .into_iter()
        .find(|(index, _)| *index == row)
        .map(|(_, row)| row.filter.q)
        .unwrap_or(1.0)
}

/// Whether the pointer is near the curve that is being edited.
///
/// The reference measures it at the pointer's own frequency: the distance
/// from the pointer down to the curve, and no more than a mark's reach.
fn near_the_trace(app: &App, transform: &Transform, pointer: Pos2) -> bool {
    let Some(phone) = app.session.eq_phone else {
        return false;
    };
    let Some(curve) = app.session.display_curves.iter().find(|curve| {
        curve.group_index == phone
            && matches!(
                curve.source,
                squiglab_core::session::CurveSource::Measurement
            )
    }) else {
        return false;
    };
    let hz = frequency_at(transform, pointer) as f32;
    let db = db_at(&curve.points, f64::from(hz));
    let y = transform.db_to_y(db);
    (y - pointer.y).abs() <= MARKER_HIT_PX
}

/// The reference's own weights for the ticks of the frequency axis.
///
/// `tickPattern` cycles along the ticks and gives each one a kind;
/// `tickThickness` turns a kind into a width, and the first and last
/// ticks — the ends of the range — take the heaviest of them. The same
/// pattern decides which figures are plain: the reference writes those
/// at eighty-six per cent and its lightest weight.
const X_TICK_PATTERN: [usize; 8] = [3, 0, 0, 1, 0, 0, 2, 0];
const X_TICK_THICKNESS: [f32; 5] = [0.2, 0.4, 0.4, 0.9, 1.5];

/// The blur the reference lays over every major decibel line.
///
/// Its stylesheet has no filter of its own: the graph's script puts a
/// Gaussian of 0.8 units over each line, cut off two units either side.
/// The ink of a fifth of a unit ends as a soft band four units wide and
/// about an eighth as strong at the heart. egui's painter has no filter,
/// so three strokes of falling width stand in for it; their alphas are
/// the Gaussian's profile read at their own edges.
const DB_LINE_BLUR: [(f32, f32); 3] = [(1.2, 0.075), (2.4, 0.045), (4.0, 0.015)];

/// Lay a figure out in the face the graph writes its numbers in.
fn figure_galley(
    painter: &egui::Painter,
    text: &str,
    font: &FontId,
    colour: Color32,
) -> Arc<Galley> {
    painter
        .ctx()
        .fonts_mut(|fonts| fonts.layout_no_wrap(text.to_owned(), font.clone(), colour))
}

/// Where the baseline of a figure's first row falls, measured down from
/// the top of its galley.
///
/// SVG writes text by the baseline and egui by the top of the box, so
/// every figure that has to stand on a line of its own needs this.
fn row_baseline(galley: &Galley) -> f32 {
    galley.rows.first().map_or(0.0, |row| {
        row.pos.y + row.glyphs.first().map_or(0.0, |glyph| glyph.pos.y)
    })
}

/// A figure written with its baseline on `baseline` and its left edge at
/// `left`, which is how the reference's SVG places every axis number.
fn figure_at(
    painter: &egui::Painter,
    galley: Arc<Galley>,
    colour: Color32,
    left: f32,
    baseline: f32,
) {
    let top = baseline - row_baseline(&galley);
    painter.add(egui::epaint::TextShape::new(
        Pos2::new(left, top),
        galley,
        colour,
    ));
}

/// A d3 path drawn as the stroke it is named with.
fn stroke_path(painter: &egui::Painter, path: &d3::path::Path, stroke: Stroke) {
    let shapes = svg_tessellate::tessellate(
        &path.to_string(),
        Some(stroke),
        None,
        svg_tessellate::Options::default(),
    );
    for shape in shapes {
        painter.add(shape);
    }
}

/// The grid, the dB scale and the frequency axis.
///
/// Everything here is the reference's own: the tick values come from
/// `fmtY` and `fmtX` in graphtool.js, the figures are its monospaced ten
/// — its eight and a half for the plain ticks below the frequency axis —
/// written by the baseline as SVG writes them, and the lines carry the
/// colour its stylesheet gives every `svg line`, which the `#333` the
/// script sets on the frequency ticks does not outlive.
fn grid(
    painter: &egui::Painter,
    rect: Rect,
    plot: Rect,
    transform: &Transform,
    minimum: f32,
    maximum: f32,
) {
    let palette = theme::current_palette();
    // The reference draws its graph in an 800-unit viewBox at the size of
    // the box, so a figure of ten units stands ten times this tall.
    let unit = rect.width() / 800.0;
    let font = FontId::new(10.0 * unit, egui::FontFamily::Monospace);
    let minor_font = FontId::new(8.6 * unit, egui::FontFamily::Monospace);
    let colour = palette.text_primary;
    let line = palette.border_strong;

    // The decibel scale. The step is one when the range is narrow and
    // five otherwise; the ticks run from the ceiling of the lower bound
    // to the floor of the upper one.
    let range = (maximum - minimum).max(1.0);
    let step = if range < 40.0 { 1.0_f32 } else { 5.0_f32 };
    let y0 = (minimum / step).ceil();
    let y1 = (maximum / step).floor();
    // The reference's own step: `d3.range(y1-y0+1).map(i => t*(y0+i))`.
    // The range steps by one tick index at a time and the decibel value
    // is taken afterwards; stepping it in decibels would skip four
    // fifths of the ticks whenever the step is five, leaving the axis
    // with one number.
    let ticks: Vec<f32> = d3::array::range(f64::from(y0), Some(f64::from(y1) + 1.0), Some(1.0))
        .into_iter()
        .map(|v| (v * f64::from(step)) as f32)
        .collect();
    // One path per band of the blur, so that the whole blurred scale is
    // three tessellations, and one for the minor lines, which the
    // reference leaves sharp.
    let mut bands = [
        d3::path::Path::default(),
        d3::path::Path::default(),
        d3::path::Path::default(),
    ];
    let mut minor_path = d3::path::Path::default();
    for db in ticks {
        let y = transform.db_to_y(db);
        if y < plot.top() || y > plot.bottom() {
            continue;
        }
        // A tick is minor when the step is one and its figure is not a
        // multiple of five; the reference hides those figures altogether
        // and draws only their line, thin and sharp.
        let minor = step == 1.0 && (db as i32) % 5 != 0;
        let across = |path: &mut d3::path::Path| {
            path.move_to(f64::from(plot.left()), f64::from(y));
            path.line_to(f64::from(plot.right()), f64::from(y));
        };
        if minor {
            across(&mut minor_path);
            continue;
        }
        for band in &mut bands {
            across(band);
        }
        // The figure: its left edge three units in from the plot's side,
        // its baseline two above the line it names.
        let galley = figure_galley(painter, &tick_db(db), &font, colour);
        figure_at(
            painter,
            galley,
            colour,
            plot.left() + 3.0 * unit,
            y - 2.0 * unit,
        );
    }
    for (band, (width, alpha)) in bands.iter().zip(DB_LINE_BLUR) {
        stroke_path(
            painter,
            band,
            Stroke::new(width * unit, line.gamma_multiply(alpha)),
        );
    }
    // The minor lines are `0.2*(1-r/45)`, the reference's own figure.
    stroke_path(
        painter,
        &minor_path,
        Stroke::new(0.2 * (1.0 - range / 45.0) * unit, line),
    );

    // The "dB" the reference writes up the left of the scale: its end on
    // the plot's top-left corner, reading upward.
    rotated_label(
        painter,
        "dB",
        Pos2::new(plot.left(), plot.top()),
        font.clone(),
        colour,
    );

    // The frequency axis. The reference's ticks run from ten units below
    // the top of the plot to two above its foot, each weighted by its
    // place in the pattern, and each carries its figure centred on it:
    // the plain ones written small, the ends nudged in from the range.
    let frequencies = x_tick_frequencies();
    let last = frequencies.len() - 1;
    // One path per width among the ticks: four tessellations for the
    // whole axis.
    let mut widths = [
        d3::path::Path::default(),
        d3::path::Path::default(),
        d3::path::Path::default(),
        d3::path::Path::default(),
    ];
    let top = plot.top() + 10.0 * unit;
    let foot = plot.bottom() - 2.0 * unit;
    for (i, hz) in frequencies.iter().enumerate() {
        if *hz < transform.x_range.0 || *hz > transform.x_range.1 {
            continue;
        }
        let x = transform.hz_to_x(*hz);
        // The ends of the range take the heaviest weight; the rest come
        // from the pattern, whose two middle kinds share a width.
        let kind = if i == 0 || i == last {
            4
        } else {
            X_TICK_PATTERN[i % 8]
        };
        let band = match kind {
            0 => 0,
            1 | 2 => 1,
            3 => 2,
            _ => 3,
        };
        widths[band].move_to(f64::from(x), f64::from(top));
        widths[band].line_to(f64::from(x), f64::from(foot));
        // The figure: the plain ticks of the pattern stand eighty-six per
        // cent tall, and the two ends are nudged in from the range.
        let plain = X_TICK_PATTERN[i % 8] == 0;
        let (face, units) = if plain {
            (&minor_font, 8.6)
        } else {
            (&font, 10.0)
        };
        let nudge = if i == 0 {
            4.0
        } else if i == last {
            -5.0
        } else {
            0.0
        };
        let galley = figure_galley(painter, &format_axis_frequency(*hz), face, colour);
        let left = x + nudge * unit - galley.size().x / 2.0;
        // The reference sets its tick text at the foot of a line of
        // H+3+3 units and drops the baseline seven-tenths of an em
        // further, where H is the plot's own height.
        let baseline = plot.bottom() + (6.0 + 0.71 * units) * unit;
        figure_at(painter, galley, colour, left, baseline);
    }
    let band_widths = [
        X_TICK_THICKNESS[0],
        X_TICK_THICKNESS[1],
        X_TICK_THICKNESS[3],
        X_TICK_THICKNESS[4],
    ];
    for (path, width) in widths.iter().zip(band_widths) {
        stroke_path(painter, path, Stroke::new(width * unit, line));
    }
}

/// A word written on its side, the way the reference writes the unit of
/// the scale up the left of the graph.
///
/// The reference turns the word a quarter turn anticlockwise about its
/// own end and plants that end on the corner it is given, so the word
/// reads upward with its last letter at the top and its body hanging
/// into the gutter beside the scale.
fn rotated_label(painter: &egui::Painter, text: &str, corner: Pos2, font: FontId, colour: Color32) {
    let galley = figure_galley(painter, text, &font, colour);
    // The pivot is the word's own end, which the corner holds: egui turns
    // a shape about the point its anchor names, so the galley is set down
    // with its foot at the pivot and then turned.
    let pivot = Pos2::new(corner.x, corner.y + galley.size().x);
    let shape = egui::epaint::TextShape::new(
        Pos2::new(pivot.x, pivot.y - galley.size().y),
        galley,
        colour,
    )
    .with_angle_and_anchor(-std::f32::consts::FRAC_PI_2, egui::Align2::LEFT_BOTTOM);
    painter.add(shape);
}

/// The names the reference gives the bands of the axis.
fn band_names(painter: &egui::Painter, plot: Rect, transform: &Transform) {
    let palette = theme::current_palette();
    let font = FontId::new(TypeStep::Label.size(), egui::FontFamily::Proportional);
    // The names sit below the frequency figures, near the foot of the box,
    // so the two rows of small type do not crowd one another.
    let unit = plot.width() / 770.0;
    let top = plot.bottom() + 22.0 * unit;
    for band in FREQUENCY_BANDS {
        let (low, high) = band.range_hz;
        if high < transform.x_range.0 || low > transform.x_range.1 {
            continue;
        }
        let start = transform.hz_to_x(low.max(transform.x_range.0));
        let end = transform.hz_to_x(high.min(transform.x_range.1));
        // A band only earns its name if the band is wider than the name.
        let width = end - start;
        let needed = band.title.len() as f32 * TypeStep::Label.size() * 0.55;
        if width < needed {
            continue;
        }
        painter.text(
            Pos2::new((start + end) / 2.0, top),
            Align2::CENTER_TOP,
            band.title,
            font.clone(),
            palette.text_tertiary,
        );
    }
}

/// The curves themselves.
fn curves_into(
    painter: &egui::Painter,
    app: &App,
    curves: &[PlotCurve<'_>],
    transform: &Transform,
) {
    // The reference draws its traces at a stroke of 2.3 in an 800-unit box,
    // and, having no `non-scaling-stroke`, lets that scale with the figure.
    // One unit of that box is this many points.
    let unit = transform.rect.width() / 770.0;
    for curve in curves {
        if curve.points.is_empty() {
            continue;
        }
        let points: Vec<[f64; 2]> = curve
            .points
            .iter()
            .map(|point| {
                [
                    f64::from(transform.hz_to_x(point.hz)),
                    f64::from(transform.db_to_y(point.db)),
                ]
            })
            .collect();
        let color = theme::color(curve.color);
        if curve.dashed {
            let smooth = app.session.smooth_level > 0;
            let mut path = d3::path::Path::default();
            let mut curve_shape = if smooth {
                d3::shape::curve_natural()
            } else {
                d3::shape::curve_cardinal(0.5)
            };
            curve_shape.line_start(&mut path);
            for point in &points {
                curve_shape.point(&mut path, point[0], point[1]);
            }
            curve_shape.line_end(&mut path);
            let shapes = svg_tessellate::tessellate(
                &path.to_string(),
                Some(Stroke::new(1.6 * unit, color)),
                None,
                svg_tessellate::Options::default(),
            );
            // Apply dashing by walking the tessellated polyline.
            for shape in shapes {
                if let egui::Shape::Path(path_shape) = shape {
                    dashed_path(painter, &path_shape.points, color);
                }
            }
        } else {
            let smooth = app.session.smooth_level > 0;
            let mut path = d3::path::Path::default();
            let mut curve_shape = if smooth {
                d3::shape::curve_natural()
            } else {
                d3::shape::curve_cardinal(0.5)
            };
            curve_shape.line_start(&mut path);
            for point in &points {
                curve_shape.point(&mut path, point[0], point[1]);
            }
            curve_shape.line_end(&mut path);
            let shapes = svg_tessellate::tessellate(
                &path.to_string(),
                Some(Stroke::new(2.3 * unit, color)),
                None,
                svg_tessellate::Options::default(),
            );
            for shape in shapes {
                painter.add(shape);
            }
        }
    }
}

/// A dashed line, for the targets.
fn dashed_path(painter: &egui::Painter, path: &[Pos2], color: Color32) {
    let dash: f32 = 6.0;
    let gap: f32 = 4.0;
    let mut drawn = 0.0;
    let mut on = true;
    for pair in path.windows(2) {
        let start = pair[0];
        let end = pair[1];
        let length = (end - start).length();
        if length <= 0.0 {
            continue;
        }
        let mut travelled = 0.0;
        while travelled < length {
            let want = if on { dash } else { gap };
            let take = (want - drawn).min(length - travelled);
            let from = start + (end - start) * (travelled / length);
            let to = start + (end - start) * ((travelled + take) / length);
            if on {
                painter.line_segment([from, to], Stroke::new(1.6, color));
            }
            travelled += take;
            drawn += take;
            if drawn >= want - 0.01 {
                on = !on;
                drawn = 0.0;
            }
        }
    }
}

/// The labels at the end of each curve.
fn labels(
    painter: &egui::Painter,
    app: &App,
    curves: &[PlotCurve<'_>],
    transform: &Transform,
    plot: Rect,
) {
    if !app.session.show_labels {
        return;
    }
    let font = TypeStep::Label.font_bold();
    let mut wanted: Vec<(String, f32, f32)> = Vec::new();
    for curve in curves {
        let Some(point) = curve.points.last() else {
            continue;
        };
        wanted.push((
            curve.label_key.to_owned(),
            transform.db_to_y(point.db),
            TypeStep::Label.size(),
        ));
    }
    let placed = place_labels(wanted, plot);
    for curve in curves {
        let Some(top) = placed.get(curve.label_key) else {
            continue;
        };
        let color = theme::color(curve.color);
        // The name is preceded by a bar of the curve's colour, so a name that
        // lies over another curve can still be read.
        let swatch = Pos2::new(plot.left() + 12.0, *top + TypeStep::Label.size() * 0.45);
        let mut path = d3::path::Path::default();
        path.rect(
            f64::from(swatch.x - 0.0),
            f64::from(swatch.y - 5.0),
            10.0,
            10.0,
        );
        let shapes = svg_tessellate::tessellate(
            &path.to_string(),
            None,
            Some(color),
            svg_tessellate::Options::default(),
        );
        for shape in shapes {
            painter.add(shape);
        }
        painter.text(
            Pos2::new(plot.left() + 28.0, *top),
            Align2::LEFT_TOP,
            curve.label,
            font.clone(),
            color,
        );
    }
}

/// The line under the pointer, and what each curve reads there.
///
/// This is the reference's inspector: a vertical line at the pointer's
/// frequency, and the value of every curve at that frequency, stacked so they
/// do not lie on top of one another.
fn inspector(
    painter: &egui::Painter,
    app: &App,
    curves: &[PlotCurve<'_>],
    transform: &Transform,
    plot: Rect,
    pointer: Option<Pos2>,
) {
    let palette = theme::current_palette();
    let Some(pointer) = pointer.filter(|position| plot.contains(*position)) else {
        return;
    };
    let frequency = transform.x_at(pointer.x);
    let x = transform.hz_to_x(frequency);
    let mut path = d3::path::Path::default();
    path.move_to(f64::from(x), f64::from(plot.top()));
    path.line_to(f64::from(x), f64::from(plot.bottom()));
    let shapes = svg_tessellate::tessellate(
        &path.to_string(),
        Some(Stroke::new(1.0, palette.text_tertiary)),
        None,
        svg_tessellate::Options::default(),
    );
    for shape in shapes {
        painter.add(shape);
    }
    // The readings, one under the other, from the top of the plot down.
    let mut next_top = plot.top() + 6.0;
    for curve in curves {
        let Some(point) = squiglab_core::graph::nearest_point(curve.points, frequency) else {
            continue;
        };
        let text = format!("{:.1} dB", point.db);
        painter.text(
            Pos2::new(x + 4.0, next_top),
            Align2::LEFT_TOP,
            text,
            TypeStep::Label.font(),
            theme::color(curve.color),
        );
        next_top += TypeStep::Label.size() + 3.0;
    }
    // The frequency itself, at the foot of the line.
    painter.text(
        Pos2::new(x, plot.bottom() - 4.0),
        Align2::CENTER_BOTTOM,
        squiglab_core::graph::format_frequency(frequency),
        TypeStep::Label.font(),
        palette.text_secondary,
    );
    let _ = app;
}

/// The rig the measurements were taken on, in the corner of the graph.
fn rig_line(painter: &egui::Painter, app: &App, rect: Rect) {
    let palette = theme::current_palette();
    let Some(description) = &app.session.rig_description else {
        return;
    };
    painter.text(
        Pos2::new(rect.right() - 10.0, rect.top() + 6.0),
        Align2::RIGHT_TOP,
        format!("Measured on: {description}"),
        TypeStep::Label.font(),
        palette.text_tertiary,
    );
}

/// The mark and the name, behind the curves.
fn watermark(painter: &egui::Painter, plot: Rect) {
    let palette = theme::current_palette();
    let center = plot.center();
    let size = 96.0;
    let source = egui::ImageSource::Bytes {
        uri: "bytes://mark.png".into(),
        bytes: squiglab_core::product::WINDOW_ICON_PNG.into(),
    };
    if let Ok(egui::load::TexturePoll::Ready { texture }) = source.load(
        painter.ctx(),
        egui::TextureOptions::LINEAR,
        egui::load::SizeHint::Scale(1.0.into()),
    ) {
        let rect = Rect::from_center_size(center, Vec2::splat(size));
        let color = if palette.is_dark() {
            Color32::WHITE.gamma_multiply(0.06)
        } else {
            Color32::BLACK.gamma_multiply(0.04)
        };
        painter.image(
            texture.id,
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            color,
        );
    }
    painter.text(
        Pos2::new(center.x, center.y + size * 0.45),
        Align2::CENTER_CENTER,
        squiglab_core::product::APP_NAME,
        TypeStep::Title.font_bold(),
        palette.text_tertiary.gamma_multiply(0.6),
    );
}

/// The frequency under the pointer, and the reading at it.
pub fn read_at(app: &App, position: Pos2, plot: Rect, transform: &Transform) -> Option<(f32, f32)> {
    if !plot.contains(position) {
        return None;
    }
    let frequency = transform.x_at(position.x);
    let mut reading: Option<f32> = None;
    for curve in &app.session.display_curves {
        if curve.points.is_empty() {
            continue;
        }
        if let Some(point) = squiglab_core::graph::nearest_point(&curve.points, frequency) {
            reading = Some(match reading {
                Some(previous) => previous.min(point.db),
                None => point.db,
            });
        }
    }
    reading.map(|db| (frequency, db))
}

/// The frequency as the reference writes it.
pub fn frequency_label(frequency: f32) -> String {
    format_frequency(frequency)
}

/// The limit the graph's scale is drawn to.
pub fn scale_limit(peak: f32) -> f32 {
    nice_limit(peak)
}

/// Where the labels sit.
pub fn label_layout(app: &App) -> squiglab_core::session::LabelLayout {
    app.session.label_layout
}

/// The graph as an SVG document.
///
/// The export is a drawing of the same graph, in a form that can be scaled
/// and printed: the axes, the grid, the curves, their names, and the mark.
pub fn export_svg(app: &App, width: f32, height: f32) -> String {
    let palette = theme::current_palette();
    let curves = plot_curves(app);
    let (minimum, maximum) = y_range(&curves);
    let (minimum, maximum) = scaled_y_range((minimum, maximum), app.session.y_scale);
    let zoom = app.session.zoom;

    let left = 44.0;
    let top = 10.0;
    let right = 10.0;
    let bottom = 46.0;
    let plot = Rect::from_min_max(
        Pos2::new(left, top),
        Pos2::new(width - right, height - bottom),
    );
    let transform = Transform::new(plot, zoom.x_range(), (minimum, maximum));

    let mut svg = String::new();
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" \
         viewBox=\"0 0 {width} {height}\">\n"
    ));
    svg.push_str(&format!(
        "<rect width=\"{width}\" height=\"{height}\" fill=\"{}\" />\n",
        hex(palette.graph)
    ));

    // The grid and the dB scale.
    let grid_color = hex(palette.border.gamma_multiply(0.6));
    let label_color = hex(palette.text_tertiary);
    let range = maximum - minimum;
    let t = if range < 40.0 { 1.0_f32 } else { 5.0_f32 };
    let y0 = (minimum / t).ceil();
    let y1 = (maximum / t).floor();
    // As the screen's grid: the range steps in tick indices, not decibels.
    let db_ticks: Vec<f32> = d3::array::range(f64::from(y0), Some(f64::from(y1) + 1.0), Some(1.0))
        .into_iter()
        .map(|v| (v * f64::from(t)) as f32)
        .collect();
    for db in db_ticks {
        let y = transform.db_to_y(db);
        if y >= plot.top() && y <= plot.bottom() {
            let major = (db as i32) % 5 == 0;
            let thickness = if major { 0.7 } else { 0.2 };
            svg.push_str(&format!(
                "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{grid_color}\" stroke-width=\"{:.1}\" />\n",
                plot.left(), y, plot.right(), y, thickness
            ));
            if major {
                svg.push_str(&format!(
                    "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"end\" font-family=\"sans-serif\" font-size=\"11\" fill=\"{label_color}\">{}</text>\n",
                    plot.left() - 6.0,
                    y + 4.0,
                    tick_db(db)
                ));
            }
        }
    }
    for frequency in x_tick_frequencies() {
        if frequency < transform.x_range.0 || frequency > transform.x_range.1 {
            continue;
        }
        let x = transform.hz_to_x(frequency);
        svg.push_str(&format!(
            "<line x1=\"{x:.2}\" y1=\"{:.2}\" x2=\"{x:.2}\" y2=\"{:.2}\" stroke=\"{grid_color}\" stroke-width=\"0.7\" />\n",
            plot.top(),
            plot.bottom()
        ));
        svg.push_str(&format!(
            "<text x=\"{x:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"11\" fill=\"{label_color}\">{}</text>\n",
            plot.bottom() + 18.0,
            format_axis_frequency(frequency)
        ));
    }

    // The curves.
    for curve in &curves {
        if curve.points.is_empty() {
            continue;
        }
        let points: Vec<[f64; 2]> = curve
            .points
            .iter()
            .map(|point| {
                [
                    f64::from(transform.hz_to_x(point.hz)),
                    f64::from(transform.db_to_y(point.db)),
                ]
            })
            .collect();
        let smooth = app.session.smooth_level > 0;
        let mut path = d3::path::Path::default();
        let mut curve_shape = if smooth {
            d3::shape::curve_natural()
        } else {
            d3::shape::curve_cardinal(0.5)
        };
        curve_shape.line_start(&mut path);
        for point in &points {
            curve_shape.point(&mut path, point[0], point[1]);
        }
        curve_shape.line_end(&mut path);
        let d = path.to_string();
        let color = hex(theme::color(curve.color));
        svg.push_str(&format!(
            "<path d=\"{d}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"2\"{dash} />\n",
            dash = if curve.dashed {
                " stroke-dasharray=\"6 4\""
            } else {
                ""
            }
        ));
    }

    // The mark and the name.
    svg.push_str(&format!(
        "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"16\" fill=\"{}\">{}</text>\n",
        plot.center().x,
        plot.center().y,
        hex(palette.text_tertiary.gamma_multiply(0.6)),
        squiglab_core::product::APP_NAME
    ));
    svg.push_str("</svg>\n");
    svg
}

/// A colour as the SVG spells it.
fn hex(color: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b())
}
