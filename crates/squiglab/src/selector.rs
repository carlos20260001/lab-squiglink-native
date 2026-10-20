//! The left column's list: the brands, and the models under them.
//!
//! The reference's selector has three tabs, and this is the first two of
//! them: the brands, and the measurements of the brand that is chosen. The
//! third tab is the equalizer, which the equalizer panel draws.

use egui::{Align, Layout, RichText, Ui};

use crate::app::{App, Tab};
use crate::theme::{self, metrics, TypeStep};
use squiglab_core::session::{matches_search, MeasurementGroup};

/// The tabs, which stay put while the panel beneath them scrolls.
pub fn show_tabs(ui: &mut Ui, app: &mut App) {
    tabs(ui, app);
}

/// The list under the tabs.
pub fn show_list(ui: &mut Ui, app: &mut App) {
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
///
/// The reference draws them as three buttons joined side by side, each a
/// third of the column's width, with the chosen one filled in the accent.
fn tabs(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();
    let width = (ui.available_width() - metrics::GAP * 2.0) / 3.0;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (index, tab) in [Tab::Brands, Tab::Models, Tab::Equalizer]
            .into_iter()
            .enumerate()
        {
            let selected = app.tab == tab;
            let radius = match index {
                0 => egui::CornerRadius {
                    nw: metrics::RADIUS as u8,
                    sw: metrics::RADIUS as u8,
                    ..egui::CornerRadius::ZERO
                },
                2 => egui::CornerRadius {
                    ne: metrics::RADIUS as u8,
                    se: metrics::RADIUS as u8,
                    ..egui::CornerRadius::ZERO
                },
                _ => egui::CornerRadius::ZERO,
            };
            let button = egui::Button::new(
                RichText::new(tab.label())
                    .font(TypeStep::Control.font())
                    .color(if selected {
                        palette.on_accent
                    } else {
                        palette.text_secondary
                    }),
            )
            .fill(if selected {
                palette.accent
            } else {
                palette.page
            })
            .stroke(egui::Stroke::new(
                1.0,
                if selected {
                    palette.accent
                } else {
                    palette.border
                },
            ))
            .corner_radius(radius);
            let response = ui
                .add_sized([width, 34.0], button)
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
    // The rule under the tab row.
    ui.add_space(metrics::GAP);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 0.0, theme::current_palette().divider);
    ui.add_space(metrics::GAP);
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

    // The reference's brands tab: the list of brands, and the models of the
    // brand that is chosen sliding in from the right, so that a sliver of
    // them is still showing.
    let height = ui.available_height();
    let origin = ui.cursor().min;
    ui.horizontal_top(|ui| {
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(egui::Rect::from_min_size(
                    origin,
                    egui::vec2(BRANDS_WIDTH, height),
                ))
                .layout(egui::Layout::top_down(egui::Align::Min)),
            |ui| {
                for (brand, _count) in counts {
                    if !app.search.is_empty() && !matches_search(&brand, &app.search) {
                        continue;
                    }
                    let selected = app.brand.as_deref() == Some(brand.as_str());
                    // The reference's list of brands carries the names alone.
                    let response = ui
                        .selectable_label(
                            selected,
                            RichText::new(brand.clone())
                                .font(TypeStep::Body.font())
                                .color(if selected {
                                    palette.on_accent
                                } else {
                                    palette.text_primary
                                }),
                        )
                        .on_hover_text("Show this brand's measurements");
                    if response.clicked() {
                        app.brand = Some(brand.clone());
                        // The reference goes to the models of the brand that
                        // is pressed.
                        app.tab = Tab::Models;
                    }
                }
            },
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(egui::Rect::from_min_size(
                    egui::pos2(origin.x + BRANDS_WIDTH, origin.y),
                    egui::vec2(BRANDS_WIDTH, height),
                ))
                .layout(egui::Layout::top_down(egui::Align::Min)),
            |ui| model_list(ui, app),
        );
    });
}

/// The width of the reference's list of brands.
const BRANDS_WIDTH: f32 = 289.0;

/// The list of the chosen brand's measurements.
fn models(ui: &mut Ui, app: &mut App) {
    // The reference's models tab shows the brands down the left and the
    // measurements of the chosen brand beside them.
    let palette = theme::current_palette();
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
        // The reference's list of models slides over the list of brands, so
        // it starts where the reference's starts and covers what is behind
        // it.
        let list_x = origin.x + MODELS_LIST_OFFSET;
        ui.painter().rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(list_x, origin.y),
                egui::vec2(metrics::LIST_WIDTH, height),
            ),
            0.0,
            palette.page,
        );
        // What is left of the brands behind is quiet: the reference veils
        // them with the page's own colour, so the names behind fade.
        ui.painter().rect_filled(
            egui::Rect::from_min_size(origin, egui::vec2(MODELS_LIST_OFFSET, height)),
            0.0,
            egui::Color32::from_rgba_unmultiplied(
                palette.page.r(),
                palette.page.g(),
                palette.page.b(),
                204,
            ),
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(egui::Rect::from_min_size(
                    egui::pos2(list_x, origin.y),
                    egui::vec2(metrics::LIST_WIDTH, height),
                ))
                .layout(egui::Layout::top_down(egui::Align::Min)),
            |ui| model_list(ui, app),
        );
    });
}

/// Where the reference's list of models begins, in the column.
const MODELS_LIST_OFFSET: f32 = 61.0;

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
                // The reference's rows are forty tall with six points
                // between them.
                ui.add_space(metrics::ROW_GAP);
                let selected = app.brand.as_deref() == Some(brand.as_str());
                // The reference clips the brand names rather than wrapping
                // them, so the column stays a fixed width.
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), metrics::ROW_HEIGHT),
                    egui::Sense::click(),
                );
                if selected {
                    ui.painter()
                        .rect_filled(rect, metrics::RADIUS, palette.accent);
                }
                let label = ui.put(
                    rect,
                    egui::Label::new(
                        RichText::new(shorten(
                            ui.ctx(),
                            &brand,
                            metrics::BRAND_COLUMN_WIDTH - 16.0,
                        ))
                        .font(TypeStep::Body.font())
                        .color(if selected {
                            palette.on_accent
                        } else {
                            palette.text_secondary
                        }),
                    )
                    .truncate(),
                );
                if label
                    .on_hover_text(format!("Show {brand}'s measurements"))
                    .clicked()
                    || response.clicked()
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
            // The reference's list: every measurement in its place, and the
            // ones that are on the graph drawn as a black pill with the
            // cross that takes them off.
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
                // The reference's rows are forty tall with six points
                // between them.
                ui.add_space(metrics::ROW_GAP);
                if on {
                    // The reference's row for a measurement that is on the
                    // graph: a black pill, and the cross at its right.
                    let mark = 36.0;
                    let width = ui.available_width() - mark;
                    ui.horizontal(|ui| {
                        // The name is written at the left of the pill, as the
                        // reference writes it.
                        let (pill, response) =
                            ui.allocate_exact_size(egui::vec2(width, 36.0), egui::Sense::click());
                        ui.painter().rect_filled(pill, 6.0, palette.text_primary);
                        ui.painter().text(
                            pill.left_center() + egui::vec2(12.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            group.name.clone(),
                            TypeStep::Body.font(),
                            palette.on_accent,
                        );
                        response.on_hover_text("This measurement is on the graph");
                        let (rect, response) =
                            ui.allocate_exact_size(egui::vec2(mark, 36.0), egui::Sense::click());
                        let glyph =
                            egui::Rect::from_center_size(rect.center(), egui::vec2(16.0, 16.0));
                        crate::icons::draw(
                            ui.painter(),
                            glyph,
                            crate::icons::Icon::Remove,
                            palette.on_accent,
                        );
                        if response
                            .on_hover_text("Take this measurement off the graph")
                            .clicked()
                        {
                            app.session.active.remove(&index);
                            app.session.rebuild_display_curves();
                        }
                    });
                    continue;
                }
                ui.horizontal(|ui| {
                    let response = ui
                        .selectable_label(
                            false,
                            RichText::new(group.name.clone())
                                .font(TypeStep::Body.font())
                                .color(palette.text_primary),
                        )
                        .on_hover_text("Show this measurement on the graph");
                    if response.clicked() {
                        app.session.active.insert(index);
                        app.session.eq_phone = Some(index);
                        app.session.rebuild_display_curves();
                    }
                    // The reference's rows carry a plus at their end, which
                    // puts the measurement on the graph.
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // The mark is the reference's own cross, in its own
                        // hundred-and-twenty point box.
                        let (rect, response) =
                            ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::click());
                        let mark =
                            egui::Rect::from_center_size(rect.center(), egui::vec2(16.0, 16.0));
                        crate::icons::draw(
                            ui.painter(),
                            mark,
                            crate::icons::Icon::Plus,
                            palette.text_tertiary,
                        );
                        if response
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
