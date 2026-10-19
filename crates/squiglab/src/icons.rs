//! The small icons the interface draws itself.
//!
//! The reference draws its buttons as SVG. The typeface the product uses has
//! no glyph for what these buttons mean — an arrow, a pin, an eye — so they
//! are drawn here rather than spelled.

use egui::{Color32, Painter, Pos2, Rect, Stroke};

/// The icons the row buttons use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    /// An eye: the curve is shown.
    Eye,
    /// A crossed eye: the curve is hidden.
    EyeCrossed,
    /// A pin: the curve stays when the others are taken off.
    Pin,
    /// An unfilled pin.
    PinOutline,
    /// An arrow into a tray: the curve is saved.
    Save,
    /// A cross: the curve is taken off the graph.
    Close,
    /// A flat line: the curve the others are read against.
    Baseline,
}

/// Draw an icon inside a rectangle.
pub fn draw(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let stroke = Stroke::new(1.4, color);
    let centre = rect.center();
    let size = rect.height().min(rect.width()) * 0.5;
    match icon {
        Icon::Eye | Icon::EyeCrossed => {
            let width = size * 1.3;
            let height = size * 0.75;
            // The eye: two arcs, drawn as a flattened ring.
            painter.add(egui::Shape::line(
                eye_outline(centre, width, height),
                stroke,
            ));
            painter.circle_filled(centre, size * 0.28, color);
            if icon == Icon::EyeCrossed {
                painter.line_segment(
                    [
                        Pos2::new(centre.x - width * 0.6, centre.y - height * 0.6),
                        Pos2::new(centre.x + width * 0.6, centre.y + height * 0.6),
                    ],
                    stroke,
                );
            }
        }
        Icon::Pin | Icon::PinOutline => {
            // A pin: a head and a spike, drawn as a circle and a line.
            painter.circle_stroke(
                Pos2::new(centre.x, centre.y - size * 0.35),
                size * 0.42,
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(centre.x, centre.y - size * 0.35 + size * 0.42),
                    Pos2::new(centre.x, centre.y + size * 0.6),
                ],
                stroke,
            );
            if icon == Icon::Pin {
                painter.circle_filled(
                    Pos2::new(centre.x, centre.y - size * 0.35),
                    size * 0.18,
                    color,
                );
            }
        }
        Icon::Save => {
            // An arrow into a tray.
            let bottom = centre.y + size * 0.6;
            painter.line_segment(
                [
                    Pos2::new(centre.x, centre.y - size * 0.7),
                    Pos2::new(centre.x, bottom),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(centre.x - size * 0.4, centre.y - size * 0.1),
                    Pos2::new(centre.x, centre.y + size * 0.35),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(centre.x + size * 0.4, centre.y - size * 0.1),
                    Pos2::new(centre.x, centre.y + size * 0.35),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(centre.x - size * 0.7, bottom),
                    Pos2::new(centre.x + size * 0.7, bottom),
                ],
                stroke,
            );
        }
        Icon::Baseline => {
            // A flat line with a tick at the end, which is what the
            // reference's baseline mark is.
            painter.line_segment(
                [
                    Pos2::new(centre.x - size * 0.8, centre.y + size * 0.4),
                    Pos2::new(centre.x + size * 0.8, centre.y + size * 0.4),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(centre.x - size * 0.8, centre.y + size * 0.4),
                    Pos2::new(centre.x - size * 0.8, centre.y - size * 0.2),
                ],
                stroke,
            );
        }
        Icon::Close => {
            painter.line_segment(
                [
                    Pos2::new(centre.x - size * 0.55, centre.y - size * 0.55),
                    Pos2::new(centre.x + size * 0.55, centre.y + size * 0.55),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(centre.x + size * 0.55, centre.y - size * 0.55),
                    Pos2::new(centre.x - size * 0.55, centre.y + size * 0.55),
                ],
                stroke,
            );
        }
    }
}

/// A button that draws an icon rather than spelling it.
pub fn button(ui: &mut egui::Ui, icon: Icon, tooltip: &str, selected: bool) -> egui::Response {
    let palette = crate::theme::current_palette();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(4), palette.hover_fill);
    }
    let color = if selected {
        palette.accent
    } else {
        palette.text_secondary
    };
    draw(ui.painter(), rect, icon, color);
    response.on_hover_text(tooltip)
}

/// The outline of an eye: a lens shape, made of two arcs.
fn eye_outline(centre: Pos2, width: f32, height: f32) -> Vec<Pos2> {
    let mut points = Vec::new();
    for step in 0..=16 {
        let angle = std::f32::consts::PI * (step as f32 / 16.0);
        points.push(Pos2::new(
            centre.x - width * 0.5 + width * angle.cos(),
            centre.y - height * angle.sin(),
        ));
    }
    // The lower arc, back from the right to the left.
    for step in 0..=16 {
        let angle = std::f32::consts::PI * (step as f32 / 16.0);
        points.push(Pos2::new(
            centre.x + width * 0.5 - width * angle.cos(),
            centre.y + height * angle.sin(),
        ));
    }
    points
}
