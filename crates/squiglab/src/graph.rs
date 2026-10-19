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
    let height = (ui.available_height() - 40.0)
        .min(by_height / metrics::GRAPH_ASPECT)
        .max(160.0);
    let width = (height * metrics::PLOT_ASPECT_RATIO).min(ui.available_width());
    let height = width / metrics::PLOT_ASPECT_RATIO;

    let available = ui.available_width();
    ui.horizontal(|ui| {
        ui.add_space((available - width).max(0.0) / 2.0);
        let (response, painter) =
            ui.allocate_painter(Vec2::new(width, height), Sense::click_and_drag());
        let rect = response.rect;
        painter.rect_filled(rect, 0.0, palette.graph);
        draw(app, &painter, rect);
    });
}

/// Draw the graph into a rectangle.
fn draw(app: &App, painter: &egui::Painter, rect: Rect) {
    let curves = plot_curves(app);

    // The plot's own area, inside the box: room for the dB scale on the left
    // and for the frequency axis and the band names below.
    let plot = Rect::from_min_max(
        Pos2::new(rect.left() + 44.0, rect.top() + 10.0),
        Pos2::new(rect.right() - 10.0, rect.bottom() - 46.0),
    );
    if plot.width() <= 0.0 || plot.height() <= 0.0 {
        return;
    }

    let (minimum, maximum) = y_range(&curves);
    let (minimum, maximum) = scaled_y_range((minimum, maximum), app.session.y_scale);
    let zoom = app.session.zoom;
    let transform = Transform::new(plot, zoom.x_range(), (minimum, maximum));

    grid(painter, plot, &transform, &minimum, &maximum);
    band_names(painter, plot, &transform);
    curves_into(painter, app, &curves, &transform);
    labels(painter, app, &curves, &transform, plot);
    watermark(painter, plot);
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
    if !app.show_labels {
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
        painter.text(
            Pos2::new(plot.left() + 12.0, *top),
            Align2::LEFT_TOP,
            curve.label,
            font.clone(),
            color,
        );
    }
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
