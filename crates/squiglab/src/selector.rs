//! The left column's list: the brands, and the models under them.
//!
//! The reference's selector has three tabs, and this is the first two of
//! them: the brands, and the measurements of the brand that is chosen. The
//! third tab is the equalizer, which the equalizer panel draws.

use egui::{Align, Layout, RichText, Ui};

use crate::app::{App, Tab};
use crate::theme::{self, metrics, TypeStep};
use squiglab_core::session::{matches_search, MeasurementGroup};

/// The tabs, and the list under them.
pub fn show(ui: &mut Ui, app: &mut App) {
    tabs(ui, app);
    ui.add_space(metrics::GAP);

    // --- the search ---
    let mut search = app.search.clone();
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut search)
                .hint_text("SEARCH")
                .desired_width(ui.available_width() - metrics::PADDING * 2.0),
        );
    });
    app.search = search;
    ui.add_space(metrics::GAP);

    match app.tab {
        Tab::Brands => brands(ui, app),
        Tab::Models => models(ui, app),
        Tab::Equalizer => {}
    }
}

/// The three tabs.
fn tabs(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();
    ui.horizontal(|ui| {
        for tab in [Tab::Brands, Tab::Models, Tab::Equalizer] {
            let selected = app.tab == tab;
            let response = ui
                .selectable_label(
                    selected,
                    RichText::new(tab.label())
                        .font(TypeStep::Control.font())
                        .color(if selected {
                            palette.text_primary
                        } else {
                            palette.text_secondary
                        }),
                )
                .on_hover_text(match tab {
                    Tab::Brands => "The brands, and the measurements under each",
                    Tab::Models => "The measurements of the brand that is chosen",
                    Tab::Equalizer => "The equalizer of the selected measurement",
                });
            if response.clicked() {
                app.tab = tab;
            }
        }
    });
}

/// The list of brands.
fn brands(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();
    let mut counts: Vec<(String, usize)> = Vec::new();
    for group in &app.session.groups {
        if group.target {
            continue;
        }
        match counts.iter_mut().find(|(brand, _)| *brand == group.brand) {
            Some((_, count)) => *count += 1,
            None => counts.push((group.brand.clone(), 1)),
        }
    }
    counts.sort_by_key(|(brand, _)| brand.to_lowercase());

    for (brand, count) in counts {
        if !app.search.is_empty() && !matches_search(&brand, &app.search) {
            continue;
        }
        let selected = app.brand.as_deref() == Some(brand.as_str());
        ui.horizontal(|ui| {
            let response = ui
                .selectable_label(
                    selected,
                    RichText::new(brand.clone())
                        .font(TypeStep::Body.font())
                        .color(palette.text_primary),
                )
                .on_hover_text("Show this brand's measurements");
            if response.clicked() {
                app.brand = Some(brand.clone());
                app.tab = Tab::Models;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(count.to_string())
                        .font(TypeStep::Label.font())
                        .color(palette.text_tertiary),
                );
            });
        });
    }
}

/// The list of the chosen brand's measurements.
fn models(ui: &mut Ui, app: &mut App) {
    // The reference's models tab shows the brands down the left and the
    // measurements of the chosen brand beside them.
    let height = ui.available_height();
    // Where the columns begin: below the tabs and the search.
    let origin = ui.cursor().min;
    ui.horizontal_top(|ui| {
        // The brand column is narrow, and its names are cut short rather
        // than wrapped: the reference does the same, and the whole name is in
        // the tooltip.
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(egui::Rect::from_min_size(
                    origin,
                    egui::vec2(metrics::BRAND_COLUMN_WIDTH, height),
                ))
                .layout(egui::Layout::top_down(egui::Align::Min)),
            |ui| brand_column(ui, app),
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(egui::Rect::from_min_size(
                    egui::pos2(origin.x + metrics::BRAND_COLUMN_WIDTH, origin.y),
                    egui::vec2(metrics::LIST_WIDTH, height),
                ))
                .layout(egui::Layout::top_down(egui::Align::Min)),
            |ui| model_list(ui, app),
        );
    });
}

/// The brands down the left of the models tab.
fn brand_column(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();
    let mut counts: Vec<(String, usize)> = Vec::new();
    for group in &app.session.groups {
        if group.target {
            continue;
        }
        match counts.iter_mut().find(|(brand, _)| *brand == group.brand) {
            Some((_, count)) => *count += 1,
            None => counts.push((group.brand.clone(), 1)),
        }
    }
    counts.sort_by_key(|(brand, _)| brand.to_lowercase());
    egui::ScrollArea::vertical()
        .id_salt("brand_column")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (brand, _) in counts {
                if !app.search.is_empty() && !matches_search(&brand, &app.search) {
                    continue;
                }
                let selected = app.brand.as_deref() == Some(brand.as_str());
                if ui
                    .selectable_label(
                        selected,
                        RichText::new(shorten(
                            ui.ctx(),
                            &brand,
                            metrics::BRAND_COLUMN_WIDTH - 16.0,
                        ))
                        .font(TypeStep::Body.font())
                        .color(if selected {
                            palette.on_accent
                        } else {
                            palette.text_primary
                        }),
                    )
                    .on_hover_text(format!("Show {brand}'s measurements"))
                    .clicked()
                {
                    app.brand = Some(brand.clone());
                }
            }
        });
}

/// The measurements of the chosen brand.
fn model_list(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();
    let Some(brand) = app.brand.clone() else {
        // The reference opens on the models of a brand, not on an empty
        // column, so the first brand is chosen.
        let first = app
            .session
            .groups
            .iter()
            .find(|group| !group.target)
            .map(|group| group.brand.clone());
        match first {
            Some(brand) => {
                app.brand = Some(brand.clone());
                model_list(ui, app);
                return;
            }
            None => {
                ui.label(
                    RichText::new("There are no measurements to show.")
                        .font(TypeStep::Body.font())
                        .color(palette.text_secondary),
                );
                return;
            }
        }
    };
    let groups: Vec<MeasurementGroup> = app
        .session
        .groups
        .iter()
        .filter(|group| !group.target && group.brand == brand)
        .cloned()
        .collect();
    egui::ScrollArea::vertical()
        .id_salt("model_list")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // The measurements that are on the graph lead the list, as chips
            // the reference's do, and each carries the cross that takes it off.
            let active: Vec<usize> = app.session.active.iter().copied().collect();
            for index in active {
                let name = app.session.groups[index].name.clone();
                let width = ui.available_width() - 28.0;
                ui.horizontal_top(|ui| {
                    ui.add_sized(
                        [width, 26.0],
                        egui::Button::new(
                            RichText::new(name.clone())
                                .font(TypeStep::Body.font())
                                .color(palette.on_accent),
                        )
                        .fill(palette.text_primary)
                        .stroke(egui::Stroke::NONE)
                        .corner_radius(egui::CornerRadius::same(4)),
                    )
                    .on_hover_text("This measurement is on the graph");
                    if ui
                        .add_sized(
                            [20.0, 26.0],
                            egui::Button::new(
                                RichText::new("×")
                                    .font(TypeStep::Body.font())
                                    .color(palette.text_secondary),
                            )
                            .fill(egui::Color32::TRANSPARENT)
                            .stroke(egui::Stroke::NONE),
                        )
                        .on_hover_text("Take this measurement off the graph")
                        .clicked()
                    {
                        app.session.active.remove(&index);
                        app.session.rebuild_display_curves();
                    }
                });
            }
            for group in &groups {
                if !app.search.is_empty() && !matches_search(&group.name, &app.search) {
                    continue;
                }
                let index = app
                    .session
                    .groups
                    .iter()
                    .position(|candidate| candidate.name == group.name)
                    .unwrap_or(0);
                let on = app.session.active.contains(&index);
                ui.horizontal(|ui| {
                    let response = ui
                        .selectable_label(
                            on,
                            RichText::new(group.name.clone())
                                .font(TypeStep::Body.font())
                                .color(if on {
                                    palette.on_accent
                                } else {
                                    palette.text_primary
                                }),
                        )
                        .on_hover_text("Show this measurement on the graph");
                    if response.clicked() {
                        if on {
                            app.session.active.remove(&index);
                        } else {
                            app.session.active.insert(index);
                        }
                        app.session.eq_phone = Some(index);
                        app.session.rebuild_display_curves();
                    }
                    // The reference's rows carry a plus at their end, which
                    // puts the measurement on the graph.
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add_sized(
                                [20.0, 20.0],
                                egui::Button::new(
                                    RichText::new("+")
                                        .font(TypeStep::Body.font())
                                        .color(palette.text_tertiary),
                                )
                                .fill(egui::Color32::TRANSPARENT)
                                .stroke(egui::Stroke::NONE),
                            )
                            .on_hover_text("Put this measurement on the graph")
                            .clicked()
                            && !on
                        {
                            app.session.active.insert(index);
                            app.session.eq_phone = Some(index);
                            app.session.rebuild_display_curves();
                        }
                    });
                });
                augment(ui, group);
            }
        });
}

/// The card the reference shows under a model: the review, the price, and
/// where to buy it.
fn augment(ui: &mut Ui, group: &MeasurementGroup) {
    let palette = theme::current_palette();
    if group.review_score.is_none() && group.price.is_none() && group.shop_link.is_none() {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        if let Some(score) = &group.review_score {
            ui.label(
                RichText::new(stars(score))
                    .font(TypeStep::Label.font())
                    .color(palette.text_secondary),
            );
        }
        if let Some(price) = &group.price {
            ui.label(
                RichText::new(price.clone())
                    .font(TypeStep::Label.font())
                    .color(palette.text_secondary),
            );
        }
        if let Some(link) = &group.shop_link {
            if ui
                .link(RichText::new("Shop").font(TypeStep::Label.font()))
                .on_hover_text(link)
                .clicked()
            {
                ui.ctx().open_url(egui::OpenUrl::new_tab(link.clone()));
            }
        }
        if let Some(link) = &group.review_link {
            if ui
                .link(RichText::new("Review").font(TypeStep::Label.font()))
                .on_hover_text(link)
                .clicked()
            {
                ui.ctx().open_url(egui::OpenUrl::new_tab(link.clone()));
            }
        }
    });
}

/// The score as the reference shows it: stars for a number, the grade itself
/// for a letter.
fn stars(score: &str) -> String {
    match score.trim().parse::<f32>() {
        Ok(number) => "\u{2605}".repeat(number.clamp(0.0, 5.0).round() as usize),
        Err(_) => score.to_owned(),
    }
}

/// A name cut short to the width it has, the way the reference cuts the
/// brands in its narrow column.
fn shorten(ctx: &egui::Context, name: &str, width: f32) -> String {
    let font = TypeStep::Body.font();
    // The fonts belong to the running context: a context of one's own has none
    // until it has run a frame, and asking it for them panics.
    let measured = |text: &str| {
        ctx.fonts_mut(|fonts| {
            text.chars()
                .map(|character| fonts.glyph_width(&font, character))
                .sum::<f32>()
        })
    };
    if measured(name) <= width {
        return name.to_owned();
    }
    let mut cut = name.to_owned();
    while cut.chars().count() > 1 {
        cut.pop();
        let candidate = format!("{cut}…");
        if measured(&candidate) <= width {
            return candidate;
        }
    }
    cut
}
