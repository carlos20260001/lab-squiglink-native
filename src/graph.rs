use crate::data::FRPoint;
use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const X_TICKS: [f32; 22] = [
    20.0, 30.0, 40.0, 50.0, 60.0, 80.0, 100.0, 200.0, 300.0, 400.0, 500.0, 600.0, 800.0, 1000.0,
    2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 8000.0, 10000.0, 20000.0,
];

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub enum ZoomRegion {
    #[default]
    Full,
    Bass,
    Mids,
    Treble,
}

impl ZoomRegion {
    pub fn x_range(self) -> (f32, f32) {
        match self {
            Self::Full => (20.0, 20000.0),
            Self::Bass => (20.0, 400.0),
            Self::Mids => (100.0, 4000.0),
            Self::Treble => (1000.0, 20000.0),
        }
    }
}

pub struct PlotCurve<'a> {
    pub label: &'a str,
    pub label_key: &'a str,
    pub points: &'a [FRPoint],
    pub color: Color32,
    pub dashed: bool,
}

#[derive(Default)]
pub struct GraphInteraction {
    pub hover_hz: Option<f32>,
    pub label_positions: BTreeMap<String, [f32; 2]>,
    pub y_scale: Option<f32>,
}

pub struct GraphCanvas<'a> {
    pub zoom: ZoomRegion,
    pub smooth_level: usize,
    pub show_labels: bool,
    pub inspect_mode: bool,
    pub label_positions: &'a BTreeMap<String, [f32; 2]>,
    pub tutorial_band: Option<(&'a str, (f32, f32))>,
    pub y_scale: f32,
}

pub fn export_svg(
    curves: &[PlotCurve<'_>],
    zoom: ZoomRegion,
    smooth_level: usize,
    label_positions: &BTreeMap<String, [f32; 2]>,
    dark_mode: bool,
    y_scale: f32,
) -> String {
    const WIDTH: f32 = 1200.0;
    const HEIGHT: f32 = 650.0;
    let plot_rect = Rect::from_min_max(Pos2::new(72.0, 24.0), Pos2::new(1182.0, 610.0));
    let transform = Transform::new(
        plot_rect,
        zoom.x_range(),
        GraphCanvas::scaled_y_range(y_range(curves), y_scale),
    );
    let (background, text_color, grid_color) = if dark_mode {
        ("#1b1b1b", "#e5e5e5", "#4a4a4a")
    } else {
        ("#ffffff", "#202020", "#c8c8c8")
    };
    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH:.0}" height="{HEIGHT:.0}" viewBox="0 0 {WIDTH:.0} {HEIGHT:.0}"><rect width="100%" height="100%" fill="{background}"/>"#
    );
    for &frequency in X_TICKS.iter().filter(|frequency| {
        **frequency >= transform.x_range.0 && **frequency <= transform.x_range.1
    }) {
        let x = transform.hz_to_x(frequency);
        let label = format_axis_frequency(frequency);
        svg.push_str(&format!(
            r#"<path d="M{x:.2} {y1:.2}V{y2:.2}" stroke="{grid_color}" stroke-width="1"/><text x="{x:.2}" y="{label_y:.2}" fill="{text_color}" font-family="sans-serif" font-size="12" text-anchor="middle">{label}</text>"#,
            y1 = plot_rect.top(),
            y2 = plot_rect.bottom(),
            label_y = plot_rect.bottom() + 24.0,
        ));
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
            r#"<path d="M{x1:.2} {y:.2}H{x2:.2}" stroke="{grid_color}" stroke-width="1"/><text x="{label_x:.2}" y="{y:.2}" fill="{text_color}" font-family="sans-serif" font-size="12" text-anchor="end" dominant-baseline="middle">{db}</text>"#,
            x1 = plot_rect.left(),
            x2 = plot_rect.right(),
            label_x = plot_rect.left() - 10.0,
        ));
    }

    svg.push_str(&format!(
        r#"<defs><clipPath id="plot-clip"><rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}"/></clipPath></defs><g clip-path="url(#plot-clip)">"#,
        plot_rect.left(),
        plot_rect.top(),
        plot_rect.width(),
        plot_rect.height(),
    ));
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
                Vec2::new(curve.label.chars().count() as f32 * 7.0, 14.0),
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
            r#"<path d="{path}" fill="none" stroke="{}" stroke-width="2"{dash}/>"#,
            svg_color(curve.color)
        ));
    }
    svg.push_str("</g>");

    if !label_candidates.is_empty() {
        let automatic = GraphCanvas::place_labels(
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
            let position = GraphCanvas::clamp_label_position(position, size, plot_rect);
            let curve = curves.iter().find(|curve| curve.label_key == key).unwrap();
            svg.push_str(&format!(
                r#"<text x="{:.2}" y="{:.2}" fill="{}" font-family="sans-serif" font-size="12">{}</text>"#,
                position.x,
                position.y + 11.0,
                svg_color(curve.color),
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
        let available = ui.available_size().max(Vec2::new(320.0, 240.0));
        let (rect, response) = ui.allocate_exact_size(available, Sense::hover());
        let visuals = ui.visuals();
        let background = visuals.panel_fill;
        let text_color = visuals.text_color();
        let grid_color = visuals.widgets.noninteractive.bg_stroke.color;
        let plot_rect = Rect::from_min_max(
            Pos2::new(rect.left() + 64.0, rect.top() + 14.0),
            Pos2::new(rect.right() - 18.0, rect.bottom() - 38.0),
        );
        let (y_min, y_max) = Self::scaled_y_range(y_range(curves), self.y_scale);
        let transform = Transform::new(plot_rect, self.zoom.x_range(), (y_min, y_max));
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, background);

        let plot_painter = painter.with_clip_rect(plot_rect);
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
                plot_painter.rect_filled(
                    band_rect,
                    0.0,
                    Color32::from_rgba_unmultiplied(250, 190, 80, 35),
                );
                painter.text(
                    band_rect.center_top() + Vec2::new(0.0, 5.0),
                    Align2::CENTER_TOP,
                    label,
                    FontId::proportional(12.0),
                    Color32::from_rgb(250, 190, 80),
                );
            }
        }
        for &frequency in X_TICKS.iter().filter(|frequency| {
            **frequency >= transform.x_range.0 && **frequency <= transform.x_range.1
        }) {
            let x = transform.hz_to_x(frequency);
            plot_painter.line_segment(
                [
                    Pos2::new(x, plot_rect.top()),
                    Pos2::new(x, plot_rect.bottom()),
                ],
                Stroke::new(0.7_f32, grid_color),
            );
            let label = format_axis_frequency(frequency);
            painter.text(
                Pos2::new(x, plot_rect.bottom() + 8.0),
                Align2::CENTER_TOP,
                label,
                FontId::proportional(11.0),
                text_color,
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
                Stroke::new(0.7_f32, grid_color),
            );
            painter.text(
                Pos2::new(plot_rect.left() - 10.0, y),
                Align2::RIGHT_CENTER,
                format!("{db}"),
                FontId::proportional(11.0),
                text_color,
            );
        }

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
                curve.color,
                curve.dashed,
                smooth,
                2.0,
            );
            screen_curves.push(Self::sample_curve_path(&points, smooth));
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
                        FontId::proportional(11.0),
                        curve.color,
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

        let default_positions = Self::place_labels(
            label_candidates
                .iter()
                .map(|(key, point, size, _, _)| ((*key).to_owned(), point.y, size.y))
                .collect(),
            plot_rect,
        );
        let mut updated_label_positions = BTreeMap::new();
        for (key, endpoint, size, color, galley) in label_candidates {
            let normalized = self.label_positions.get(key).copied();
            let mut top_left = normalized.map_or_else(
                || {
                    let y = default_positions.get(key).copied().unwrap_or(endpoint.y);
                    Pos2::new((endpoint.x + 6.0).min(plot_rect.right() - size.x), y)
                },
                |position| {
                    Pos2::new(
                        plot_rect.left() + position[0].clamp(0.0, 1.0) * plot_rect.width(),
                        plot_rect.top() + position[1].clamp(0.0, 1.0) * plot_rect.height(),
                    )
                },
            );
            top_left = Self::clamp_label_position(top_left, size, plot_rect);
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
                top_left = Self::clamp_label_position(
                    drag_origin + label_response.drag_delta(),
                    size,
                    plot_rect,
                );
            }
            if label_response.drag_stopped() {
                ui.ctx()
                    .data_mut(|data| data.remove::<Pos2>(label_response.id));
            }
            painter.galley(top_left, galley, color);
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
            Stroke::new(1.5_f32, text_color),
        );
        painter.circle_filled(
            handle_rect.center(),
            7.0,
            if handle_response.hovered() || handle_response.dragged() {
                visuals.selection.bg_fill
            } else {
                visuals.widgets.noninteractive.bg_fill
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
            if let Some(index) = Self::nearest_curve_index(&screen_curves, position, 14.0) {
                Self::draw_curve_lines(
                    &plot_painter,
                    &screen_curves[index],
                    curves[index].color,
                    curves[index].dashed,
                    4.0,
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
                    Stroke::new(1.0_f32, text_color),
                );
                painter.text(
                    Pos2::new(x, plot_rect.top() + 5.0),
                    Align2::CENTER_TOP,
                    format_frequency(frequency),
                    FontId::proportional(11.0),
                    text_color,
                );
                let readout = curves
                    .iter()
                    .filter_map(|curve| {
                        nearest_point(curve.points, frequency).map(|point| (curve, point))
                    })
                    .map(|(curve, point)| format!("{}  {:.1} dB", curve.label, point.db))
                    .collect::<Vec<_>>()
                    .join("\n");
                painter.text(
                    plot_rect.left_top() + Vec2::new(8.0, 8.0),
                    Align2::LEFT_TOP,
                    readout,
                    FontId::proportional(11.0),
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

    fn place_labels(mut labels: Vec<(String, f32, f32)>, plot_rect: Rect) -> BTreeMap<String, f32> {
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

    fn clamp_label_position(position: Pos2, size: Vec2, bounds: Rect) -> Pos2 {
        Pos2::new(
            position
                .x
                .clamp(bounds.left(), (bounds.right() - size.x).max(bounds.left())),
            position
                .y
                .clamp(bounds.top(), (bounds.bottom() - size.y).max(bounds.top())),
        )
    }

    fn scaled_y_range((minimum, maximum): (f32, f32), scale: f32) -> (f32, f32) {
        let center = (minimum + maximum) / 2.0;
        let half_range = (maximum - minimum) * scale.clamp(0.35, 4.0) / 2.0;
        (center - half_range, center + half_range)
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

    fn sample_curve_path(points: &[Pos2], smooth: bool) -> Vec<Pos2> {
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

    fn point_segment_distance(point: Pos2, start: Pos2, end: Pos2) -> f32 {
        let segment = end - start;
        let length_squared = segment.length_sq();
        if length_squared == 0.0 {
            return point.distance(start);
        }
        let projection = ((point - start).dot(segment) / length_squared).clamp(0.0, 1.0);
        point.distance(start + segment * projection)
    }

    fn nearest_curve_index(curves: &[Vec<Pos2>], position: Pos2, threshold: f32) -> Option<usize> {
        curves
            .iter()
            .enumerate()
            .filter_map(|(index, points)| {
                let distance = points
                    .windows(2)
                    .map(|pair| Self::point_segment_distance(position, pair[0], pair[1]))
                    .fold(f32::INFINITY, f32::min);
                (distance <= threshold).then_some((index, distance))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(index, _)| index)
    }
}

fn curve_control_points(points: &[Pos2], natural: bool) -> Vec<[Pos2; 4]> {
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

fn natural_control_points(values: &[f32]) -> (Vec<f32>, Vec<f32>) {
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

fn cubic_bezier(start: Pos2, control1: Pos2, control2: Pos2, end: Pos2, t: f32) -> Pos2 {
    let inverse = 1.0 - t;
    start * inverse.powi(3)
        + control1.to_vec2() * (3.0 * inverse.powi(2) * t)
        + control2.to_vec2() * (3.0 * inverse * t.powi(2))
        + end.to_vec2() * t.powi(3)
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

#[derive(Clone, Copy)]
struct Transform {
    rect: Rect,
    x_range: (f32, f32),
    y_range: (f32, f32),
}

impl Transform {
    fn new(rect: Rect, x_range: (f32, f32), y_range: (f32, f32)) -> Self {
        Self {
            rect,
            x_range,
            y_range,
        }
    }

    fn hz_to_x(self, hz: f32) -> f32 {
        let min = self.x_range.0.ln();
        let t = (hz.ln() - min) / (self.x_range.1.ln() - min);
        self.rect.left() + t * self.rect.width()
    }

    fn x_at(self, x: f32) -> f32 {
        let t = (x - self.rect.left()) / self.rect.width();
        (self.x_range.0.ln() + t * (self.x_range.1.ln() - self.x_range.0.ln())).exp()
    }

    fn db_to_y(self, db: f32) -> f32 {
        self.rect.bottom()
            - (db - self.y_range.0) / (self.y_range.1 - self.y_range.0) * self.rect.height()
    }
}

fn y_range(curves: &[PlotCurve<'_>]) -> (f32, f32) {
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

fn format_frequency(hz: f32) -> String {
    if hz >= 1000.0 {
        format!("{:.2} kHz", hz / 1000.0)
    } else {
        format!("{hz:.0} Hz")
    }
}

fn format_axis_frequency(hz: f32) -> String {
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

fn nearest_point(points: &[FRPoint], frequency: f32) -> Option<&FRPoint> {
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

#[cfg(test)]
mod tests {
    use super::{
        curve_control_points, export_svg, format_axis_frequency, GraphCanvas, PlotCurve, Transform,
        ZoomRegion,
    };
    use crate::data::FRPoint;
    use egui::{pos2, Color32, Rect};
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
        let positions = GraphCanvas::place_labels(
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
    fn dragged_label_anchor_is_clamped_to_plot_bounds() {
        let plot = Rect::from_min_max(pos2(10.0, 20.0), pos2(110.0, 120.0));
        assert_eq!(
            GraphCanvas::clamp_label_position(pos2(-10.0, 500.0), egui::vec2(30.0, 15.0), plot),
            pos2(10.0, 105.0)
        );
    }

    #[test]
    fn vertical_scale_expands_and_contracts_around_the_same_center() {
        assert_eq!(GraphCanvas::scaled_y_range((20.0, 80.0), 1.0), (20.0, 80.0));
        assert_eq!(
            GraphCanvas::scaled_y_range((20.0, 80.0), 2.0),
            (-10.0, 110.0)
        );
        assert_eq!(GraphCanvas::scaled_y_range((20.0, 80.0), 0.5), (35.0, 65.0));
        assert_eq!(GraphCanvas::scaled_y_range((20.0, 80.0), 0.0), (39.5, 60.5));
    }

    #[test]
    fn hover_highlight_selects_nearest_curve_within_threshold() {
        let curves = vec![
            vec![pos2(0.0, 0.0), pos2(100.0, 0.0)],
            vec![pos2(0.0, 30.0), pos2(100.0, 30.0)],
        ];
        assert_eq!(
            GraphCanvas::nearest_curve_index(&curves, pos2(50.0, 27.0), 14.0),
            Some(1)
        );
        assert_eq!(
            GraphCanvas::nearest_curve_index(&curves, pos2(50.0, 15.0), 5.0),
            None
        );
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
                color: Color32::from_rgb(12, 34, 56),
                dashed: false,
            },
            PlotCurve {
                label: "Target",
                label_key: "target",
                points: &points,
                color: Color32::from_rgb(90, 100, 110),
                dashed: true,
            },
        ];
        let svg = export_svg(&curves, ZoomRegion::Full, 5, &BTreeMap::new(), true, 1.0);
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
            color: Color32::WHITE,
            dashed: false,
        }];
        let full = export_svg(&curves, ZoomRegion::Full, 5, &BTreeMap::new(), false, 1.0);
        let zoomed = export_svg(&curves, ZoomRegion::Bass, 5, &BTreeMap::new(), false, 2.0);

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
            color: Color32::WHITE,
            dashed: false,
        }];

        let svg = export_svg(&curves, ZoomRegion::Full, 1, &BTreeMap::new(), false, 1.0);
        assert!(svg.contains(" C "));
    }

    #[test]
    fn axis_endpoint_labels_include_units() {
        assert_eq!(format_axis_frequency(20.0), "20Hz");
        assert_eq!(format_axis_frequency(20000.0), "20kHz");
        assert_eq!(format_axis_frequency(1000.0), "1k");
    }
}
