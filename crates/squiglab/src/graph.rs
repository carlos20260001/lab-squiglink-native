//! The graph.
//!
//! The geometry is the core's: where a frequency sits on the axis, how a
//! curve is smoothed, where a label goes. This is only the drawing of it, and
//! the layout rule the reference lays the graph out by: the graph is at most
//! 2.2 units across for every unit of the window's height, less the chrome
//! around it, and it is centred in its column.

use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use squiglab_core::graph::{
    format_axis_frequency, format_frequency, nice_limit, place_labels, sample_curve_path,
    scaled_y_range, tick_db, y_range, LabelLayout, PlotCurve, Transform, FREQUENCY_BANDS,
};

use crate::app::App;
use crate::theme::{self, metrics, TypeStep};

/// The frequencies the axis is marked at.
const X_TICKS: [f32; 22] = [
    20.0, 30.0, 40.0, 50.0, 60.0, 80.0, 100.0, 200.0, 300.0, 400.0, 500.0, 600.0, 800.0, 1000.0,
    2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 8000.0, 10000.0, 20000.0,
];

/// The graph, in the middle of the window.
pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let palette = theme::current_palette();
    // The reference's rule for the graph's width: two and a bit units across
    // for every unit of the window's height, less the chrome around it. The
    // graph keeps its proportion, so the room there is sets the height and
    // the height sets the width.
    let window_height = ui.ctx().content_rect().height();
    let by_height =
        (window_height - metrics::GRAPH_CHROME_HEIGHT).max(120.0) * metrics::GRAPH_ASPECT;
    // The graph is as tall as the reference's rule says, and the column
    // scrolls under it, as the reference's page does.
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
        painter.rect_filled(rect, 0.0, palette.graph);
        let plot = plot_rect(rect);
        let transform = transform_for(app, plot);
        draw(app, &painter, rect, response.hover_pos());
        scale(app, &painter, rect, &response);
        eq_editor(app, &painter, plot, &transform, &response, &ctx);
    });
}

/// The plot's own area, inside the box: room for the dB scale on the left
/// and for the frequency axis and the band names below.
fn plot_rect(rect: Rect) -> Rect {
    Rect::from_min_max(
        Pos2::new(rect.left() + 44.0, rect.top() + 10.0),
        Pos2::new(rect.right() - 10.0, rect.bottom() - 46.0),
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
/// range up and down, and the two circles change how far it reaches. A
/// double-click puts it back where the curves say it should be.
fn scale(app: &mut App, painter: &egui::Painter, rect: Rect, response: &egui::Response) {
    let palette = theme::current_palette();
    let plot = Rect::from_min_max(
        Pos2::new(rect.left() + 44.0, rect.top() + 10.0),
        Pos2::new(rect.right() - 10.0, rect.bottom() - 46.0),
    );
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
    let centre = (minimum + maximum) / 2.0;
    let half = (maximum - minimum) / 2.0;
    let middle_y = plot.center().y;
    let half_y = half / (maximum - minimum) * plot.height();
    let x = plot.left() - 6.0;

    // The scale's line, and the mark in the middle.
    painter.line_segment(
        [
            Pos2::new(x, middle_y - half_y),
            Pos2::new(x, middle_y + half_y),
        ],
        egui::Stroke::new(1.0, palette.border),
    );
    painter.line_segment(
        [Pos2::new(x - 5.0, middle_y), Pos2::new(x + 5.0, middle_y)],
        egui::Stroke::new(1.0, palette.text_tertiary),
    );

    // The two circles, which change how far the scale reaches.
    for sign in [-1.0_f32, 1.0] {
        painter.circle_stroke(
            Pos2::new(x, middle_y + half_y * sign),
            7.0,
            egui::Stroke::new(1.5, palette.text_secondary),
        );
    }

    // Dragging the middle mark moves the range; dragging a circle widens it.
    // The scale is only taken hold of at the scale itself: the rest of the
    // graph belongs to the equalizer, as it does in the reference.
    let on_the_scale = response
        .hover_pos()
        .is_some_and(|point| (point.x - x).abs() <= 12.0);
    if response.drag_started() && on_the_scale {
        // Where the pointer took hold, which is what tells the mark from the
        // circles once the pointer has moved.
        app.scale_grab = response.hover_pos();
    }
    if response.dragged() && app.scale_grab.is_some() {
        let delta = response.drag_delta().y;
        if delta != 0.0 {
            let per_pixel = (maximum - minimum) / plot.height();
            let range;
            // The circle the pointer took hold of: the one above the middle
            // or the one below it, which reach in opposite directions.
            let circle = app
                .scale_grab
                .filter(|grab| (grab.y - middle_y).abs() > half_y * 0.6);
            if let Some(grab) = circle {
                // The pointer is on a circle: the scale reaches further or less.
                let sign = if grab.y < middle_y { -1.0 } else { 1.0 };
                let new_half = (half + sign * delta * per_pixel).abs().max(1.0);
                range = (centre - new_half, centre + new_half);
            } else {
                // The pointer is on the mark: the whole range moves.
                let shift = -delta * per_pixel;
                range = (minimum + shift, maximum + shift);
            }
            app.session.graph_range = Some(range);
            app.session.dirty = true;
        }
    }
    if response.double_clicked() {
        app.session.graph_range = None;
        app.session.dirty = true;
    }
}

/// Draw the graph into a rectangle, with the pointer where it is.
fn draw(app: &App, painter: &egui::Painter, rect: Rect, pointer: Option<Pos2>) {
    let curves = plot_curves(app);

    let plot = plot_rect(rect);
    if plot.width() <= 0.0 || plot.height() <= 0.0 {
        return;
    }

    // The range is the curves' own, unless the user has dragged the scale.
    let (minimum, maximum) = match app.session.graph_range {
        Some(range) => range,
        None => {
            let (minimum, maximum) = y_range(&curves);
            scaled_y_range((minimum, maximum), app.session.y_scale)
        }
    };
    let transform = transform_for(app, plot);

    grid(painter, plot, &transform, &minimum, &maximum);
    band_names(painter, plot, &transform);
    curves_into(painter, app, &curves, &transform);
    if app.session.inspect_mode {
        inspector(painter, app, &curves, &transform, plot, pointer);
    }
    labels(painter, app, &curves, &transform, plot);
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
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(left, plot.top()), Pos2::new(right, plot.bottom())),
            0.0,
            palette.accent.gamma_multiply(0.18),
        );
    }

    // The marks.
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
        let color = theme::color(crate::graph::curve_color(
            app,
            app.session.eq_phone.unwrap_or(0),
        ));
        painter.circle_filled(centre, radius, palette.graph);
        painter.circle_stroke(centre, radius, Stroke::new(1.4, color));
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
    let hz = hz as f32;
    for window in points.windows(2) {
        let (left, right) = (&window[0], &window[1]);
        if hz >= left.hz && hz <= right.hz {
            let span = right.hz - left.hz;
            if span <= 0.0 {
                return left.db;
            }
            let place = (hz - left.hz) / span;
            return left.db + (right.db - left.db) * place;
        }
    }
    if hz < points[0].hz {
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

/// The grid, the dB scale and the frequency axis.
fn grid(painter: &egui::Painter, plot: Rect, transform: &Transform, minimum: &f32, maximum: &f32) {
    let palette = theme::current_palette();
    let grid_color = palette.border.gamma_multiply(0.6);
    let label_font = FontId::new(TypeStep::Label.size(), egui::FontFamily::Proportional);
    let label_color = palette.text_tertiary;

    // The horizontal lines: every five decibels.
    let step = 5.0_f32;
    let mut db = (*minimum / step).ceil() * step;
    while db <= *maximum {
        let y = transform.db_to_y(db);
        if y >= plot.top() && y <= plot.bottom() {
            painter.line_segment(
                [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
                Stroke::new(0.7, grid_color),
            );
            painter.text(
                Pos2::new(plot.left() - 6.0, y),
                Align2::RIGHT_CENTER,
                tick_db(db),
                label_font.clone(),
                label_color,
            );
        }
        db += step;
    }

    // The vertical lines: the marked frequencies.
    for frequency in X_TICKS {
        if frequency < transform.x_range.0 || frequency > transform.x_range.1 {
            continue;
        }
        let x = transform.hz_to_x(frequency);
        painter.line_segment(
            [Pos2::new(x, plot.top()), Pos2::new(x, plot.bottom())],
            Stroke::new(0.7, grid_color),
        );
        painter.text(
            Pos2::new(x, plot.bottom() + 6.0),
            Align2::CENTER_TOP,
            format_axis_frequency(frequency),
            label_font.clone(),
            label_color,
        );
    }
}

/// The names the reference gives the bands of the axis.
fn band_names(painter: &egui::Painter, plot: Rect, transform: &Transform) {
    let palette = theme::current_palette();
    let font = FontId::new(TypeStep::Label.size(), egui::FontFamily::Proportional);
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
            Pos2::new((start + end) / 2.0, plot.bottom() + 22.0),
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
    for curve in curves {
        if curve.points.is_empty() {
            continue;
        }
        let points: Vec<Pos2> = curve
            .points
            .iter()
            .map(|point| Pos2::new(transform.hz_to_x(point.hz), transform.db_to_y(point.db)))
            .collect();
        let path = sample_curve_path(&points, app.session.smooth_level > 0);
        let color = theme::color(curve.color);
        if curve.dashed {
            dashed_path(painter, &path, color);
        } else {
            painter.add(egui::Shape::line(path, Stroke::new(2.0, color)));
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
        painter.rect_filled(
            Rect::from_min_max(swatch - Vec2::new(0.0, 5.0), swatch + Vec2::new(10.0, 5.0)),
            0.0,
            color,
        );
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
    painter.line_segment(
        [Pos2::new(x, plot.top()), Pos2::new(x, plot.bottom())],
        egui::Stroke::new(1.0, palette.text_tertiary),
    );
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
pub fn label_layout(app: &App) -> LabelLayout {
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
    let mut db = (minimum / 5.0).ceil() * 5.0;
    while db <= maximum {
        let y = transform.db_to_y(db);
        if y >= plot.top() && y <= plot.bottom() {
            svg.push_str(&format!(
                "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{grid_color}\" stroke-width=\"0.7\" />\n",
                plot.left(), y, plot.right(), y
            ));
            svg.push_str(&format!(
                "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"end\" font-family=\"sans-serif\" font-size=\"11\" fill=\"{label_color}\">{}</text>\n",
                plot.left() - 6.0,
                y + 4.0,
                tick_db(db)
            ));
        }
        db += 5.0;
    }
    for frequency in X_TICKS {
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
        let points: Vec<Pos2> = curve
            .points
            .iter()
            .map(|point| Pos2::new(transform.hz_to_x(point.hz), transform.db_to_y(point.db)))
            .collect();
        let path = sample_curve_path(&points, app.session.smooth_level > 0);
        let color = hex(theme::color(curve.color));
        let points = path
            .iter()
            .map(|point| format!("{:.2},{:.2}", point.x, point.y))
            .collect::<Vec<_>>()
            .join(" ");
        svg.push_str(&format!(
            "<polyline points=\"{points}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"2\"{dash} />\n",
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
