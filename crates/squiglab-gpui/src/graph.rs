//! The graph, in the GPUI interface.
//!
//! A first port of the reference's plot: the grid, the decibel and frequency
//! axes, the curves that are on the graph, all drawn on a logarithmic
//! frequency axis with a linear decibel axis. The labels, inspector and
//! equalizer editor follow in later milestones; this module is the seam at
//! which they will hang.

use gpui::{
    point, px, Bounds, Hsla, PathBuilder, PathStyle, Pixels, Point, Rgba, StrokeOptions, TextRun,
    Window,
};
use squiglab_core::color::get_curve_color_with_offset;
use squiglab_core::d3::math::{Point as CorePoint, Rect as CoreRect};
use squiglab_core::graph::{
    db_tick_values, format_axis_frequency, place_labels, scaled_y_range, tick_db,
    x_tick_frequencies, Transform, FREQUENCY_BANDS,
};
use squiglab_core::session::Session;

/// A snapshot of the session data the graph needs, so it can be moved into
/// a canvas paint closure.
#[derive(Clone)]
pub struct GraphData {
    pub curves: Vec<CurveSnapshot>,
    pub zoom_x: (f32, f32),
    pub y_scale: f32,
    pub color_offset: usize,
    pub line_color: Rgba,
    pub text_color: Rgba,
}

#[derive(Clone)]
pub struct CurveSnapshot {
    pub label: String,
    pub label_key: String,
    pub points: Vec<(f32, f32)>,
    pub index: usize,
    pub target: bool,
    pub target_slot: usize,
}

impl GraphData {
    pub fn from_session(session: &Session, line_color: Rgba, text_color: Rgba) -> Self {
        let mut curves = Vec::new();
        let mut target_slot = 0_usize;
        for (index, c) in session.display_curves.iter().enumerate() {
            let slot = if c.target {
                let s = target_slot;
                target_slot += 1;
                s
            } else {
                0
            };
            curves.push(CurveSnapshot {
                label: c.label.clone(),
                label_key: format!("{index}_{}", c.label),
                points: c.points.iter().map(|p| (p.hz, p.db)).collect(),
                index,
                target: c.target,
                target_slot: slot,
            });
        }
        Self {
            curves,
            zoom_x: session.zoom.x_range(),
            y_scale: session.y_scale,
            color_offset: session.color_offset,
            line_color,
            text_color,
        }
    }
}

fn stroke(width: f32) -> PathStyle {
    let mut opts = StrokeOptions::default();
    opts.line_width = width;
    PathStyle::Stroke(opts)
}

/// Paint a run of text at `origin` (the baseline), using the reference's
/// font. GPUI's canvas has no ready-made text element, so we lay the line
/// out with the text system and stamp each glyph.
fn paint_text(
    window: &mut Window,
    text: &str,
    origin: Point<Pixels>,
    font_size: Pixels,
    color: Hsla,
) {
    let text_system = window.text_system();
    let font = gpui::font("Open Sans");
    let font_id = text_system.resolve_font(&font);
    let run = TextRun {
        font,
        color,
        len: text.len(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let layout = text_system.layout_line(text, font_size, &[run], None);
    for run in &layout.runs {
        for glyph in &run.glyphs {
            let _ = window.paint_glyph(
                point(
                    px(f32::from(origin.x) + f32::from(glyph.position.x)),
                    px(f32::from(origin.y) + f32::from(glyph.position.y)),
                ),
                font_id,
                glyph.id,
                font_size,
                color,
            );
        }
    }
}

/// Paint text horizontally centered around `origin.x`.
fn paint_text_centered(
    window: &mut Window,
    text: &str,
    origin: Point<Pixels>,
    font_size: Pixels,
    color: Hsla,
) {
    let text_system = window.text_system();
    let font = gpui::font("Open Sans");
    let font_id = text_system.resolve_font(&font);
    let run = TextRun {
        font,
        color,
        len: text.len(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let layout = text_system.layout_line(text, font_size, &[run], None);
    let half_width = layout.width / 2.0;
    for run in &layout.runs {
        for glyph in &run.glyphs {
            let _ = window.paint_glyph(
                point(
                    px(f32::from(origin.x) - f32::from(half_width) + f32::from(glyph.position.x)),
                    px(f32::from(origin.y) + f32::from(glyph.position.y)),
                ),
                font_id,
                glyph.id,
                font_size,
                color,
            );
        }
    }
}

/// The grid, the decibel scale and the frequency axis.
///
/// Everything here is the reference's own: the tick values come from
/// `fmtY` and `fmtX` in graphtool.js, the figures are written in the
/// monospaced face at the baseline, and the lines carry the colour the
/// reference's stylesheet gives every grid line.
fn draw_grid(
    window: &mut Window,
    plot: &CoreRect,
    transform: &Transform,
    minimum: f32,
    maximum: f32,
    line_color: Rgba,
    text_color: Hsla,
) {
    let unit = plot.width() / 800.0;
    let font_size = px(10.0 * unit);
    let minor_font_size = px(8.6 * unit);

    // The decibel scale. The step is one when the range is narrow and
    // five otherwise; the ticks run from the ceiling of the lower bound
    // to the floor of the upper one.
    let range = (maximum - minimum).max(1.0);
    let step = if range < 40.0 { 1.0_f32 } else { 5.0_f32 };
    let ticks = db_tick_values(minimum, maximum);
    let mut minor_path = PathBuilder::default();
    minor_path.style = stroke(1.0);

    for db in &ticks {
        let y = transform.db_to_y(*db);
        if y < plot.top() || y > plot.bottom() {
            continue;
        }
        let minor = step == 1.0 && (*db as i32) % 5 != 0;
        let across = |path: &mut PathBuilder| {
            path.move_to(point(px(plot.left()), px(y)));
            path.line_to(point(px(plot.right()), px(y)));
        };
        if minor {
            across(&mut minor_path);
            continue;
        }
        // The figure: its left edge three units in from the plot's side,
        // its baseline two above the line it names.
        paint_text(
            window,
            &tick_db(*db),
            point(px(plot.left() + 3.0 * unit), px(y - 2.0 * unit)),
            font_size,
            text_color,
        );
    }
    let blur: [(f32, f32); 3] = [(1.2, 0.075), (2.4, 0.045), (4.0, 0.015)];
    for (width, alpha) in blur {
        let mut builder = PathBuilder::default();
        builder.style = stroke(width * unit);
        for db in &ticks {
            let y = transform.db_to_y(*db);
            if y < plot.top() || y > plot.bottom() {
                continue;
            }
            let minor = step == 1.0 && (*db as i32) % 5 != 0;
            if minor {
                continue;
            }
            builder.move_to(point(px(plot.left()), px(y)));
            builder.line_to(point(px(plot.right()), px(y)));
        }
        if let Ok(path) = builder.build() {
            window.paint_path(path, Hsla::from(line_color).alpha(alpha));
        }
    }
    // The minor lines are `0.2*(1-r/45)`, the reference's own figure.
    let mut opts = gpui::StrokeOptions::default();
    opts.line_width = 0.2 * (1.0 - range / 45.0) * unit;
    minor_path.style = gpui::PathStyle::Stroke(opts);
    if let Ok(path) = minor_path.build() {
        window.paint_path(path, line_color);
    }

    // The frequency axis. The reference's ticks run from ten units below
    // the top of the plot to two above its foot, each weighted by its
    // place in the pattern, and each carries its figure centred on it.
    const X_TICK_PATTERN: [usize; 8] = [3, 0, 0, 1, 0, 0, 2, 0];
    let frequencies = x_tick_frequencies();
    let last = frequencies.len().saturating_sub(1);
    let mut widths = [
        PathBuilder::default(),
        PathBuilder::default(),
        PathBuilder::default(),
        PathBuilder::default(),
    ];
    for w in &mut widths {
        w.style = stroke(1.0);
    }
    let top = plot.top() + 10.0 * unit;
    let foot = plot.bottom() - 2.0 * unit;
    for (i, hz) in frequencies.iter().enumerate() {
        if *hz < transform.x_range.0 || *hz > transform.x_range.1 {
            continue;
        }
        let x = transform.hz_to_x(*hz);
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
        widths[band].move_to(point(px(x), px(top)));
        widths[band].line_to(point(px(x), px(foot)));
        // The figure: the plain ticks of the pattern stand eighty-six per
        // cent tall, and the two ends are nudged in from the range.
        let plain = X_TICK_PATTERN[i % 8] == 0;
        let (fs, units) = if plain {
            (minor_font_size, 8.6)
        } else {
            (font_size, 10.0)
        };
        let nudge = if i == 0 {
            4.0
        } else if i == last {
            -5.0
        } else {
            0.0
        };
        let label = format_axis_frequency(*hz);
        paint_text(
            window,
            &label,
            point(px(x + nudge * unit), px(foot + units * unit)),
            fs,
            text_color,
        );
    }
    for w in widths {
        if let Ok(path) = w.build() {
            window.paint_path(path, line_color);
        }
    }
}

fn draw_watermark(window: &mut Window, plot: &CoreRect, unit: f32, text_color: Hsla) {
    let font_size = px(22.0 * unit);
    paint_text_centered(
        window,
        squiglab_core::product::APP_NAME,
        point(px(plot.center().x), px(plot.center().y)),
        font_size,
        text_color.alpha(0.08),
    );
}

fn draw_frequency_bands(
    window: &mut Window,
    plot: &CoreRect,
    transform: &Transform,
    unit: f32,
    text_color: Hsla,
) {
    let font_size = px(8.5 * unit);
    let top = plot.bottom() + 14.0 * unit;
    for band in &FREQUENCY_BANDS {
        let (low, high) = band.range_hz;
        if high < transform.x_range.0 || low > transform.x_range.1 {
            continue;
        }
        let start = transform.hz_to_x(low.max(transform.x_range.0));
        let end = transform.hz_to_x(high.min(transform.x_range.1));
        let width = end - start;
        let needed = band.title.len() as f32 * 8.5 * unit * 0.55;
        if width < needed {
            continue;
        }
        paint_text_centered(
            window,
            band.title,
            point(px((start + end) / 2.0), px(top)),
            font_size,
            text_color.alpha(0.6),
        );
    }
}

fn draw_labels(
    window: &mut Window,
    plot: &CoreRect,
    curves: &[CurveSnapshot],
    color_offset: usize,
    unit: f32,
) {
    let mut wanted = Vec::new();
    for curve in curves {
        if curve.target {
            continue;
        }
        if let Some((hz, db)) = curve.points.last() {
            wanted.push((curve.label_key.clone(), *hz, *db));
        }
    }
    if wanted.is_empty() {
        return;
    }
    let placed = place_labels(wanted, *plot);
    let font_size = px(11.0 * unit);

    for curve in curves {
        let Some(top) = placed.get(&curve.label_key) else {
            continue;
        };
        let c = get_curve_color_with_offset(curve.index, color_offset);
        let color = gpui::rgb(((c.r as u32) << 16) | ((c.g as u32) << 8) | (c.b as u32));

        // Swatch square
        let swatch_x = plot.left() + 12.0 * unit;
        let swatch_y = *top - 8.0 * unit;
        let swatch_size = 8.0 * unit;
        let mut builder = PathBuilder::default();
        builder.move_to(point(px(swatch_x), px(swatch_y)));
        builder.line_to(point(px(swatch_x + swatch_size), px(swatch_y)));
        builder.line_to(point(
            px(swatch_x + swatch_size),
            px(swatch_y + swatch_size),
        ));
        builder.line_to(point(px(swatch_x), px(swatch_y + swatch_size)));
        builder.close();
        if let Ok(path) = builder.build() {
            window.paint_path(path, color);
        }

        // Label text
        paint_text(
            window,
            &curve.label,
            point(px(plot.left() + 24.0 * unit), px(*top)),
            font_size,
            Hsla::from(color),
        );
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

    // Watermark at the center
    draw_watermark(window, &plot, unit, Hsla::from(data.text_color));

    // Grid first, so the curves draw over it.
    draw_grid(
        window,
        &plot,
        &transform,
        minimum,
        maximum,
        data.line_color,
        Hsla::from(data.text_color),
    );

    // Frequency band names under axis
    draw_frequency_bands(window, &plot, &transform, unit, Hsla::from(data.text_color));

    // Then the curves themselves.
    for curve in &data.curves {
        if curve.points.is_empty() {
            continue;
        }
        let color = get_curve_color_with_offset(curve.index, data.color_offset);
        let alpha = if curve.target {
            match curve.target_slot {
                0 => 1.0,
                1 => 0.8,
                2 => 0.65,
                _ => 0.5,
            }
        } else {
            1.0
        };
        let mut builder = PathBuilder::default();
        let mut opts = gpui::StrokeOptions::default();
        opts.line_width = if curve.target { 1.5 } else { 2.0 };
        builder.style = gpui::PathStyle::Stroke(opts);
        builder.move_to(point(
            px(transform.hz_to_x(curve.points[0].0)),
            px(transform.db_to_y(curve.points[0].1)),
        ));
        for (hz, db) in &curve.points[1..] {
            builder.line_to(point(
                px(transform.hz_to_x(*hz)),
                px(transform.db_to_y(*db)),
            ));
        }
        if let Ok(path) = builder.build() {
            let color =
                gpui::rgb(((color.r as u32) << 16) | ((color.g as u32) << 8) | (color.b as u32));
            window.paint_path(path, Hsla::from(color).alpha(alpha));
        }
    }

    // Placed curve labels with swatches
    draw_labels(window, &plot, &data.curves, data.color_offset, unit);
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::rgba;
    use squiglab_core::data::FRPoint;
    use squiglab_core::session::{CurveSource, DisplayCurve, Session};

    #[test]
    fn graph_data_snapshots_curves_and_slots() {
        let session = Session {
            display_curves: vec![
                DisplayCurve {
                    group_index: 0,
                    source: CurveSource::Measurement,
                    label: "Phone A".to_string(),
                    points: vec![
                        FRPoint { hz: 20.0, db: 60.0 },
                        FRPoint {
                            hz: 20000.0,
                            db: 70.0,
                        },
                    ],
                    target: false,
                },
                DisplayCurve {
                    group_index: 1,
                    source: CurveSource::Measurement,
                    label: "Target T1".to_string(),
                    points: vec![
                        FRPoint { hz: 20.0, db: 65.0 },
                        FRPoint {
                            hz: 20000.0,
                            db: 65.0,
                        },
                    ],
                    target: true,
                },
                DisplayCurve {
                    group_index: 2,
                    source: CurveSource::Measurement,
                    label: "Target T2".to_string(),
                    points: vec![
                        FRPoint { hz: 20.0, db: 60.0 },
                        FRPoint {
                            hz: 20000.0,
                            db: 60.0,
                        },
                    ],
                    target: true,
                },
            ],
            ..Default::default()
        };

        let line_color = rgba(0x808080ff);
        let text_color = rgba(0x000000ff);
        let data = GraphData::from_session(&session, line_color, text_color);

        assert_eq!(data.curves.len(), 3);
        assert!(!data.curves[0].target);
        assert_eq!(data.curves[0].target_slot, 0);

        assert!(data.curves[1].target);
        assert_eq!(data.curves[1].target_slot, 0);

        assert!(data.curves[2].target);
        assert_eq!(data.curves[2].target_slot, 1);
        assert_eq!(data.curves[0].points.len(), 2);
    }
}
