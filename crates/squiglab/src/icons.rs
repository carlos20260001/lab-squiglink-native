//! The marks the program's buttons carry.
//!
//! Every one of them is the reference's own vector: the paths in its
//! stylesheet's masks, and the paths in its own markup, drawn in the box the
//! reference draws them in and at the size it draws them at. The reference's
//! own shape, rather than something like it.

use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};

/// A mark one of the row's buttons carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    /// An eye: the curve is shown.
    Eye,
    /// A crossed eye: the curve is hidden.
    EyeCrossed,
    /// A pin: the curve stays when the others are taken off.
    Pin,
    /// An arrow into a tray: the curve is saved.
    Download,
    /// A cross: the curve is taken off the graph.
    Remove,
    /// A triangle: the source is played.
    Play,
    /// A square: the source is stopped.
    Stop,
    /// A cross of two bars: the measurement is put on the graph.
    Plus,
    /// A cog: the settings.
    Settings,
    /// An arrow bending back: the equalizer is cleared.
    Reset,
    /// A speaker: the volume.
    Volume,
    /// A wave: the curve the others are read against.
    Squiggle,
    /// A line: the curve that is the baseline.
    BaselineLine,
}

/// The size a mark is drawn at, in points.
const SIZE: f32 = 16.0;

/// Draw a mark inside a rectangle.
///
/// The mark is drawn in the box the reference draws it in, and at the size
/// the reference draws it at, so that the shapes are its own rather than
/// something like them.
pub fn draw(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let box_of = box_of(icon);
    let scale = SIZE / box_of;
    let origin = rect.min;
    if filled(icon) {
        if icon == Icon::Remove {
            // The reference's cross is a shape that cannot be filled here, so
            // it is drawn as the two bars it is made of, with the same
            // geometry: the ends of the arms are five units in a box of a
            // hundred and twenty, and the bars are ten wide.
            let bar = 10.0 * scale;
            let arm = 28.18 * scale;
            let far = 91.82 * scale;
            let stroke = Stroke::new(bar, color);
            painter.add(egui::Shape::line(
                vec![
                    Pos2::new(origin.x + arm, origin.y + arm),
                    Pos2::new(origin.x + far, origin.y + far),
                ],
                stroke,
            ));
            painter.add(egui::Shape::line(
                vec![
                    Pos2::new(origin.x + arm, origin.y + far),
                    Pos2::new(origin.x + far, origin.y + arm),
                ],
                stroke,
            ));
            return;
        }
        if icon == Icon::Plus {
            // The reference's cross of two bars, with the same geometry: the
            // bars are ten wide in a box of a hundred and twenty, and they
            // reach from fifteen to a hundred and five.
            let bar = 10.0 * scale;
            let near = 15.0 * scale;
            let far = 105.0 * scale;
            let middle = 60.0 * scale;
            let stroke = Stroke::new(bar, color);
            painter.add(egui::Shape::line(
                vec![
                    Pos2::new(origin.x + near, origin.y + middle),
                    Pos2::new(origin.x + far, origin.y + middle),
                ],
                stroke,
            ));
            painter.add(egui::Shape::line(
                vec![
                    Pos2::new(origin.x + middle, origin.y + near),
                    Pos2::new(origin.x + middle, origin.y + far),
                ],
                stroke,
            ));
            return;
        }
        for path in paths_for(icon) {
            let points = flatten(path, scale, origin);
            if points.len() > 2 {
                painter.add(egui::Shape::Path(egui::epaint::PathShape {
                    points,
                    fill: color,
                    stroke: egui::epaint::PathStroke::NONE,
                    closed: true,
                }));
            }
        }
        return;
    }
    let stroke = Stroke::new(2.0 * scale, color);
    for path in paths_for(icon) {
        let points = flatten(path, scale, origin);
        if points.len() > 1 {
            painter.add(egui::Shape::line(points, stroke));
        }
    }
}

/// The reference's paths, in its own box.
fn paths_for(icon: Icon) -> Vec<&'static str> {
    match icon {
        // The download, from the stylesheet's mask.
        Icon::Download => vec![
            "M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2 -2v-2",
            "M7 11l5 5l5 -5",
            "M12 4l0 12",
        ],
        // The eye, from the stylesheet's mask.
        Icon::Eye => vec![
            "M10 12a2 2 0 1 0 4 0a2 2 0 0 0 -4 0",
            "M21 12c-2.4 4 -5.4 6 -9 6c-3.6 0 -6.6 -2 -9 -6c2.4 -4 5.4 -6 9 -6c3.6 0 6.6 2 9 6",
        ],
        // The crossed eye.
        Icon::EyeCrossed => vec![
            "M10.585 10.587a2 2 0 0 0 2.829 2.828",
            "M16.681 16.673a8.717 8.717 0 0 1 -4.681 1.327c-3.6 0 -6.6 -2 -9 -6c1.272 -2.12 2.712 -3.678 4.32 -4.674m2.86 -1.146a9.055 9.055 0 0 1 1.82 -.18c3.6 0 6.6 2 9 6c-.666 1.11 -1.379 2.067 -2.138 2.87",
            "M3 3l18 18",
        ],
        // The pin.
        Icon::Pin => vec![
            "M15 4.5l-4 4l-4 1.5l-1.5 1.5l7 7l1.5 -1.5l1.5 -4l4 -4",
            "M9 15l-4.5 4.5",
            "M14.5 4l5.5 5.5",
        ],
        // The cog, from the stylesheet's mask.
        Icon::Settings => vec![
            "M10.325 4.317c.426 -1.756 2.924 -1.756 3.35 0a1.724 1.724 0 0 0 2.573 1.066c1.543 -.94 3.31 .826 2.37 2.37a1.724 1.724 0 0 0 1.065 2.572c1.756 .426 1.756 2.924 0 3.35a1.724 1.724 0 0 0 -1.066 2.573c.94 1.543 -.826 3.31 -2.37 2.37a1.724 1.724 0 0 0 -2.572 1.065c-.426 1.756 -2.924 1.756 -3.35 0a1.724 1.724 0 0 0 -2.573 -1.066c-1.543 .94 -3.31 -.826 -2.37 -2.37a1.724 1.724 0 0 0 -1.065 -2.572c-1.756 -.426 -1.756 -2.924 0 -3.35a1.724 1.724 0 0 0 1.066 -2.573c-.94 -1.543 .826 -3.31 2.37 -2.37c1 .608 2.296 .07 2.572 -1.065",
            "M9 12a3 3 0 1 0 6 0a3 3 0 0 0 -6 0",
        ],
        // The arrow that bends back, from the stylesheet's mask.
        Icon::Reset => vec!["M9 14l-4 -4l4 -4", "M5 10h11a4 4 0 1 1 0 8h-1"],
        // The wave the reference draws for the baseline, from its stylesheet.
        Icon::Squiggle => vec![
            "M21 12h-2c-.894 0 -1.662 -.857 -1.761 -2c-.296 -3.45 -.749 -6 -2.749 -6s-2.5 3.582 -2.5 8s-.5 8 -2.5 8s-2.452 -2.547 -2.749 -6c-.1 -1.147 -.867 -2 -1.763 -2h-2",
        ],
        // The line the reference draws for the chosen baseline.
        Icon::BaselineLine => vec!["M4 12h16"],
        // The speaker, from the reference's own markup.
        Icon::Volume => vec![
            "M15 8a5 5 0 0 1 0 8",
            "M17.7 5a9 9 0 0 1 0 14",
            "M6 15h-2a1 1 0 0 1 -1 -1v-4a1 1 0 0 1 1 -1h2l3.5 -4.5a.8 .8 0 0 1 1.5 .5v14a.8 .8 0 0 1 -1.5 .5l-3.5 -4.5",
        ],
        // The player's triangle, filled.
        Icon::Play => vec![
            "M6 4v16a1 1 0 0 0 1.524 .852l13 -8a1 1 0 0 0 0 -1.704l-13 -8a1 1 0 0 0 -1.524 .852z",
        ],
        // The player's square, filled.
        Icon::Stop => vec!["M17 4h-10a3 3 0 0 0 -3 3v10a3 3 0 0 0 3 3h10a3 3 0 0 0 3 -3v-10a3 3 0 0 0 -3 -3z"],
        // The cross that takes a curve off the graph, filled, in the
        // reference's own hundred-and-twenty point box.
        Icon::Remove => vec![
            "M95.36,24.64h0a5,5,0,0,0-7.08,0L60,52.93,31.72,24.64a5,5,0,0,0-7.08,0h0a5,5,0,0,0,0,7.08L52.93,60,24.64,88.28a5,5,0,0,0,0,7.08h0a5,5,0,0,0,7.08,0L60,67.07,88.28,95.36a5,5,0,0,0,7.08,0h0a5,5,0,0,0,0-7.08L67.07,60,95.36,31.72A5,5,0,0,0,95.36,24.64Z",
        ],
        // The cross that puts a measurement on the graph, in the same box.
        Icon::Plus => vec![
            "M110,60h0a5,5,0,0,0-5-5H65V15a5,5,0,0,0-5-5h0a5,5,0,0,0-5,5V55H15a5,5,0,0,0-5,5h0a5,5,0,0,0,5,5H55v40a5,5,0,0,0,5,5h0a5,5,0,0,0,5-5V65h40A5,5,0,0,0,110,60Z",
        ],
    }
}

/// The box the reference's mark is drawn in.
fn box_of(icon: Icon) -> f32 {
    match icon {
        // The two crosses are drawn in a hundred-and-twenty point box.
        Icon::Remove | Icon::Plus => 120.0,
        _ => 24.0,
    }
}

/// Whether the mark is filled in, rather than drawn in outline.
fn filled(icon: Icon) -> bool {
    matches!(icon, Icon::Play | Icon::Stop | Icon::Remove | Icon::Plus)
}

/// A mark in a cell, as the reference's row of buttons draws them.
///
/// The cell is thirty-six points across, and the mark sixteen. Some of the
/// reference's marks sit in a box with a hairline border, and some sit on
/// the page; which is which is the caller's business.
pub fn cell(
    ui: &mut egui::Ui,
    icon: Icon,
    tooltip: &str,
    boxed: bool,
    color: egui::Color32,
) -> egui::Response {
    let palette = crate::theme::current_palette();
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(36.0), egui::Sense::click());
    if boxed {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(6), palette.page);
        ui.painter().rect_stroke(
            rect,
            egui::CornerRadius::same(6),
            egui::Stroke::new(1.0, palette.divider),
            egui::StrokeKind::Inside,
        );
    }
    let mark = egui::Rect::from_center_size(rect.center(), Vec2::splat(16.0));
    draw(ui.painter(), mark, icon, color);
    response.on_hover_text(tooltip)
}

/// The points of an SVG path, in points, on a 24 by 24 box scaled to the
/// size asked for.
///
/// The reference's paths are SVG, and egui draws lines rather than paths, so
/// the curves are flattened into the points of a polyline. The commands the
/// reference uses are moves, lines, horizontal and vertical lines, cubic
/// curves, and elliptical arcs.
fn flatten(path: &str, scale: f32, origin: Pos2) -> Vec<Pos2> {
    let mut points: Vec<Pos2> = Vec::new();
    let mut cursor = Pos2::ZERO;
    let mut start = Pos2::ZERO;
    let mut command = ' ';
    let mut numbers: Vec<f32> = Vec::new();

    for token in tokens(path) {
        match token {
            Token::Command(next) => {
                apply(command, &numbers, &mut cursor, &mut start, &mut points);
                numbers.clear();
                command = next;
            }
            Token::Number(value) => numbers.push(value),
        }
    }
    apply(command, &numbers, &mut cursor, &mut start, &mut points);

    points
        .into_iter()
        .map(|point| Pos2::new(origin.x + point.x * scale, origin.y + point.y * scale))
        .collect()
}

/// One piece of a path.
enum Token {
    Command(char),
    Number(f32),
}

/// The commands and the numbers of a path.
fn tokens(path: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut characters = path.chars().peekable();
    while let Some(character) = characters.next() {
        if character.is_ascii_alphabetic() {
            out.push(Token::Command(character));
            continue;
        }
        if character.is_ascii_digit() || character == '.' {
            let mut text = String::new();
            text.push(character);
            while let Some(&next) = characters.peek() {
                if next.is_ascii_digit() || next == '.' {
                    text.push(next);
                    characters.next();
                } else {
                    break;
                }
            }
            out.push(Token::Number(text.parse().unwrap_or(0.0)));
            continue;
        }
        if character == '-' || character == '+' {
            // A sign starts the next number: "5-5" is two numbers.
            let mut text = String::new();
            text.push(character);
            while let Some(&next) = characters.peek() {
                if next.is_ascii_digit() || next == '.' {
                    text.push(next);
                    characters.next();
                } else {
                    break;
                }
            }
            out.push(Token::Number(text.parse().unwrap_or(0.0)));
        }
    }
    out
}

/// The points of one command.
fn apply(
    command: char,
    numbers: &[f32],
    cursor: &mut Pos2,
    start: &mut Pos2,
    points: &mut Vec<Pos2>,
) {
    match command {
        'M' | 'm' => {
            for pair in numbers.chunks(2) {
                if pair.len() < 2 {
                    break;
                }
                let point = if command == 'm' {
                    Pos2::new(cursor.x + pair[0], cursor.y + pair[1])
                } else {
                    Pos2::new(pair[0], pair[1])
                };
                *cursor = point;
                *start = point;
                points.push(point);
            }
        }
        'L' | 'l' => {
            for pair in numbers.chunks(2) {
                if pair.len() < 2 {
                    break;
                }
                let point = if command == 'l' {
                    Pos2::new(cursor.x + pair[0], cursor.y + pair[1])
                } else {
                    Pos2::new(pair[0], pair[1])
                };
                *cursor = point;
                points.push(point);
            }
        }
        'H' | 'h' => {
            for x in numbers {
                let point = if command == 'h' {
                    Pos2::new(cursor.x + x, cursor.y)
                } else {
                    Pos2::new(*x, cursor.y)
                };
                *cursor = point;
                points.push(point);
            }
        }
        'V' | 'v' => {
            for y in numbers {
                let point = if command == 'v' {
                    Pos2::new(cursor.x, cursor.y + y)
                } else {
                    Pos2::new(cursor.x, *y)
                };
                *cursor = point;
                points.push(point);
            }
        }
        'C' | 'c' => {
            for group in numbers.chunks(6) {
                if group.len() < 6 {
                    break;
                }
                let offset = if command == 'c' { *cursor } else { Pos2::ZERO };
                let first = Pos2::new(offset.x + group[0], offset.y + group[1]);
                let second = Pos2::new(offset.x + group[2], offset.y + group[3]);
                let end = Pos2::new(offset.x + group[4], offset.y + group[5]);
                for step in 1..=16 {
                    let t = step as f32 / 16.0;
                    points.push(cubic(*cursor, first, second, end, t));
                }
                *cursor = end;
            }
        }
        'A' | 'a' => {
            for group in numbers.chunks(7) {
                if group.len() < 7 {
                    break;
                }
                let offset = if command == 'a' { *cursor } else { Pos2::ZERO };
                let end = Pos2::new(offset.x + group[5], offset.y + group[6]);
                arc(
                    *cursor,
                    Pos2::new(group[0], group[1]),
                    group[2],
                    group[3],
                    group[4],
                    end,
                    points,
                );
                *cursor = end;
            }
        }
        'Z' | 'z' => {
            points.push(*start);
            *cursor = *start;
        }
        _ => {}
    }
}

/// The points of an elliptical arc, from the way the SVG describes one: the
/// radii, the rotation, the two flags, and where it ends.
fn arc(
    from: Pos2,
    radii: Pos2,
    rotation: f32,
    large_arc: f32,
    sweep: f32,
    to: Pos2,
    points: &mut Vec<Pos2>,
) {
    let (rx, ry) = (radii.x.abs().max(0.001), radii.y.abs().max(0.001));
    if (to - from).length() < 1e-6 {
        points.push(to);
        return;
    }
    let phi = rotation.to_radians();
    let (sin_phi, cos_phi) = phi.sin_cos();
    let dx = (from.x - to.x) / 2.0;
    let dy = (from.y - to.y) / 2.0;
    let x1 = cos_phi * dx + sin_phi * dy;
    let y1 = -sin_phi * dx + cos_phi * dy;
    // The radii are scaled up when they are too small to reach.
    let lambda = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
    let (rx, ry) = if lambda > 1.0 {
        (rx * lambda.sqrt(), ry * lambda.sqrt())
    } else {
        (rx, ry)
    };
    let sign = if large_arc != sweep { 1.0 } else { -1.0 };
    let numerator = (rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1)
        .max(0.0)
        .sqrt();
    let coefficient = sign * numerator / (rx * y1 * y1 + ry * x1 * x1);
    let cx1 = coefficient * rx * y1 / ry;
    let cy1 = -coefficient * ry * x1 / rx;
    let centre = Pos2::new(
        cos_phi * cx1 - sin_phi * cy1 + (from.x + to.x) / 2.0,
        sin_phi * cx1 + cos_phi * cy1 + (from.y + to.y) / 2.0,
    );
    let angle = |ux: f32, uy: f32, vx: f32, vy: f32| {
        let dot = ux * vx + uy * vy;
        let length = (ux * ux + uy * uy).sqrt() * (vx * vx + vy * vy).sqrt();
        let mut value = (dot / length).clamp(-1.0, 1.0).acos();
        if ux * vy - uy * vx < 0.0 {
            value = -value;
        }
        value
    };
    let theta = angle(1.0, 0.0, (x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut delta = angle(
        (x1 - cx1) / rx,
        (y1 - cy1) / ry,
        (-x1 - cx1) / rx,
        (-y1 - cy1) / ry,
    );
    if sweep == 0.0 && delta > 0.0 {
        delta -= std::f32::consts::TAU;
    }
    if sweep == 1.0 && delta < 0.0 {
        delta += std::f32::consts::TAU;
    }
    for step in 1..=16 {
        let t = theta + delta * (step as f32 / 16.0);
        points.push(Pos2::new(
            centre.x + rx * t.cos() * cos_phi - ry * t.sin() * sin_phi,
            centre.y + rx * t.cos() * sin_phi + ry * t.sin() * cos_phi,
        ));
    }
}

/// A point on a cubic curve.
fn cubic(from: Pos2, first: Pos2, second: Pos2, to: Pos2, t: f32) -> Pos2 {
    let u = 1.0 - t;
    let a = u * u * u;
    let b = 3.0 * u * u * t;
    let c = 3.0 * u * t * t;
    let d = t * t * t;
    Pos2::new(
        a * from.x + b * first.x + c * second.x + d * to.x,
        a * from.y + b * first.y + c * second.y + d * to.y,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A half circle is a half circle, whichever way the flags are set.
    #[test]
    fn an_arc_is_flattened_onto_its_circle() {
        for flags in ["0 0 0", "0 0 1", "0 1 0", "0 1 1"] {
            let path = format!("M0 0a5 5 {flags} 10 0");
            let points = flatten(&path, 1.0, Pos2::ZERO);
            assert!(points.len() > 8, "{flags}: only {} points", points.len());
            for point in &points {
                let distance = ((point.x - 5.0).powi(2) + point.y.powi(2)).sqrt();
                assert!(
                    (distance - 5.0).abs() < 0.05,
                    "{flags}: a point was {distance} from the middle"
                );
            }
        }
    }

    /// The cross that takes a curve off the graph is the reference's own
    /// shape: a wide X in the middle of its box.
    #[test]
    fn the_cross_is_the_reference_s_own_shape() {
        let path = "M95.36,24.64h0a5,5,0,0,0-7.08,0L60,52.93,31.72,24.64a5,5,0,0,0-7.08,0h0a5,5,0,0,0,0,7.08L52.93,60,24.64,88.28a5,5,0,0,0,0,7.08h0a5,5,0,0,0,7.08,0L60,67.07,88.28,95.36a5,5,0,0,0,7.08,0h0a5,5,0,0,0,0-7.08L67.07,60,95.36,31.72A5,5,0,0,0,95.36,24.64Z";
        let points = flatten(path, 1.0, Pos2::ZERO);
        let left = points.iter().map(|p| p.x).fold(f32::MAX, f32::min);
        let right = points.iter().map(|p| p.x).fold(f32::MIN, f32::max);
        let top = points.iter().map(|p| p.y).fold(f32::MAX, f32::min);
        let bottom = points.iter().map(|p| p.y).fold(f32::MIN, f32::max);
        assert!(
            right - left > 60.0,
            "the cross is only {} wide",
            right - left
        );
        assert!(
            bottom - top > 60.0,
            "the cross is only {} tall",
            bottom - top
        );
        // It is in the middle of its own box.
        assert!(((left + right) / 2.0 - 60.0).abs() < 1.0);
        assert!(((top + bottom) / 2.0 - 60.0).abs() < 1.0);
    }
}
