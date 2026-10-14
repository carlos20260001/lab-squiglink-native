//! Reusable interface components built on the tokens in [`crate::theme`].
//!
//! Every widget here resolves colors and sizes from the token set rather than
//! hard-coding them, so the two themes and any future theme change apply here
//! automatically.

use egui::{Align, Color32, FontId, RichText, Stroke, Ui};

use crate::graph::ZoomRegion;
use crate::theme::{self, Palette, TypeStep};

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

/// The application wordmark in the toolbar.
pub fn wordmark(ui: &mut Ui) {
    let palette = Palette::resolve(ui.visuals().dark_mode);
    ui.label(
        RichText::new("SquigLab")
            .font(theme::font_bold(TypeStep::Subtitle))
            .color(palette.text_primary),
    );
}

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
