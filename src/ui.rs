//! Reusable interface components built on the tokens in [`crate::theme`].
//!
//! Every widget here resolves colors and sizes from the token set rather than
//! hard-coding them, so the two themes and any future theme change apply here
//! automatically.

use egui::{pos2, Align, Color32, FontId, Pos2, Rect, RichText, Shape, Stroke, Ui, Vec2};

use crate::theme::{self, Palette, TypeStep};
use squiglab_core::graph::ZoomRegion;

pub use crate::theme::Spacing;

/// A row of mutually exclusive options that reads as one control.
///
/// Used for zoom region, normalization mode, and channel selection. Renders the
/// options as a single contiguous group with the current one filled with the
/// accent, so the current selection is obvious at a glance in a dense toolbar.
pub fn segmented<'a, T: PartialEq + Clone>(
    ui: &mut Ui,
    options: &'a [(T, &'a str)],
    selected: &'a mut T,
    tooltip: Option<&'a str>,
) -> bool {
    let mut changed = false;
    let palette = Palette::resolve(ui.visuals().dark_mode);

    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(2))
        .fill(ui.visuals().extreme_bg_color)
        .corner_radius(egui::CornerRadius::same(theme::RADIUS_CONTROL as u8))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (value, label) in options {
                let is_selected = *selected == *value;
                let response = ui
                    .selectable_label(is_selected, *label)
                    .on_hover_text(tooltip.unwrap_or_default());
                if response.clicked() && !is_selected {
                    *selected = value.clone();
                    changed = true;
                }
            }
        });

    let _ = palette;
    changed
}

/// Horizontal rule used to separate logical clusters in the toolbar.
///
/// A fixed one-pixel divider rather than a full panel edge, so the eye reads
/// grouping without the toolbar turning into a row of boxes.
pub fn cluster_rule(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 20.0), egui::Sense::hover());
    let palette = Palette::resolve(ui.visuals().dark_mode);
    ui.painter().vline(
        rect.center().x,
        rect.min.y..=rect.max.y,
        Stroke::new(1.0, palette.border),
    );
}

/// Uppercase micro label that titles a toolbar cluster.
pub fn cluster_label(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .font(FontId::new(
                TypeStep::Micro.size(),
                egui::FontFamily::Proportional,
            ))
            .color(Palette::resolve(ui.visuals().dark_mode).text_tertiary),
    );
}

/// The wordmark, drawn in a box the height of the control row beside it, with
/// the text centred in it. The category control's frame leaves a few points
/// above its own text, so a wordmark laid out at the top of the row rides
/// above the small caps it should share a cap line with.
///
/// Returns the rect the text sits in.
pub fn wordmark(ui: &mut Ui) -> egui::Rect {
    let palette = Palette::resolve(ui.visuals().dark_mode);
    let font = theme::font_bold(TypeStep::Subtitle);
    let galley = ui
        .painter()
        .layout_no_wrap("SquigLab".to_owned(), font, palette.text_primary);
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(galley.size().x, CONTROL_ROW_HEIGHT),
        egui::Sense::hover(),
    );
    let position = egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
    let size = galley.size();
    ui.painter().galley(position, galley, palette.text_primary);
    egui::Rect::from_min_size(position, size)
}

/// The height of a row of header controls. The wordmark is centred against
/// this band so it shares a cap line with the controls beside it.
pub const CONTROL_ROW_HEIGHT: f32 = 26.0;

/// A quiet text button used for toolbar actions that are not primary.
///
/// Secondary actions stay visually quieter than the accent-filled primary
/// controls so the toolbar has a focal point.
pub fn action_button(ui: &mut Ui, label: &str, tooltip: &str) -> bool {
    ui.add(
        egui::Button::new(RichText::new(label).font(theme::font(TypeStep::Body)))
            .fill(egui::Color32::TRANSPARENT)
            .stroke(Stroke::NONE)
            .min_size(egui::vec2(0.0, 22.0)),
    )
    .on_hover_text(tooltip)
    .clicked()
}

/// A rounded selectable chip.
///
/// The reference tool marks what is in play with pills rather than with
/// checkboxes, because a row of them reads as a set of switches rather than as a
/// list of options, and at the density a target strip runs at the difference
/// matters. Selected means filled with the accent; unselected means an outline,
/// so the set is legible without a label saying which is which.
pub fn pill(
    ui: &mut Ui,
    label: &str,
    selected: bool,
    tooltip: impl Into<String>,
) -> egui::Response {
    let palette = Palette::resolve(ui.visuals().dark_mode);
    let (text_color, fill, stroke) = if selected {
        (palette.on_accent, palette.accent, Stroke::NONE)
    } else {
        (
            palette.text_secondary,
            Color32::TRANSPARENT,
            palette.control_stroke(),
        )
    };
    ui.add(
        egui::Button::new(
            RichText::new(label)
                .font(theme::font(TypeStep::Caption))
                .color(text_color),
        )
        .fill(fill)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(11))
        .min_size(egui::vec2(0.0, 22.0)),
    )
    .on_hover_text(tooltip.into())
}

/// Panel section heading: a bold title with an optional right-aligned meta count.
pub fn section_header(ui: &mut Ui, title: &str, meta: Option<&str>) {
    let palette = Palette::resolve(ui.visuals().dark_mode);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(title)
                .font(theme::font_bold(TypeStep::Subtitle))
                .color(palette.text_primary),
        );
        if let Some(meta) = meta {
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(meta)
                        .font(FontId::new(
                            TypeStep::Micro.size(),
                            egui::FontFamily::Proportional,
                        ))
                        .color(palette.text_tertiary),
                );
            });
        }
    });
}

/// Zoom regions as a single segmented control.
pub fn zoom_segmented(ui: &mut Ui, zoom: &mut ZoomRegion) -> bool {
    segmented(
        ui,
        &[
            (ZoomRegion::Full, "Full"),
            (ZoomRegion::Bass, "Bass"),
            (ZoomRegion::Mids, "Mids"),
            (ZoomRegion::Treble, "Treble"),
        ],
        zoom,
        Some("Limit the frequency range shown in the graph"),
    )
}

/// Height of a tab's label, including the rule that marks the active one.
const TAB_HEIGHT: f32 = 26.0;
/// Thickness of the rule under a tab.
const TAB_RULE_HEIGHT: f32 = 2.0;

/// A row of tab buttons, as the reference tool's left column has, with the
/// active tab's contents rendered by the caller below it.
///
/// The active tab is marked with an accent rule rather than a filled
/// background: a fill would compete with the accent-filled controls inside
/// the panel the tab opens.
pub fn tab_strip<T: Copy + PartialEq>(ui: &mut Ui, selected: &mut T, options: &[(&str, T)]) {
    let palette = Palette::resolve(ui.visuals().dark_mode);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = Spacing::COMPACT;
        for (label, value) in options {
            let is_selected = *selected == *value;
            let color = if is_selected {
                palette.text_primary
            } else {
                palette.text_secondary
            };
            let response = ui.add(
                egui::Button::new(
                    RichText::new(*label)
                        .font(theme::font_bold(TypeStep::Subtitle))
                        .color(color),
                )
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::NONE)
                .min_size(egui::vec2(0.0, TAB_HEIGHT)),
            );
            if response.clicked() {
                *selected = *value;
            }
            let rule = egui::Rect::from_min_size(
                egui::pos2(response.rect.left(), response.rect.max.y),
                egui::vec2(response.rect.width(), TAB_RULE_HEIGHT),
            );
            ui.painter().rect_filled(
                rule,
                0.0,
                if is_selected {
                    palette.accent
                } else {
                    Color32::TRANSPARENT
                },
            );
        }
    });
    ui.add_space(Spacing::MICRO);
}

/// The row-control icons.
///
/// Vector-drawn rather than taken from a font: a glyph only renders at all if
/// the bundled Open Sans happens to cover it, and this set — an eye, a pin, a
/// download tray — is largely outside Latin-1 coverage. Drawing them instead
/// keeps one stroke weight, one colour and one size across both themes,
/// matching the chrome around them rather than whatever the font happens to
/// ship.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    /// Visible; clicking hides.
    Eye,
    /// Hidden; clicking shows.
    EyeHidden,
    /// Pinned; clicking unpins.
    Pin,
    /// Set as the baseline to compare against.
    Baseline,
    /// Download the curve.
    Download,
    /// Remove the curve.
    Close,
    /// The curve's own shape, standing in for a preview.
    Wave,
    /// Add.
    Plus,
}

/// The square an icon is drawn into, before a button's own padding.
const ICON_SIZE: f32 = 12.0;

impl Icon {
    /// Where this icon's parts sit, drawn into `rect`.
    ///
    /// Returned as data rather than painted straight to the painter, for two
    /// reasons: the shapes are then testable without a painter, and a caller
    /// wanting a differently-styled control can reuse them.
    pub fn geometry(self, rect: Rect) -> IconGeometry {
        let center = rect.center();
        // Drawn to the rect, not to a remembered size: a rect is all a caller
        // has promised to give, and an icon that ignored it would not only
        // overflow — it would also be the same size wherever it appeared.
        let unit = rect.width().min(rect.height()).max(0.0) / 12.0;
        let at = |dx: f32, dy: f32| pos2(center.x + dx * unit, center.y + dy * unit);
        let line = |from: (f32, f32), to: (f32, f32)| (at(from.0, from.1), at(to.0, to.1));
        let mut icon = IconGeometry::default();
        match self {
            Self::Eye | Self::EyeHidden => {
                let (left, right) = (at(-4.5, 0.0), at(4.5, 0.0));
                // An almond, opened through the top and the bottom.
                icon.curves.push(quads(left, at(0.0, -4.2), right));
                icon.curves.push(quads(right, at(0.0, 4.2), left));
                icon.discs.push((center, 1.7 * unit));
                if self == Self::EyeHidden {
                    // The slash runs corner to corner, the same diagonal the
                    // eye opens on, so it reads as negating the eye rather than
                    // as an unrelated mark.
                    icon.lines.push(line((-5.0, 5.0), (5.0, -5.0)));
                }
            }
            Self::Pin => {
                // A pushpin on a diagonal: a head, a needle, a foot.
                icon.discs.push((at(-1.2, -2.2), 1.7 * unit));
                icon.lines.push(line((-1.2, -0.4), (1.8, 3.6)));
                icon.lines.push(line((0.4, 1.4), (3.4, 1.2)));
            }
            Self::Baseline => {
                // Two curves meeting on a reference line: this curve, and the
                // one everything is measured against.
                icon.lines.push(line((-4.5, 2.5), (4.5, 2.5)));
                icon.curves
                    .push(quads(at(-4.5, -1.0), at(0.0, -4.4), at(4.5, -1.0)));
                icon.curves
                    .push(quads(at(-4.5, 0.6), at(0.0, -2.0), at(4.5, 0.6)));
            }
            Self::Download => {
                icon.lines.push(line((0.0, -4.6), (0.0, 1.4)));
                icon.lines.push(line((-2.4, -1.2), (0.0, 1.6)));
                icon.lines.push(line((2.4, -1.2), (0.0, 1.6)));
                icon.lines.push(line((-4.4, 4.6), (4.4, 4.6)));
            }
            Self::Close => {
                icon.lines.push(line((-3.4, -3.4), (3.4, 3.4)));
                icon.lines.push(line((3.4, -3.4), (-3.4, 3.4)));
            }
            Self::Wave => {
                // One period of a sine, the shape the tool spends its time on.
                const STEPS: usize = 24;
                let points: Vec<Pos2> = (0..=STEPS)
                    .map(|step| {
                        let phase = step as f32 / STEPS as f32;
                        let x = center.x + (phase - 0.5) * 9.0 * unit;
                        pos2(
                            x,
                            center.y + (phase * std::f32::consts::TAU).sin() * 2.6 * unit,
                        )
                    })
                    .collect();
                icon.lines
                    .extend(points.windows(2).map(|pair| (pair[0], pair[1])));
            }
            Self::Plus => {
                icon.lines.push(line((-4.0, 0.0), (4.0, 0.0)));
                icon.lines.push(line((0.0, -4.0), (0.0, 4.0)));
            }
        }
        icon
    }

    /// Draw this icon into `rect`.
    pub fn paint(self, painter: &egui::Painter, rect: Rect, color: Color32) {
        let geometry = self.geometry(rect).stroked(color);
        for (from, to) in geometry.lines {
            painter.line_segment([from, to], Stroke::new(1.3, color));
        }
        for (center, radius) in geometry.discs {
            painter.circle_filled(center, radius, color);
        }
        for curve in geometry.curves {
            painter.add(Shape::QuadraticBezier(curve));
        }
    }
}

/// A quadratic curve before it has a colour: [`Icon::geometry`] places the
/// points, and [`IconGeometry::stroked`] gives them a weight once the caller
/// knows what it wants.
fn quads(from: Pos2, control: Pos2, to: Pos2) -> egui::epaint::QuadraticBezierShape {
    egui::epaint::QuadraticBezierShape::from_points_stroke(
        [from, control, to],
        false,
        Color32::TRANSPARENT,
        Stroke::NONE,
    )
}

/// What an [`Icon`] is made of: line segments, filled discs, and quadratic
/// curves, each positioned in the rect it was built for.
#[derive(Default, Debug)]
pub struct IconGeometry {
    pub lines: Vec<(Pos2, Pos2)>,
    pub discs: Vec<(Pos2, f32)>,
    pub curves: Vec<egui::epaint::QuadraticBezierShape>,
}

impl IconGeometry {
    /// Give the geometry its stroke, leaving the points where they were.
    fn stroked(self, color: Color32) -> Self {
        let stroke = Stroke::new(1.3, color);
        Self {
            curves: self
                .curves
                .iter()
                .map(|curve| {
                    egui::epaint::QuadraticBezierShape::from_points_stroke(
                        curve.points,
                        curve.closed,
                        Color32::TRANSPARENT,
                        stroke,
                    )
                })
                .collect(),
            ..self
        }
    }

    /// Whether every point the geometry contributes lies inside `bounds`.
    ///
    /// Test-only: nothing at runtime needs to know, and an icon is fixed.
    ///
    /// Everything a quadratic curve reaches is inside the convex hull of its
    /// three points, so testing those is sufficient. An icon that overflowed
    /// its button would touch the control beside it.
    #[cfg(test)]
    fn fits(&self, bounds: Rect) -> bool {
        let hull = self.curves.iter().flat_map(|curve| curve.points);
        self.lines
            .iter()
            .flat_map(|(from, to)| [*from, *to])
            .chain(self.discs.iter().flat_map(|(center, radius)| {
                [
                    *center - Vec2::splat(*radius),
                    *center + Vec2::splat(*radius),
                ]
            }))
            .chain(hull)
            .all(|point| bounds.contains(point))
    }
}

/// An icon-only button.
///
/// Label-free controls need a tooltip to say what they are, and needing one is
/// the reason to reach for this rather than for text: the manager's rows carry
/// one command per curve, and at that density the words cost more than the
/// marks.
pub fn icon_button(ui: &mut Ui, icon: Icon, tooltip: &str) -> egui::Response {
    icon_button_tinted(ui, icon, tooltip, None)
}

/// An icon-only button, with the icon drawn in a colour of the caller's
/// choosing: an icon that doubles as state reads better in the accent.
pub fn icon_button_tinted(
    ui: &mut Ui,
    icon: Icon,
    tooltip: &str,
    tint: Option<Color32>,
) -> egui::Response {
    icon_button_enabled(ui, icon, tooltip, tint, true)
}

/// An icon-only button that can refuse to be pressed.
///
/// A disabled mark still has to be drawn: a control that vanished would leave a
/// gap in the row, and one that stayed bright would invite the click it will not
/// take.
pub fn icon_button_enabled(
    ui: &mut Ui,
    icon: Icon,
    tooltip: &str,
    tint: Option<Color32>,
    enabled: bool,
) -> egui::Response {
    let palette = Palette::resolve(ui.visuals().dark_mode);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(22.0, 22.0),
        if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, theme::RADIUS_CONTROL, palette.hover_fill);
    }
    let color = tint.unwrap_or(if enabled {
        palette.text_secondary
    } else {
        palette.text_disabled
    });
    icon.paint(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(ICON_SIZE)),
        color,
    );
    // A dead control needs why it is dead, not what it would do.
    response
        .on_hover_text(tooltip)
        .on_disabled_hover_text(tooltip)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cap line of a piece of text: the top of the ink it puts on the
    /// screen, measured from the origin its galley is drawn at.
    ///
    /// The cap line is what the eye reads as the top of a line of text, so two
    /// labels that share one are on the same line whatever their sizes.
    fn cap_line(ui: &mut egui::Ui, text: &str, font: egui::FontId) -> f32 {
        let galley = ui
            .painter()
            .layout_no_wrap(text.to_owned(), font, egui::Color32::WHITE);
        galley.rows[0].row.visuals.mesh_bounds.min.y + galley.rows[0].pos.y
    }

    /// The icon button's own footprint, which is also what its icon must stay
    /// inside.
    fn button_rect() -> Rect {
        Rect::from_min_size(pos2(100.0, 100.0), egui::vec2(22.0, 22.0))
    }

    #[test]
    fn the_wordmark_shares_a_cap_line_with_the_category_control() {
        let ctx = egui::Context::default();
        theme::register_fonts(&ctx);
        let category = 0usize;
        let mut wordmark_box = egui::Rect::ZERO;
        let mut pill_box = egui::Rect::ZERO;
        let mut wordmark_cap = 0.0_f32;
        let mut pill_cap = 0.0_f32;
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.horizontal(|ui| {
                wordmark_box = wordmark(ui);
                ui.add_space(16.0);
                egui::Frame::group(ui.style())
                    .inner_margin(egui::Margin::same(2))
                    .fill(ui.visuals().extreme_bg_color)
                    .corner_radius(egui::CornerRadius::same(6))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        for (index, label) in ["EARBUDS", "HEADPHONES", "IEMS"].iter().enumerate() {
                            let response = ui.selectable_label(index == category, *label);
                            if index == 2 {
                                pill_box = response.rect;
                            }
                        }
                    });
                // The cap lines, each measured from the origin its own text is
                // drawn at.
                wordmark_cap = wordmark_box.top()
                    + cap_line(ui, "SquigLab", theme::font_bold(theme::TypeStep::Subtitle));
                pill_cap =
                    pill_box.top() + cap_line(ui, "IEMS", theme::font(theme::TypeStep::Body));
            });
        });
        let mut textures = output.textures_delta;
        let tex_manager = ctx.tex_manager();
        let mut manager = tex_manager.write();
        for (id, deltas) in &textures.set {
            for delta in deltas {
                manager.set(*id, delta.clone());
            }
        }
        // The delta has been applied; say so, or it complains when it is
        // dropped.
        textures.clear();
        let difference = (wordmark_cap - pill_cap).abs();
        assert!(
            difference <= 1.0,
            "the wordmark's cap line is {difference:.1} points from the control's"
        );
    }

    /// Every icon, so a new one is covered without editing the tests.
    const ALL_ICONS: [Icon; 8] = [
        Icon::Eye,
        Icon::EyeHidden,
        Icon::Pin,
        Icon::Baseline,
        Icon::Download,
        Icon::Close,
        Icon::Wave,
        Icon::Plus,
    ];

    #[test]
    fn every_icon_fits_inside_the_button_it_lives_in() {
        let rect = button_rect();
        let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(ICON_SIZE));
        for icon in ALL_ICONS {
            let geometry = icon.geometry(icon_rect);
            let filled = geometry.lines.is_empty() && geometry.discs.is_empty();
            assert!(!filled, "{icon:?} draws nothing at all");
            assert!(
                geometry.fits(rect),
                "{icon:?} overflows the button it is drawn in"
            );
        }
    }

    #[test]
    fn an_icon_drawn_in_a_degenerate_rect_collapses_to_its_center() {
        // A zero-sized rect is what a panel squeezed shut hands a widget. It
        // must not panic, and nothing may reach outside the point it was given.
        for icon in ALL_ICONS {
            let rect = Rect::from_min_size(pos2(37.0, 41.0), egui::vec2(0.0, 0.0));
            let geometry = icon.geometry(rect);
            let points = geometry
                .lines
                .iter()
                .flat_map(|(from, to)| [*from, *to])
                .chain(geometry.discs.iter().map(|(center, _)| *center))
                .chain(geometry.curves.iter().flat_map(|curve| curve.points));
            for point in points {
                assert_eq!(point, rect.center(), "{icon:?} escaped a collapsed rect");
            }
        }
    }

    #[test]
    fn the_hidden_eye_adds_the_slash_the_visible_one_does_not() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(22.0, 22.0));
        let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(ICON_SIZE));
        assert!(Icon::Eye.geometry(icon_rect).lines.is_empty());
        assert_eq!(Icon::EyeHidden.geometry(icon_rect).lines.len(), 1);
    }

    #[test]
    fn a_stroked_geometry_keeps_every_point_where_it_was() {
        let rect = button_rect();
        let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(ICON_SIZE));
        let before = Icon::Eye.geometry(icon_rect);
        let after = Icon::Eye.geometry(icon_rect).stroked(Color32::WHITE);
        assert_eq!(before.lines, after.lines);
        assert_eq!(before.curves.len(), after.curves.len());
    }
}
