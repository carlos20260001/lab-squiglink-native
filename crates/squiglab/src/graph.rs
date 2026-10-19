use crate::theme::{self, Palette, TypeStep};
use egui::{Align2, Color32, Id, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use squiglab_core::graph::{
    clamp_label_position, corner_label_position, curve_control_points, format_axis_frequency,
    format_frequency, nearest_curve_index, nearest_point, nice_limit, place_labels,
    sample_curve_path, scaled_y_range, tick_db, y_range, Transform,
};
pub use squiglab_core::graph::{LabelLayout, PlotCurve, ZoomRegion};
use std::collections::BTreeMap;

const X_TICKS: [f32; 22] = [
    20.0, 30.0, 40.0, 50.0, 60.0, 80.0, 100.0, 200.0, 300.0, 400.0, 500.0, 600.0, 800.0, 1000.0,
    2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 8000.0, 10000.0, 20000.0,
];

#[derive(Default)]
pub struct GraphInteraction {
    pub hover_hz: Option<f32>,
    pub label_positions: BTreeMap<String, [f32; 2]>,
    pub y_scale: Option<f32>,
}

/// The strip at the top of the curve manager showing how far each measurement
/// sits from the target.
///
/// The graph answers "what does this headphone do". This answers "how far is
/// that from the target", which is a different question and one the graph
/// cannot show, because on the graph the two curves sit at their absolute
/// levels and their distance depends on the axis.
///
/// Everything here is a deviation, so the scale is symmetric about zero: a
/// curve above the target is as wrong as one the same distance below it, and
/// an asymmetric axis would say otherwise.
pub struct DeltaBar {
    pub zoom: ZoomRegion,
    /// What the deviations are measured against, drawn in the caption.
    pub caption: String,
}

impl DeltaBar {
    /// How tall the strip is, caption and axis included.
    pub const HEIGHT: f32 = 88.0;
    const CAPTION_HEIGHT: f32 = 18.0;
    const AXIS_HEIGHT: f32 = 16.0;
    const INSET_LEFT: f32 = 64.0;
    const INSET_RIGHT: f32 = 18.0;

    /// Draw the strip, returning the frequency under the pointer so a caller
    /// can read a deviation off it.
    ///
    /// Takes the width the panel offers rather than any other plot's, because
    /// it is drawn inside the manager and the manager spans the full window:
    /// pinning it to the graph's width would either leave a gap or overflow it.
    /// The frequency mapping is unchanged, so it still describes the same range
    /// the graph is showing at the current zoom.
    ///
    /// Returns `None` without drawing anything when there is nothing to say:
    /// a bar with no curves is a bar of nothing, and it would only take room
    /// from the rows below it.
    pub fn show(&self, ui: &mut Ui, curves: &[PlotCurve<'_>]) -> Option<f32> {
        if curves.is_empty() {
            return None;
        }
        let palette = crate::theme::current_palette();
        let axis_font = theme::font(TypeStep::Caption);
        let size = egui::vec2(ui.available_width(), Self::HEIGHT);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, palette.surface_base);

        let area = Rect::from_min_max(
            Pos2::new(
                rect.left() + Self::INSET_LEFT,
                rect.top() + Self::CAPTION_HEIGHT,
            ),
            Pos2::new(
                rect.right() - Self::INSET_RIGHT,
                rect.bottom() - Self::AXIS_HEIGHT,
            ),
        );
        // Symmetric about zero, because a deviation has no preferred sign:
        // being 6 dB above the target is no closer to correct than 6 dB below.
        let limit = nice_limit(
            curves
                .iter()
                .flat_map(|curve| curve.points.iter().map(|point| point.db.abs()))
                .filter(|value| value.is_finite())
                .fold(0.0_f32, f32::max),
        );
        let transform = Transform::new(area, self.zoom.x_range(), (-limit, limit));
        let plot_painter = painter.with_clip_rect(area);

        for tick in [-limit, limit] {
            let y = transform.db_to_y(tick);
            plot_painter.line_segment(
                [Pos2::new(area.left(), y), Pos2::new(area.right(), y)],
                Stroke::new(theme::graph::GRID_WIDTH, palette.border),
            );
            painter.text(
                Pos2::new(rect.left() + Self::INSET_LEFT - 6.0, y),
                Align2::LEFT_CENTER,
                tick_db(tick),
                axis_font.clone(),
                palette.text_tertiary,
            );
        }
        // Zero is the line the whole bar is read against, so it is drawn as
        // the axis rather than as another gridline.
        let zero = transform.db_to_y(0.0);
        plot_painter.line_segment(
            [Pos2::new(area.left(), zero), Pos2::new(area.right(), zero)],
            Stroke::new(1.2, palette.text_secondary),
        );
        painter.text(
            Pos2::new(rect.left() + Self::INSET_LEFT - 6.0, zero),
            Align2::LEFT_CENTER,
            "0",
            axis_font.clone(),
            palette.text_secondary,
        );
        for hz in X_TICKS {
            let x = transform.hz_to_x(hz);
            if x < area.left() || x > area.right() {
                continue;
            }
            plot_painter.line_segment(
                [Pos2::new(x, area.top()), Pos2::new(x, area.bottom())],
                Stroke::new(theme::graph::GRID_WIDTH, palette.border),
            );
        }

        for curve in curves {
            let points: Vec<Pos2> = curve
                .points
                .iter()
                .map(|point| Pos2::new(transform.hz_to_x(point.hz), transform.db_to_y(point.db)))
                .collect();
            GraphCanvas::draw_curve(
                &plot_painter,
                &points,
                theme::color(curve.color),
                curve.dashed,
                false,
                theme::graph::CURVE_WIDTH,
            );
        }

        // Caption: what the deviations are against, and in what.
        painter.text(
            Pos2::new(rect.left() + Self::INSET_LEFT, rect.top() + 10.0),
            Align2::LEFT_CENTER,
            self.caption.clone(),
            theme::font(TypeStep::Micro),
            palette.text_tertiary,
        );
        painter.text(
            Pos2::new(rect.right() - Self::INSET_RIGHT, rect.top() + 10.0),
            Align2::RIGHT_CENTER,
            format!("\u{b1}{limit:.1} dB"),
            theme::font(TypeStep::Micro),
            palette.text_tertiary,
        );

        response.hover_pos().map(|pos| transform.x_at(pos.x))
    }
}

pub struct GraphCanvas<'a> {
    pub zoom: ZoomRegion,
    pub smooth_level: usize,
    pub show_labels: bool,
    pub inspect_mode: bool,
    pub label_positions: &'a BTreeMap<String, [f32; 2]>,
    pub label_layout: LabelLayout,
    pub tutorial_band: Option<(&'a str, (f32, f32))>,
    /// Named frequency regions drawn beneath the axis at all times, so the
    /// graph is readable as a map even when the tutorial is closed.
    pub bands: &'a [FrequencyBand],
    /// Optional provenance caption drawn in the top-right corner of the graph.
    pub rig_description: Option<&'a str>,
    /// The mark and the name behind the curves.
    pub watermark: Watermark<'a>,
    pub y_scale: f32,
}

/// The mark the reference tool stamps behind the curves, and the name it
/// prints under it.
///
/// The web reference draws these into the same SVG it displays and exports
/// (`web/config.js`, `watermark()`), so a graph carries the mark wherever it
/// goes. The screen draws the mark from a texture the caller has loaded; the
/// export embeds the same bytes.
#[derive(Clone, Copy, Debug, Default)]
pub struct Watermark<'a> {
    /// The mark itself, as PNG bytes.
    pub png: Option<&'a [u8]>,
    /// The mark as a loaded texture, for the screen.
    pub texture: Option<egui::TextureId>,
    /// The name printed under the mark.
    pub text: Option<&'a str>,
}

/// What an exported graph says about itself: the mark and the name behind the
/// curves, and the rig the measurements came from.
#[derive(Clone, Copy, Default)]
pub struct Provenance<'a> {
    pub watermark: Watermark<'a>,
    pub rig_description: Option<&'a str>,
}

/// The width of the reference tool's plot area, in its own SVG units: an
/// 800-unit graph less 15 units of padding on each side. The watermark is
/// sized against it, so the mark keeps the reference's proportions whatever
/// the plot is scaled to.
const REFERENCE_PLOT_WIDTH: f32 = 770.0;

/// A named region of the frequency axis.
#[derive(Clone, Copy)]
pub struct FrequencyBand {
    pub title: &'static str,
    pub range_hz: (f32, f32),
}

pub fn export_svg(
    curves: &[PlotCurve<'_>],
    zoom: ZoomRegion,
    smooth_level: usize,
    label_positions: &BTreeMap<String, [f32; 2]>,
    dark_mode: bool,
    y_scale: f32,
    provenance: Provenance<'_>,
) -> String {
    // Wide and short, matching the on-screen plot, so an exported graph has the
    // same apparent slopes as the one it was exported from.
    const WIDTH: f32 = 1200.0;
    const DOMAIN_MARGIN: f32 = 72.0;
    const TOP_MARGIN: f32 = 24.0;
    let plot_rect = Rect::from_min_max(
        Pos2::new(DOMAIN_MARGIN, TOP_MARGIN),
        Pos2::new(
            WIDTH - DOMAIN_MARGIN,
            TOP_MARGIN + (WIDTH - 2.0 * DOMAIN_MARGIN) / GraphCanvas::PLOT_ASPECT_RATIO,
        ),
    );
    const HEIGHT: f32 =
        TOP_MARGIN + (WIDTH - 2.0 * DOMAIN_MARGIN) / GraphCanvas::PLOT_ASPECT_RATIO + 60.0;
    let transform = Transform::new(
        plot_rect,
        zoom.x_range(),
        scaled_y_range(y_range(curves), y_scale),
    );
    let palette = Palette::resolve(dark_mode);
    let (background, text_color, grid_color) = (
        svg_color(palette.surface_base),
        svg_color(palette.text_tertiary),
        svg_color(palette.border),
    );
    let axis_size = TypeStep::Caption.size();
    let label_size = TypeStep::Body.size();
    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{WIDTH:.0}" height="{HEIGHT:.0}" viewBox="0 0 {WIDTH:.0} {HEIGHT:.0}"><rect width="100%" height="100%" fill="{background}"/>"#
    );
    // Match the canvas: grid lines stay dense, labels thin out when they would
    // collide, so a narrow export is not a wall of overlapping tick text.
    const MIN_TICK_LABEL_GAP: f32 = 26.0;
    let mut last_label_x = f32::NEG_INFINITY;
    for &frequency in X_TICKS.iter().filter(|frequency| {
        **frequency >= transform.x_range.0 && **frequency <= transform.x_range.1
    }) {
        let x = transform.hz_to_x(frequency);
        let label = format_axis_frequency(frequency);
        svg.push_str(&format!(
            r#"<path d="M{x:.2} {y1:.2}V{y2:.2}" stroke="{grid_color}" stroke-width="{grid_width}"/>"#,
            y1 = plot_rect.top(),
            y2 = plot_rect.bottom(),
            grid_width = theme::graph::GRID_WIDTH,
        ));
        if x - last_label_x >= MIN_TICK_LABEL_GAP {
            last_label_x = x;
            svg.push_str(&format!(
                r#"<text x="{x:.2}" y="{label_y:.2}" fill="{text_color}" font-family="Open Sans, sans-serif" font-size="{axis_size}" text-anchor="middle">{label}</text>"#,
                label_y = plot_rect.bottom() + 24.0,
            ));
        }
    }
    let db_step = if transform.y_range.1 - transform.y_range.0 > 70.0 {
        10
    } else {
        5
    };
    let first_db = (transform.y_range.0 / db_step as f32).ceil() as i32 * db_step;
    let last_db = (transform.y_range.1 / db_step as f32).floor() as i32 * db_step;
    for db in (first_db..=last_db).step_by(db_step as usize) {
        let y = transform.db_to_y(db as f32);
        svg.push_str(&format!(
            r#"<path d="M{x1:.2} {y:.2}H{x2:.2}" stroke="{grid_color}" stroke-width="{grid_width}"/><text x="{label_x:.2}" y="{y:.2}" fill="{text_color}" font-family="Hack, monospace" font-size="{axis_size}" text-anchor="end" dominant-baseline="middle">{db}</text>"#,
            x1 = plot_rect.left(),
            x2 = plot_rect.right(),
            label_x = plot_rect.left() - 10.0,
            grid_width = theme::graph::GRID_WIDTH,
        ));
    }

    svg.push_str(&format!(
        r#"<defs><clipPath id="plot-clip"><rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}"/></clipPath></defs><g clip-path="url(#plot-clip)">"#,
        plot_rect.left(),
        plot_rect.top(),
        plot_rect.width(),
        plot_rect.height(),
    ));
    // The mark and the name, in the reference's group at a fifth of the usual
    // opacity, centred on the plot and drawn before the curves so they lie
    // over it.
    let watermark_scale = plot_rect.width() / REFERENCE_PLOT_WIDTH;
    let watermark_center = Pos2::new(
        plot_rect.center().x,
        plot_rect.center().y - 20.0 * watermark_scale,
    );
    svg.push_str(&format!(
        r#"<g opacity="0.2" transform="translate({:.2},{:.2})">"#,
        watermark_center.x, watermark_center.y,
    ));
    if let Some(png) = provenance.watermark.png {
        let side = 128.0 * watermark_scale;
        svg.push_str(&format!(
            r#"<image x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" xlink:href="data:image/png;base64,{}"/>"#,
            -side / 2.0,
            -side / 2.0,
            side,
            side,
            base64(png),
        ));
    }
    if let Some(text) = provenance.watermark.text {
        svg.push_str(&format!(
            r#"<text x="0" y="{:.2}" fill="{}" font-family="Open Sans, sans-serif" font-size="{:.2}" text-anchor="middle">{}</text>"#,
            70.0 * watermark_scale,
            svg_color(palette.text_secondary),
            28.0 * watermark_scale,
            escape_xml(text),
        ));
    }
    svg.push_str("</g>");
    let mut label_candidates = Vec::new();
    for curve in curves {
        let points: Vec<Pos2> = curve
            .points
            .iter()
            .map(|point| Pos2::new(transform.hz_to_x(point.hz), transform.db_to_y(point.db)))
            .collect();
        if let Some(last) = curve
            .points
            .iter()
            .rev()
            .find(|point| point.hz >= transform.x_range.0 && point.hz <= transform.x_range.1)
        {
            label_candidates.push((
                curve.label_key,
                Pos2::new(transform.hz_to_x(last.hz), transform.db_to_y(last.db)),
                Vec2::new(
                    curve.label.chars().count() as f32 * label_size * 0.56,
                    label_size * 1.3,
                ),
            ));
        }
        if points.len() < 2 {
            continue;
        }
        let path = svg_curve_path(&points, smooth_level > 0);
        let dash = if curve.dashed {
            r#" stroke-dasharray="6 4""#
        } else {
            ""
        };
        svg.push_str(&format!(
            r#"<path d="{path}" fill="none" stroke="{}" stroke-width="{}"{dash}/>"#,
            svg_color(theme::color(curve.color)),
            theme::graph::CURVE_WIDTH,
        ));
    }
    svg.push_str("</g>");

    // The provenance line the screen shows in the corner of the graph.
    if let Some(rig) = provenance.rig_description {
        svg.push_str(&format!(
            r#"<text x="{:.2}" y="{:.2}" fill="{text_color}" font-family="Hack, monospace" font-size="{:.2}" text-anchor="end">Measured on: {}</text>"#,
            plot_rect.right() - 4.0,
            plot_rect.top() + 10.0,
            TypeStep::Micro.size(),
            escape_xml(rig),
        ));
    }

    if !label_candidates.is_empty() {
        let automatic = place_labels(
            label_candidates
                .iter()
                .map(|(key, point, size)| ((*key).to_owned(), point.y, size.y))
                .collect(),
            plot_rect,
        );
        for (key, endpoint, size) in label_candidates {
            let position = label_positions.get(key).map_or_else(
                || {
                    Pos2::new(
                        endpoint.x + 6.0,
                        automatic.get(key).copied().unwrap_or(endpoint.y),
                    )
                },
                |normalized| {
                    Pos2::new(
                        plot_rect.left() + normalized[0].clamp(0.0, 1.0) * plot_rect.width(),
                        plot_rect.top() + normalized[1].clamp(0.0, 1.0) * plot_rect.height(),
                    )
                },
            );
            let position = clamp_label_position(position, size, plot_rect);
            let curve = curves.iter().find(|curve| curve.label_key == key).unwrap();
            svg.push_str(&format!(
                r#"<text x="{:.2}" y="{:.2}" fill="{}" font-family="Open Sans, sans-serif" font-size="{label_size}" font-weight="700">{}</text>"#,
                position.x,
                position.y + label_size,
                svg_color(theme::color(curve.color)),
                escape_xml(curve.label),
            ));
        }
    }
    svg.push_str("</svg>");
    svg
}

fn svg_color(color: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b())
}

/// Base64 for the mark the export embeds as a data URI.
///
/// The SVG has to carry the image itself: a file next to it would be a second
/// thing to lose, and a graph that is exported on its own should still show
/// what it is.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        let bits = (u32::from(chunk[0]) << 16) | (u32::from(second) << 8) | u32::from(third);
        out.push(ALPHABET[(bits >> 18) as usize & 63] as char);
        out.push(ALPHABET[(bits >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(bits >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[bits as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

impl GraphCanvas<'_> {
    pub fn show(&self, ui: &mut Ui, curves: &[PlotCurve<'_>]) -> GraphInteraction {
        // The graph keeps the reference's framing instead of stretching to
        // fill the panel: the same dB range drawn over twice the space reads
        // as twice as dramatic, and a tall window is exactly that case. The
        // widget still claims the whole panel, so the graph is the surface the
        // user interacts with; the plot inside it is sized by the reference's
        // rule and centred.
        let panel = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(panel, Sense::hover());
        let window_height = ui.ctx().viewport_rect().height();
        let (plot_width, plot_height) = Self::plot_size(panel, window_height);
        let visuals = ui.visuals();
        let palette = Palette::resolve(visuals.dark_mode);
        // The graph is the darkest surface in the application so saturated
        // measurement colors sit on the quietest ground available.
        let background = palette.surface_base;
        let text_color = palette.text_primary;
        let axis_text = palette.text_tertiary;
        let grid_color = palette.border;
        let accent = palette.accent;
        let axis_font = || theme::font(TypeStep::Caption);
        let label_font = || theme::font_bold(TypeStep::Body);
        // The reference centres the graph in its column (`margin: 16px auto 0
        // auto`), so a graph narrower than the column sits in the middle of
        // it rather than against the edge, and keeps it at the top of the
        // column, with the manager following underneath.
        let space_width = (rect.width() - Self::INSET_LEFT - Self::INSET_RIGHT).max(0.0);
        let plot_left = rect.left() + Self::INSET_LEFT + (space_width - plot_width) / 2.0;
        let plot_rect = Rect::from_min_max(
            // Left room for the dB scale, top for the rig note, and enough
            // underneath for the frequency ticks plus the named bands below them.
            Pos2::new(plot_left, rect.top() + Self::INSET_TOP),
            Pos2::new(
                plot_left + plot_width,
                rect.top() + Self::INSET_TOP + plot_height,
            ),
        );
        let (y_min, y_max) = scaled_y_range(y_range(curves), self.y_scale);
        let transform = Transform::new(plot_rect, self.zoom.x_range(), (y_min, y_max));
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, background);

        let plot_painter = painter.with_clip_rect(plot_rect);
        // Provenance caption, top-right, in the recessive type colour so it
        // never competes with the curves.
        if let Some(rig) = self.rig_description {
            painter.text(
                Pos2::new(plot_rect.right() - 4.0, rect.top() + 2.0),
                Align2::RIGHT_TOP,
                format!("Measured on: {rig}"),
                theme::font_mono(TypeStep::Micro),
                palette.text_tertiary,
            );
        }
        if let Some((label, (low_hz, high_hz))) = self.tutorial_band {
            let band_rect = Rect::from_min_max(
                Pos2::new(
                    transform.hz_to_x(low_hz).max(plot_rect.left()),
                    plot_rect.top(),
                ),
                Pos2::new(
                    transform.hz_to_x(high_hz).min(plot_rect.right()),
                    plot_rect.bottom(),
                ),
            );
            if band_rect.is_positive() {
                plot_painter.rect_filled(band_rect, 0.0, palette.warning.gamma_multiply(0.18));
                painter.text(
                    band_rect.center_top() + Vec2::new(0.0, 5.0),
                    Align2::CENTER_TOP,
                    label,
                    theme::font(TypeStep::Caption),
                    palette.warning,
                );
            }
        }
        // Labels are density-aware: keep every grid line, but drop a label when it
        // would sit closer than MIN_TICK_LABEL_GAP to the previous one. Without
        // this, a narrow window overlaps the tick text into an unreadable band.
        const MIN_TICK_LABEL_GAP: f32 = 26.0;
        let mut last_label_x = f32::NEG_INFINITY;
        for &frequency in X_TICKS.iter().filter(|frequency| {
            **frequency >= transform.x_range.0 && **frequency <= transform.x_range.1
        }) {
            let x = transform.hz_to_x(frequency);
            plot_painter.line_segment(
                [
                    Pos2::new(x, plot_rect.top()),
                    Pos2::new(x, plot_rect.bottom()),
                ],
                Stroke::new(theme::graph::GRID_WIDTH, grid_color),
            );
            let label = format_axis_frequency(frequency);
            if x - last_label_x >= MIN_TICK_LABEL_GAP {
                last_label_x = x;
                painter.text(
                    Pos2::new(x, plot_rect.bottom() + 8.0),
                    Align2::CENTER_TOP,
                    label,
                    axis_font(),
                    axis_text,
                );
            }
        }

        // Named frequency regions under the axis. These stay visible whether or
        // not the tutorial is running, so the graph reads as a map of where each
        // feature lives; the band the tutorial is currently on is picked out.
        let band_y = plot_rect.bottom() + 26.0;
        for band in self.bands {
            let (low, high) = band.range_hz;
            if high < transform.x_range.0 || low > transform.x_range.1 {
                continue;
            }
            let start = transform.hz_to_x(low).max(plot_rect.left());
            let end = transform.hz_to_x(high).min(plot_rect.right());
            // A band only earns its label if the band is wider than the label
            // itself. On a narrow viewport the higher bands collapse to a few
            // pixels, and printing their names there overlaps the neighbours into
            // an unreadable smear.
            let label_width = band.title.len() as f32 * TypeStep::Micro.size() * 0.58;
            if end - start < label_width + 6.0 {
                continue;
            }
            let active = self.tutorial_band.map(|(title, _)| title) == Some(band.title);
            let color = if active {
                palette.accent
            } else {
                palette.text_tertiary
            };
            painter.text(
                Pos2::new((start + end) * 0.5, band_y),
                Align2::CENTER_TOP,
                band.title,
                if active {
                    theme::font_bold(TypeStep::Micro)
                } else {
                    theme::font(TypeStep::Micro)
                },
                color,
            );
        }

        let db_step = if y_max - y_min > 70.0 { 10 } else { 5 };
        let first_db = (y_min / db_step as f32).ceil() as i32 * db_step;
        let last_db = (y_max / db_step as f32).floor() as i32 * db_step;
        for db in (first_db..=last_db).step_by(db_step as usize) {
            let y = transform.db_to_y(db as f32);
            plot_painter.line_segment(
                [
                    Pos2::new(plot_rect.left(), y),
                    Pos2::new(plot_rect.right(), y),
                ],
                Stroke::new(theme::graph::GRID_WIDTH, grid_color),
            );
            painter.text(
                Pos2::new(plot_rect.left() - 10.0, y),
                Align2::RIGHT_CENTER,
                format!("{db}"),
                theme::font_mono(TypeStep::Caption),
                axis_text,
            );
        }

        // The mark and the name go down first, so the curves are drawn over
        // them, faded back far enough to read as background.
        self.draw_watermark(&plot_painter, plot_rect);

        let mut label_candidates = Vec::new();
        let mut screen_curves = Vec::with_capacity(curves.len());
        for curve in curves {
            let points: Vec<Pos2> = curve
                .points
                .iter()
                .map(|point| Pos2::new(transform.hz_to_x(point.hz), transform.db_to_y(point.db)))
                .collect();
            let smooth = self.smooth_level > 0;
            Self::draw_curve(
                &plot_painter,
                &points,
                theme::color(curve.color),
                curve.dashed,
                smooth,
                theme::graph::CURVE_WIDTH,
            );
            screen_curves.push(sample_curve_path(&points, smooth));
            if self.show_labels {
                if let Some(point) = curve
                    .points
                    .iter()
                    .rev()
                    .find(|point| {
                        point.hz >= transform.x_range.0 && point.hz <= transform.x_range.1
                    })
                    .map(|point| {
                        Pos2::new(transform.hz_to_x(point.hz), transform.db_to_y(point.db))
                    })
                {
                    let galley = painter.layout_no_wrap(
                        curve.label.to_owned(),
                        label_font(),
                        theme::color(curve.color),
                    );
                    label_candidates.push((
                        curve.label_key,
                        point,
                        galley.size(),
                        curve.color,
                        galley,
                    ));
                }
            }
        }

        let default_positions = place_labels(
            label_candidates
                .iter()
                .map(|(key, point, size, _, _)| ((*key).to_owned(), point.y, size.y))
                .collect(),
            plot_rect,
        );
        let mut updated_label_positions = BTreeMap::new();
        for (index, (key, endpoint, size, color, galley)) in
            label_candidates.into_iter().enumerate()
        {
            let normalized = self.label_positions.get(key).copied();
            let mut top_left = normalized.map_or_else(
                || {
                    if self.label_layout == LabelLayout::CurveEnd {
                        let y = default_positions.get(key).copied().unwrap_or(endpoint.y);
                        Pos2::new((endpoint.x + 6.0).min(plot_rect.right() - size.x), y)
                    } else {
                        // Grouped: stack the first few labels in one corner as a
                        // legend. Any beyond the cap keep following their curve,
                        // so a large selection still gets labelled.
                        corner_label_position(
                            self.label_layout,
                            index,
                            size,
                            plot_rect,
                            theme::TypeStep::Body.size() + 8.0,
                        )
                    }
                },
                |position| {
                    Pos2::new(
                        plot_rect.left() + position[0].clamp(0.0, 1.0) * plot_rect.width(),
                        plot_rect.top() + position[1].clamp(0.0, 1.0) * plot_rect.height(),
                    )
                },
            );
            top_left = clamp_label_position(top_left, size, plot_rect);
            let label_rect = Rect::from_min_size(top_left, size);
            let label_response = ui.interact(
                label_rect.expand(3.0),
                Id::new(("curve-label", key)),
                Sense::drag(),
            );
            if label_response.drag_started() {
                ui.ctx()
                    .data_mut(|data| data.insert_temp(label_response.id, top_left));
            }
            if label_response.dragged() {
                let drag_origin = ui
                    .ctx()
                    .data(|data| data.get_temp::<Pos2>(label_response.id))
                    .unwrap_or(top_left);
                top_left = clamp_label_position(
                    drag_origin + label_response.drag_delta(),
                    size,
                    plot_rect,
                );
            }
            if label_response.drag_stopped() {
                ui.ctx()
                    .data_mut(|data| data.remove::<Pos2>(label_response.id));
            }
            painter.galley(top_left, galley, theme::color(color));
            if normalized.is_some() || label_response.dragged() {
                updated_label_positions.insert(
                    key.to_owned(),
                    [
                        ((top_left.x - plot_rect.left()) / plot_rect.width()).clamp(0.0, 1.0),
                        ((top_left.y - plot_rect.top()) / plot_rect.height()).clamp(0.0, 1.0),
                    ],
                );
            }
        }

        let mut updated_y_scale = None;
        let handle_rect = Rect::from_center_size(
            Pos2::new(rect.left() + 25.0, plot_rect.center().y),
            Vec2::new(24.0, 42.0),
        );
        let handle_response = ui.interact(
            handle_rect,
            Id::new("graph-y-scale-handle"),
            Sense::click_and_drag(),
        );
        painter.line_segment(
            [
                Pos2::new(handle_rect.center().x, handle_rect.top() + 5.0),
                Pos2::new(handle_rect.center().x, handle_rect.bottom() - 5.0),
            ],
            Stroke::new(theme::graph::INSPECTOR_WIDTH, palette.border_strong),
        );
        painter.circle_filled(
            handle_rect.center(),
            7.0,
            if handle_response.hovered() || handle_response.dragged() {
                accent
            } else {
                palette.border_strong
            },
        );
        if handle_response.drag_started() {
            ui.ctx()
                .data_mut(|data| data.insert_temp(handle_response.id, self.y_scale));
        }
        if handle_response.dragged() {
            let start_scale = ui
                .ctx()
                .data(|data| data.get_temp::<f32>(handle_response.id))
                .unwrap_or(self.y_scale);
            updated_y_scale = Some(
                (start_scale * (handle_response.drag_delta().y / 120.0).exp()).clamp(0.35, 4.0),
            );
        }
        if handle_response.drag_stopped() {
            ui.ctx()
                .data_mut(|data| data.remove::<f32>(handle_response.id));
        } else if handle_response.double_clicked() {
            updated_y_scale = Some(1.0);
        }
        if handle_response.hovered() {
            handle_response
                .clone()
                .on_hover_text("Drag to scale the dB axis; double-click to reset.");
        }

        let hover_hz = response.hover_pos().and_then(|position| {
            plot_rect
                .contains(position)
                .then(|| transform.x_at(position.x))
        });
        if let Some(position) = response
            .hover_pos()
            .filter(|position| plot_rect.contains(*position))
        {
            if let Some(index) = nearest_curve_index(&screen_curves, position, 14.0) {
                Self::draw_curve_lines(
                    &plot_painter,
                    &screen_curves[index],
                    theme::color(curves[index].color),
                    curves[index].dashed,
                    theme::graph::CURVE_WIDTH_HOVER,
                );
            }
        }
        if self.inspect_mode {
            if let Some(frequency) = hover_hz {
                let x = transform.hz_to_x(frequency);
                plot_painter.line_segment(
                    [
                        Pos2::new(x, plot_rect.top()),
                        Pos2::new(x, plot_rect.bottom()),
                    ],
                    Stroke::new(theme::graph::INSPECTOR_WIDTH, accent),
                );
                painter.text(
                    Pos2::new(x, plot_rect.top() + 5.0),
                    Align2::CENTER_TOP,
                    format_frequency(frequency),
                    theme::font_bold(TypeStep::Caption),
                    accent,
                );
                // Readouts are measurements: monospace keeps the dB values in
                // column so curves can be compared digit by digit.
                let readout = curves
                    .iter()
                    .filter_map(|curve| {
                        nearest_point(curve.points, frequency).map(|point| (curve, point))
                    })
                    .map(|(curve, point)| format!("{}  {:>6.1} dB", curve.label, point.db))
                    .collect::<Vec<_>>()
                    .join("\n");
                let readout_rect = Rect::from_min_size(
                    plot_rect.left_top() + Vec2::new(8.0, 8.0),
                    painter
                        .layout_no_wrap(
                            readout.clone(),
                            theme::font_mono(TypeStep::Caption),
                            text_color,
                        )
                        .size(),
                );
                painter.rect_filled(
                    readout_rect.expand(6.0),
                    theme::RADIUS_CONTROL,
                    palette.surface_raised,
                );
                painter.rect_stroke(
                    readout_rect.expand(6.0),
                    theme::RADIUS_CONTROL,
                    Stroke::new(1.0, palette.border),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    readout_rect.min,
                    Align2::LEFT_TOP,
                    readout,
                    theme::font_mono(TypeStep::Caption),
                    text_color,
                );
            }
        }

        GraphInteraction {
            hover_hz,
            label_positions: updated_label_positions,
            y_scale: updated_y_scale,
        }
    }

    /// Left room for the dB scale.
    const INSET_LEFT: f32 = 64.0;
    /// Top room for the rig note.
    const INSET_TOP: f32 = 14.0;
    /// Right edge breathing room.
    const INSET_RIGHT: f32 = 18.0;
    /// Room underneath for the frequency ticks plus the named bands below them.
    const INSET_BOTTOM: f32 = 54.0;
    /// Floors that keep the axis and band labels fitting on tiny viewports.
    const MIN_PLOT_WIDTH: f32 = 120.0;
    const MIN_PLOT_HEIGHT: f32 = 120.0;

    /// The plot size inside a panel: everything the margins leave.
    ///
    /// The widget claims the whole panel and the plot stretches to fill it, so
    /// a viewport taller than the reference proportion grows the plot instead
    /// of leaving a dead band between the graph and the manager below it.
    /// The size of the plot in the space it has.
    ///
    /// The reference scales its graph with the window rather than with the
    /// space it is given: the graph's width is capped against the window's
    /// height (`max-width: calc((100vh - 300px) * 2.2)`, and the smaller of
    /// that and `calc(50vh * 2.2)`), and the graph keeps its own
    /// proportions instead of stretching. A tall window therefore draws a
    /// wide, short graph with room below it, as the reference does, rather
    /// than the same dB range stretched over the whole viewport.
    ///
    /// When there is not enough room for the graph's proportions, it shrinks
    /// to fit rather than being squashed.
    pub fn plot_size(panel: egui::Vec2, window_height: f32) -> (f32, f32) {
        let space = (
            (panel.x - Self::INSET_LEFT - Self::INSET_RIGHT).max(Self::MIN_PLOT_WIDTH),
            (panel.y - Self::INSET_TOP - Self::INSET_BOTTOM).max(Self::MIN_PLOT_HEIGHT),
        );
        // The reference's cap on the graph's width: `max-width: calc((100vh -
        // 300px) * 2.2)`. It writes that after `calc(50vh * 2.2)`, so the
        // later one is the cap that applies.
        let cap = 2.2 * (window_height - 300.0);
        let mut width = space.0;
        if cap > 0.0 {
            width = width.min(cap);
        }
        let mut height = width / Self::PLOT_ASPECT_RATIO;
        if height > space.1 {
            height = space.1;
            width = height * Self::PLOT_ASPECT_RATIO;
        }
        (
            width.max(Self::MIN_PLOT_WIDTH),
            height.max(Self::MIN_PLOT_HEIGHT),
        )
    }

    /// How many units wide the plot is for every unit of height. The reference
    /// tool draws a wide, short graph, so the SVG export keeps that framing.
    /// The live canvas no longer caps itself to this ratio: on viewports taller
    /// than the ratio implies, the cap left dead space and the user reads it as
    /// wasted room.
    const PLOT_ASPECT_RATIO: f32 = 3.4;

    /// The mark and the name, faded back behind the curves.
    ///
    /// The reference draws them in one group at a fifth of the usual opacity,
    /// centred on the plot. egui's fonts are not scalable, so the name uses the
    /// app's type scale rather than the reference's size; the mark keeps the
    /// reference's proportions exactly.
    fn draw_watermark(&self, painter: &egui::Painter, plot_rect: Rect) {
        if self.watermark.texture.is_none() && self.watermark.text.is_none() {
            return;
        }
        let scale = plot_rect.width() / REFERENCE_PLOT_WIDTH;
        let center = plot_rect.center() - Vec2::new(0.0, 20.0 * scale);
        let tint = Color32::from_white_alpha(51);
        if let Some(texture) = self.watermark.texture {
            let side = 128.0 * scale;
            painter.image(
                texture,
                Rect::from_center_size(center, Vec2::new(side, side)),
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                tint,
            );
        }
        if let Some(text) = self.watermark.text {
            painter.text(
                center + Vec2::new(0.0, 70.0 * scale),
                Align2::CENTER_CENTER,
                text,
                theme::font(TypeStep::Title),
                tint,
            );
        }
    }

    fn draw_curve(
        painter: &egui::Painter,
        points: &[Pos2],
        color: Color32,
        dashed: bool,
        smooth: bool,
        width: f32,
    ) {
        let controls = curve_control_points(points, smooth);
        if controls.is_empty() {
            for (index, pair) in points.windows(2).enumerate() {
                if !dashed || index % 2 == 0 {
                    painter.line_segment(pair.try_into().unwrap(), Stroke::new(width, color));
                }
            }
            return;
        }

        for (index, [start, control1, control2, end]) in controls.into_iter().enumerate() {
            if dashed && index % 2 != 0 {
                continue;
            }
            painter.add(egui::Shape::CubicBezier(
                egui::epaint::CubicBezierShape::from_points_stroke(
                    [start, control1, control2, end],
                    false,
                    Color32::TRANSPARENT,
                    Stroke::new(width, color),
                ),
            ));
        }
    }

    fn draw_curve_lines(
        painter: &egui::Painter,
        points: &[Pos2],
        color: Color32,
        dashed: bool,
        width: f32,
    ) {
        for (index, pair) in points.windows(2).enumerate() {
            if !dashed || index % 2 == 0 {
                painter.line_segment([pair[0], pair[1]], Stroke::new(width, color));
            }
        }
    }
}

fn svg_curve_path(points: &[Pos2], smooth: bool) -> String {
    if points.is_empty() {
        return String::new();
    }
    let mut path = format!("M {:.2} {:.2}", points[0].x, points[0].y);
    let controls = curve_control_points(points, smooth);
    if controls.is_empty() {
        for point in points.iter().skip(1) {
            path.push_str(&format!(" L {:.2} {:.2}", point.x, point.y));
        }
    } else {
        for [_, control1, control2, end] in controls {
            path.push_str(&format!(
                " C {:.2} {:.2} {:.2} {:.2} {:.2} {:.2}",
                control1.x, control1.y, control2.x, control2.y, end.x, end.y,
            ));
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::{export_svg, Provenance, Watermark};
    use egui::{pos2, Rect};
    use squiglab_core::data::FRPoint;
    use squiglab_core::graph::{
        clamp_label_position, curve_control_points, format_axis_frequency, nearest_curve_index,
        place_labels, scaled_y_range, PlotCurve, Transform, ZoomRegion,
    };
    use std::collections::BTreeMap;

    #[test]
    fn logarithmic_frequency_mapping_round_trips() {
        let transform = Transform::new(
            Rect::from_min_max(pos2(0.0, 0.0), pos2(900.0, 400.0)),
            (20.0, 20000.0),
            (20.0, 120.0),
        );
        for frequency in [20.0, 100.0, 1000.0, 20000.0] {
            assert!((transform.x_at(transform.hz_to_x(frequency)) / frequency - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn zoom_ranges_cover_the_expected_bands() {
        assert_eq!(ZoomRegion::Bass.x_range(), (20.0, 400.0));
        assert_eq!(ZoomRegion::Treble.x_range(), (1000.0, 20000.0));
    }

    #[test]
    fn automatic_label_placement_stays_inside_the_plot_and_separates_labels() {
        let plot = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 100.0));
        let positions = place_labels(
            vec![
                ("top".to_owned(), 1.0, 12.0),
                ("middle".to_owned(), 48.0, 12.0),
                ("bottom".to_owned(), 99.0, 12.0),
            ],
            plot,
        );
        let tops: Vec<f32> = ["top", "middle", "bottom"]
            .iter()
            .map(|key| positions[*key])
            .collect();
        assert!(tops[0] >= plot.top());
        assert!(tops[2] + 12.0 <= plot.bottom());
        assert!(tops.windows(2).all(|pair| pair[1] - pair[0] >= 14.0));
    }

    #[test]
    fn target_curves_do_not_stretch_the_plotted_db_range() {
        // A target reaching 14.7 dB at 20 kHz must not drag the domain down to
        // 10 dB and compress the measurements it is being compared against.
        let _measurement = PlotCurve {
            label: "M",
            label_key: "m",
            points: &[
                FRPoint { hz: 20.0, db: 42.0 },
                FRPoint {
                    hz: 20000.0,
                    db: 72.0,
                },
            ],
            color: squiglab_core::color::Rgb::new(255, 255, 255),
            dashed: false,
            target: false,
        };
        let target = PlotCurve {
            label: "T",
            label_key: "t",
            points: &[
                FRPoint { hz: 20.0, db: 40.0 },
                FRPoint {
                    hz: 20000.0,
                    db: 14.7,
                },
            ],
            color: squiglab_core::color::Rgb::new(128, 128, 128),
            dashed: true,
            target: true,
        };
        let measurement_points: [FRPoint; 2] = [
            FRPoint { hz: 20.0, db: 42.0 },
            FRPoint {
                hz: 20000.0,
                db: 72.0,
            },
        ];
        let measurement = PlotCurve {
            label: "M",
            label_key: "m",
            points: &measurement_points,
            color: squiglab_core::color::Rgb::new(255, 255, 255),
            dashed: false,
            target: false,
        };
        let without_target = super::y_range(&[measurement]);
        let with_target = super::y_range(&[measurement, target]);
        assert_eq!(with_target, without_target);
        assert!(
            with_target.0 > 30.0,
            "target stretched the domain: {with_target:?}"
        );
    }

    #[test]
    fn only_targets_still_produces_a_usable_range() {
        let target = PlotCurve {
            label: "T",
            label_key: "t",
            points: &[
                FRPoint { hz: 20.0, db: 40.0 },
                FRPoint {
                    hz: 20000.0,
                    db: 14.7,
                },
            ],
            color: squiglab_core::color::Rgb::new(128, 128, 128),
            dashed: true,
            target: true,
        };
        let range = super::y_range(&[target]);
        assert!(range.0 <= 14.7 && range.1 >= 40.0);
    }

    #[test]
    fn the_plot_is_wider_than_it_is_tall_like_the_reference_tool() {
        let aspect = crate::graph::GraphCanvas::PLOT_ASPECT_RATIO;
        assert!(
            (3.0..=3.8).contains(&aspect),
            "aspect ratio {aspect} does not match the reference tool's framing"
        );
    }

    #[test]
    fn the_plot_keeps_the_reference_proportions_on_a_tall_viewport() {
        // A tall viewport used to stretch the plot over the whole panel, so
        // the same dB range was drawn over far more space than the reference
        // draws it over and every slope read as more dramatic. The graph now
        // keeps the reference's proportions, and its width is capped against
        // the window's height as the reference caps it.
        let panel = egui::vec2(1150.0, 1650.0);
        let (width, height) = super::GraphCanvas::plot_size(panel, 2000.0);
        let aspect = width / height;
        assert!(
            (3.0..=3.8).contains(&aspect),
            "aspect ratio {aspect} does not match the reference tool's framing"
        );
        // The width cap is the reference's: the graph never grows past 2.2
        // units across for every unit of window height, less the 300 units
        // the reference keeps for its chrome.
        let (width, _) = super::GraphCanvas::plot_size(egui::vec2(4000.0, 1650.0), 1000.0);
        assert!(
            width <= 2.2 * 700.0,
            "the graph is {width} wide in a 1000-unit-tall window"
        );
        // A tall window leaves the cap wide open, so the graph fills its
        // column as the reference's does.
        let (width, _) = super::GraphCanvas::plot_size(egui::vec2(1650.0, 1650.0), 1200.0);
        assert_eq!(
            width,
            1650.0 - super::GraphCanvas::INSET_LEFT - super::GraphCanvas::INSET_RIGHT
        );
    }

    #[test]
    fn the_plot_shrinks_rather_than_being_squashed() {
        // When there is not enough room for the graph's proportions, it
        // shrinks to fit instead of being squashed into the space.
        let (width, height) = super::GraphCanvas::plot_size(egui::vec2(400.0, 100.0), 600.0);
        assert_eq!(height, super::GraphCanvas::MIN_PLOT_HEIGHT);
        assert!(width <= 400.0);
        // Tiny viewports stop at the floor rather than collapsing the axes.
        let (_, height) = super::GraphCanvas::plot_size(egui::vec2(200.0, 80.0), 600.0);
        assert_eq!(height, super::GraphCanvas::MIN_PLOT_HEIGHT);
    }

    #[test]
    fn grouped_labels_stack_in_the_corner_without_overlapping() {
        let plot = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 600.0));
        let corner = crate::graph::LabelLayout::CornerBottomLeft;
        let tops: Vec<egui::Pos2> = (0..4)
            .map(|index| {
                crate::graph::corner_label_position(
                    corner,
                    index,
                    egui::vec2(60.0, 18.0),
                    plot,
                    18.0,
                )
            })
            .collect();
        // Inside the plot, inset from the left and the bottom, one line apart.
        for position in &tops {
            assert!(position.x >= plot.left());
            assert!(position.y >= plot.top());
            assert!(position.x + 60.0 <= plot.right());
            assert!(position.y + 18.0 <= plot.bottom());
        }
        assert!(
            tops.windows(2).all(|pair| pair[0].y - pair[1].y >= 18.0),
            "labels overlapped: {tops:?}"
        );
        // Highest label is closest to the bottom, matching the reference tool.
        assert!(tops[0].y > tops[3].y);
    }

    #[test]
    fn labels_desiring_nearby_positions_still_do_not_overlap() {
        // Two curves ending near each other request almost the same y. The
        // second must be pushed down rather than drawn on top of the first.
        let plot = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 600.0));
        let positions = place_labels(
            vec![
                ("first".to_owned(), 300.0, 18.0),
                ("second".to_owned(), 305.4, 18.0),
                ("third".to_owned(), 310.8, 18.0),
            ],
            plot,
        );
        let tops: Vec<f32> = ["first", "second", "third"]
            .iter()
            .map(|key| positions[*key])
            .collect();
        assert!(
            tops.windows(2).all(|pair| pair[1] - pair[0] >= 18.0),
            "labels overlapped: {tops:?}"
        );
    }

    #[test]
    fn dragged_label_anchor_is_clamped_to_plot_bounds() {
        let plot = Rect::from_min_max(pos2(10.0, 20.0), pos2(110.0, 120.0));
        assert_eq!(
            clamp_label_position(pos2(-10.0, 500.0), egui::vec2(30.0, 15.0), plot),
            pos2(10.0, 105.0)
        );
    }

    #[test]
    fn vertical_scale_expands_and_contracts_around_the_same_center() {
        assert_eq!(scaled_y_range((20.0, 80.0), 1.0), (20.0, 80.0));
        assert_eq!(scaled_y_range((20.0, 80.0), 2.0), (-10.0, 110.0));
        assert_eq!(scaled_y_range((20.0, 80.0), 0.5), (35.0, 65.0));
        assert_eq!(scaled_y_range((20.0, 80.0), 0.0), (39.5, 60.5));
    }

    #[test]
    fn hover_highlight_selects_nearest_curve_within_threshold() {
        let curves = vec![
            vec![pos2(0.0, 0.0), pos2(100.0, 0.0)],
            vec![pos2(0.0, 30.0), pos2(100.0, 30.0)],
        ];
        assert_eq!(
            nearest_curve_index(&curves, pos2(50.0, 27.0), 14.0),
            Some(1)
        );
        assert_eq!(nearest_curve_index(&curves, pos2(50.0, 15.0), 5.0), None);
    }

    #[test]
    fn svg_export_contains_axes_curves_labels_and_escaped_text() {
        let points = [
            FRPoint {
                hz: 100.0,
                db: 60.0,
            },
            FRPoint {
                hz: 1000.0,
                db: 65.0,
            },
        ];
        let curves = [
            PlotCurve {
                label: "IEM <A & B>",
                label_key: "measurement",
                points: &points,
                color: squiglab_core::color::Rgb::new(12, 34, 56),
                dashed: false,
                target: false,
            },
            PlotCurve {
                label: "Target",
                label_key: "target",
                points: &points,
                color: squiglab_core::color::Rgb::new(90, 100, 110),
                dashed: true,
                target: true,
            },
        ];
        let svg = export_svg(
            &curves,
            ZoomRegion::Full,
            5,
            &BTreeMap::new(),
            true,
            1.0,
            Provenance::default(),
        );
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<path"));
        assert!(svg.contains("#0c2238"));
        assert!(svg.contains("stroke-dasharray=\"6 4\""));
        assert!(svg.contains(r#"clip-path="url(#plot-clip)""#));
        assert!(svg.contains(r#"<clipPath id="plot-clip">"#));
        assert!(svg.contains("IEM &lt;A &amp; B&gt;"));
        assert!(svg.contains("20k"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn svg_export_carries_the_mark_the_name_and_the_rig_line() {
        // The reference stamps all three into the graph it exports
        // (`web/config.js`, `watermark()`), so an exported graph says what it
        // is and where it came from.
        let points = [
            FRPoint {
                hz: 100.0,
                db: 60.0,
            },
            FRPoint {
                hz: 1000.0,
                db: 65.0,
            },
        ];
        let curves = [PlotCurve {
            label: "Curve",
            label_key: "curve",
            points: &points,
            color: squiglab_core::color::Rgb::new(255, 255, 255),
            dashed: false,
            target: false,
        }];
        let svg = export_svg(
            &curves,
            ZoomRegion::Full,
            5,
            &BTreeMap::new(),
            true,
            1.0,
            Provenance {
                watermark: Watermark {
                    png: Some(b"not really a png"),
                    texture: None,
                    text: Some("SquigLab"),
                },
                rig_description: Some("clone IEC 711"),
            },
        );
        // The mark travels inside the file, as a data URI, so the SVG is still
        // one file to hand over.
        assert!(svg.contains("data:image/png;base64,"));
        // The name is printed under the mark, in the faded group.
        assert!(svg.contains("SquigLab"));
        assert!(svg.contains(r#"<g opacity="0.2""#));
        // The rig line the screen shows in the corner.
        assert!(svg.contains("Measured on: clone IEC 711"));
    }

    #[test]
    fn svg_export_respects_zoom_and_vertical_scale() {
        let points = [
            FRPoint {
                hz: 100.0,
                db: 40.0,
            },
            FRPoint {
                hz: 1000.0,
                db: 60.0,
            },
        ];
        let curves = [PlotCurve {
            label: "Curve",
            label_key: "curve",
            points: &points,
            color: squiglab_core::color::Rgb::new(255, 255, 255),
            dashed: false,
            target: false,
        }];
        let full = export_svg(
            &curves,
            ZoomRegion::Full,
            5,
            &BTreeMap::new(),
            false,
            1.0,
            Provenance::default(),
        );
        let zoomed = export_svg(
            &curves,
            ZoomRegion::Bass,
            5,
            &BTreeMap::new(),
            false,
            2.0,
            Provenance::default(),
        );

        assert_ne!(full, zoomed);
        assert!(full.contains("<svg"));
        assert!(zoomed.contains("<svg"));
        assert!(zoomed.contains("clip-path=\"url(#plot-clip)\""));
        assert!(zoomed.contains("stroke=\"#ffffff\""));
    }

    #[test]
    fn graph_curve_modes_generate_natural_and_cardinal_beziers() {
        let points = [
            pos2(0.0, 0.0),
            pos2(1.0, 4.0),
            pos2(2.0, 0.0),
            pos2(3.0, 2.0),
        ];
        let natural = curve_control_points(&points, true);
        let cardinal = curve_control_points(&points, false);

        assert_eq!(natural.len(), points.len() - 1);
        assert_eq!(cardinal.len(), points.len() - 1);
        assert_eq!(natural[0][0], points[0]);
        assert_eq!(natural[0][3], points[1]);
        assert_ne!(natural[1][1], cardinal[1][1]);
    }

    #[test]
    fn svg_exports_cubic_paths_when_curve_smoothing_is_enabled() {
        let points = [
            FRPoint {
                hz: 100.0,
                db: 40.0,
            },
            FRPoint {
                hz: 500.0,
                db: 60.0,
            },
            FRPoint {
                hz: 1000.0,
                db: 45.0,
            },
        ];
        let curves = [PlotCurve {
            label: "Curve",
            label_key: "curve",
            points: &points,
            color: squiglab_core::color::Rgb::new(255, 255, 255),
            dashed: false,
            target: false,
        }];

        let svg = export_svg(
            &curves,
            ZoomRegion::Full,
            1,
            &BTreeMap::new(),
            false,
            1.0,
            Provenance::default(),
        );
        assert!(svg.contains(" C "));
    }

    #[test]
    fn axis_endpoint_labels_include_units() {
        assert_eq!(format_axis_frequency(20.0), "20Hz");
        assert_eq!(format_axis_frequency(20000.0), "20kHz");
        assert_eq!(format_axis_frequency(1000.0), "1k");
    }
}
