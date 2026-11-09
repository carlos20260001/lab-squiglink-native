//! The shell: the window's frame, and the state the column reads.
//!
//! This is the first milestone of the GPUI interface: the header, the tab row,
//! the search, and the list of brands and measurements. The graph, the
//! equalizer and the sound tools are still drawn by the egui interface.

use std::path::PathBuf;
use std::sync::Arc;

use gpui::prelude::*;
use gpui::{
    div, img, px, AnyElement, Context, FontWeight, Image, ImageFormat, Render, Rgba, Window,
};
use squiglab_core::product;
use squiglab_core::session::{matches_search, DataPaths, Session};

use crate::icons;
use crate::icons::Icon;
use crate::theme::{self, metrics, Palette};

/// The three tabs of the left column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    /// The brands, and the measurements under each.
    Brands,
    /// The models, and the measurements of each.
    Models,
    /// The equalizer of the selected measurement.
    Equalizer,
}

impl Tab {
    /// The name the reference gives the tab.
    pub fn label(self) -> &'static str {
        match self {
            Tab::Brands => "Brands",
            Tab::Models => "Models",
            Tab::Equalizer => "Equalizer",
        }
    }
}

/// The application: the state, and the frame it draws.
pub struct App {
    /// The program: what is loaded, what is selected, and what has changed.
    session: Session,
    /// The tab of the left column that is open.
    tab: Tab,
    /// The search in the list.
    search: String,
    /// The brand whose measurements the list shows.
    brand: Option<String>,
    /// The message shown at the foot of the column.
    status: Option<String>,
    /// The product's mark, decoded once and kept.
    icon: Arc<Image>,
}

impl App {
    /// The application, with the measurements loaded.
    pub fn new() -> Self {
        let mut session = Session::default();
        let paths = DataPaths {
            measurements: PathBuf::from("data"),
            headphones: PathBuf::from("data_hp"),
            corrections: PathBuf::from("data_deltas"),
        };
        let mut status = None;
        match session.load(&paths, None) {
            Ok(loaded) => {
                if let Some(problem) = loaded.problem {
                    status = Some(problem);
                }
            }
            Err(problem) => status = Some(problem),
        }
        // The equalizer works on one measurement and against one target: the
        // ones that are on the graph, if there are any.
        if session.eq_phone.is_none() {
            session.eq_phone = session.active.iter().next().copied();
        }
        if session.eq_target.is_none() {
            session.eq_target = session.active_targets.iter().next().copied();
        }
        session.rebuild_display_curves();
        // The reference opens on the models of a brand, not on an empty
        // column, so the first brand is chosen.
        let brand = session
            .groups
            .iter()
            .find(|group| !group.target)
            .map(|group| group.brand.clone());
        Self {
            session,
            tab: Tab::Models,
            search: String::new(),
            brand,
            status,
            icon: Arc::new(Image::from_bytes(
                ImageFormat::Png,
                product::WINDOW_ICON_PNG.to_vec(),
            )),
        }
    }

    /// The brands, with the number of measurements filed under each.
    fn brand_counts(&self) -> Vec<(String, usize)> {
        let mut counts: Vec<(String, usize)> = Vec::new();
        for group in &self.session.groups {
            if group.target {
                continue;
            }
            match counts.iter_mut().find(|(brand, _)| *brand == group.brand) {
                Some((_, count)) => *count += 1,
                None => counts.push((group.brand.clone(), 1)),
            }
        }
        counts.sort_by_key(|(brand, _)| brand.to_lowercase());
        counts
    }

    fn header(&self, palette: &Palette) -> impl IntoElement {
        div()
            .h(px(metrics::HEADER_HEIGHT))
            .w_full()
            .flex_shrink_0()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .px(px(metrics::PADDING))
            .bg(palette.header)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(metrics::GAP))
                    .child(
                        img(self.icon.clone())
                            .w(px(20.0))
                            .h(px(20.0))
                            .flex_shrink_0(),
                    )
                    .child(
                        div()
                            .text_size(px(theme::type_scale::LOGO))
                            .font_weight(FontWeight::BOLD)
                            .text_color(palette.on_accent)
                            .child(product::APP_NAME),
                    ),
            )
            .child(
                div()
                    .text_size(px(theme::type_scale::LABEL))
                    .text_color(palette.text_secondary)
                    .child(format!("{} curves", self.session.display_curves.len())),
            )
    }

    fn left_column(&self, palette: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let mut column = div()
            .w(px(metrics::COLUMN_WIDTH))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(palette.page)
            // The reference closes the column with a one-point rule against
            // the graph, not a gap.
            .border_r(px(1.0))
            .border_color(palette.border_strong)
            // The reference's tab row carries sixteen points of padding on
            // its three sides and a rule across the full column beneath it.
            .child(
                div()
                    .px(px(metrics::COLUMN_PADDING))
                    .pt(px(metrics::PADDING))
                    .pb(px(metrics::PADDING))
                    .border_b(px(1.0))
                    .border_color(palette.divider)
                    .flex_shrink_0()
                    .child(self.tabs(palette, cx)),
            )
            .child(
                div()
                    .px(px(metrics::COLUMN_PADDING))
                    .pt(px(metrics::PADDING))
                    .flex_shrink_0()
                    .child(self.search_field(palette)),
            )
            .child(
                div()
                    .px(px(metrics::COLUMN_PADDING))
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .flex_col()
                    .child(self.list(palette, cx)),
            );
        if let Some(status) = &self.status {
            column = column.child(
                div()
                    .py(px(metrics::GAP_LARGE))
                    .text_size(px(theme::type_scale::LABEL))
                    .text_color(palette.text_secondary)
                    .child(status.clone()),
            );
        }
        column
    }

    /// The three tabs: three buttons joined side by side, each a third of the
    /// column's width, with the chosen one filled in the accent.
    fn tabs(&self, palette: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let mut row = div()
            .flex()
            .flex_row()
            .w_full()
            .h(px(metrics::TAB_HEIGHT))
            .flex_shrink_0();
        for (index, tab) in [Tab::Brands, Tab::Models, Tab::Equalizer]
            .into_iter()
            .enumerate()
        {
            let selected = self.tab == tab;
            let mut button = div()
                .id(("tab", index))
                .flex_1()
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .border_color(if selected {
                    palette.accent
                } else {
                    palette.border_strong
                })
                .bg(if selected {
                    palette.accent
                } else {
                    palette.page
                })
                .text_size(px(theme::type_scale::CONTROL))
                .text_color(if selected {
                    palette.on_accent
                } else {
                    palette.text_quiet
                })
                .cursor_pointer()
                .child(tab.label());
            // The reference rounds the row's two ends, not the joins, and
            // each button draws the outer edge only: the border between two
            // buttons belongs to the button on its right.
            button = match index {
                0 => button
                    .border_t(px(1.0))
                    .border_b(px(1.0))
                    .border_l(px(1.0))
                    .rounded_l(px(metrics::RADIUS)),
                1 => button.border_1(),
                _ => button
                    .border_t(px(1.0))
                    .border_b(px(1.0))
                    .border_r(px(1.0))
                    .rounded_r(px(metrics::RADIUS)),
            };
            row = row.child(button.on_click(cx.listener(move |this, _, _, _| {
                this.tab = tab;
            })));
        }
        row
    }

    fn search_field(&self, palette: &Palette) -> impl IntoElement {
        div()
            .h(px(metrics::INPUT_HEIGHT))
            .w_full()
            .flex_shrink_0()
            .flex()
            .items_center()
            .px(px(metrics::PADDING))
            .rounded(px(metrics::RADIUS))
            .border_1()
            .border_color(palette.border_strong)
            .bg(palette.field)
            .text_size(px(theme::type_scale::LABEL))
            .text_color(palette.text_input)
            .child("SEARCH")
    }

    fn list(&self, palette: &Palette, cx: &mut Context<Self>) -> AnyElement {
        match self.tab {
            Tab::Brands => self.brands(palette, cx).into_any_element(),
            Tab::Models => self.models(palette, cx).into_any_element(),
            Tab::Equalizer => div().flex_1().w_full().min_h(px(0.0)).into_any_element(),
        }
    }

    /// The brands, as one scrolling list, with the models of the chosen brand
    /// beginning to slide in from the right. The reference keeps sixty points
    /// of the brand column showing, so the tab reads as a pair of panels.
    fn brands(&self, palette: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let strip = div()
            .absolute()
            .left(px(273.0))
            .top(px(0.0))
            .w(px(60.0))
            .h_full()
            .overflow_hidden()
            .child(self.model_panel(palette, cx, 0.0, 289.0));
        div()
            .relative()
            .w_full()
            .flex_1()
            .min_h(px(0.0))
            .child(self.brand_list(palette, cx))
            .child(strip)
    }

    /// The brands, as one scrolling list.
    fn brand_list(&self, palette: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let mut list = div()
            .id("brands_list")
            .absolute()
            .left(px(0.0))
            .top(px(0.0))
            .w(px(257.0))
            .h_full()
            .overflow_y_scroll()
            .flex()
            .flex_col();
        let mut first = true;
        for (index, (brand, _)) in self.brand_counts().into_iter().enumerate() {
            if !self.search.is_empty() && !matches_search(&brand, &self.search) {
                continue;
            }
            let selected = self.brand.as_deref() == Some(brand.as_str());
            let name = brand.clone();
            let mut row = div()
                .id(("brand_row", index))
                .h(px(metrics::ROW_HEIGHT))
                .mt(px(if first {
                    metrics::PADDING
                } else {
                    metrics::ROW_GAP
                }))
                .flex_shrink_0()
                .flex()
                .items_center()
                .px(px(12.0))
                .rounded(px(metrics::RADIUS))
                .text_size(px(theme::type_scale::BODY))
                .text_color(if selected {
                    palette.on_accent
                } else {
                    palette.text_primary
                })
                .cursor_pointer()
                .child(div().truncate().child(brand.clone()));
            first = false;
            if selected {
                row = row.bg(palette.accent);
            }
            list = list.child(row.on_click(cx.listener(move |this, _, _, _| {
                this.brand = Some(name.clone());
                // The reference goes to the models of the brand that is
                // pressed.
                this.tab = Tab::Models;
            })));
        }
        list
    }

    /// The brands down the left of the models tab, and the measurements of
    /// the chosen brand beside them.
    fn models(&self, palette: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let mut brand_column = div()
            .id("brand_column")
            .absolute()
            .left(px(0.0))
            .top(px(0.0))
            .w(px(257.0))
            .h_full()
            .overflow_y_scroll()
            .flex()
            .flex_col();
        let mut first = true;
        for (index, (brand, _)) in self.brand_counts().into_iter().enumerate() {
            if !self.search.is_empty() && !matches_search(&brand, &self.search) {
                continue;
            }
            let selected = self.brand.as_deref() == Some(brand.as_str());
            let name = brand.clone();
            let mut row = div()
                .id(("brand_column_row", index))
                .h(px(metrics::ROW_HEIGHT))
                .mt(px(if first {
                    metrics::PADDING
                } else {
                    metrics::ROW_GAP
                }))
                .flex_shrink_0()
                .flex()
                .items_center()
                .px(px(12.0))
                .rounded(px(metrics::RADIUS))
                .overflow_hidden()
                .text_size(px(theme::type_scale::BODY))
                .text_color(if selected {
                    palette.on_accent
                } else {
                    palette.text_primary
                })
                .cursor_pointer()
                .child(div().truncate().child(brand.clone()));
            first = false;
            if selected {
                row = row.bg(palette.accent);
            }
            brand_column = brand_column.child(row.on_click(cx.listener(move |this, _, _, _| {
                this.brand = Some(name.clone());
            })));
        }

        // The reference slides the models over the brands and leaves the
        // first forty-four points of the brand column showing, veiled by the
        // page's own colour so the names behind fade rather than disappear.
        let veil = Rgba {
            a: 0.8,
            ..palette.page
        };
        div()
            .relative()
            .w_full()
            .flex_1()
            .min_h(px(0.0))
            .child(brand_column)
            .child(
                div()
                    .absolute()
                    .left(px(0.0))
                    .top(px(0.0))
                    .w(px(44.0))
                    .h_full()
                    .bg(veil),
            )
            .child(self.model_panel(palette, cx, 44.0, 289.0))
    }

    /// The measurements of the chosen brand, as the panel that slides over
    /// the brand column.
    fn model_panel(
        &self,
        palette: &Palette,
        cx: &mut Context<Self>,
        left: f32,
        width: f32,
    ) -> impl IntoElement {
        let mut model_list = div()
            .id("model_list")
            .absolute()
            .left(px(left))
            .top(px(0.0))
            .w(px(width))
            .h_full()
            .border_l(px(1.0))
            .border_color(palette.border_strong)
            .pl(px(16.0))
            .pr(px(10.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .bg(palette.page);
        let mut first = true;
        for (index, group) in self.session.groups.iter().enumerate() {
            if group.target || self.brand.as_deref() != Some(group.brand.as_str()) {
                continue;
            }
            if !self.search.is_empty() && !matches_search(&group.name, &self.search) {
                continue;
            }
            let on = self.session.active.contains(&index);
            let name = group.name.clone();
            let mut row = div()
                .id(("model_row", index))
                .w(px(256.0))
                .h(px(metrics::ROW_HEIGHT))
                .mt(px(if first {
                    metrics::PADDING
                } else {
                    metrics::ROW_GAP
                }))
                .flex_shrink_0()
                .flex()
                .flex_row()
                .items_center()
                .text_size(px(theme::type_scale::BODY))
                .cursor_pointer();
            first = false;
            // The reference's row for a measurement that is on the graph is
            // one black pill across the list, with its cross at the right.
            row = if on {
                row.rounded(px(metrics::RADIUS))
                    .bg(palette.text_primary)
                    .text_color(palette.on_primary)
            } else {
                row.text_color(palette.text_primary)
            };
            row = row.child(div().flex_1().pl(px(12.0)).truncate().child(name));
            row = if on {
                row.child(
                    div()
                        .w(px(36.0))
                        .h(px(36.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icons::render(Icon::Remove, palette.on_primary)),
                )
            } else {
                row.child(
                    div()
                        .w(px(36.0))
                        .h(px(36.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icons::render(Icon::Plus, palette.border_strong)),
                )
            };
            model_list = model_list.child(row.on_click(cx.listener(move |this, _, _, _| {
                if this.session.active.contains(&index) {
                    this.session.active.remove(&index);
                } else {
                    this.session.active.insert(index);
                    this.session.eq_phone = Some(index);
                }
                this.session.rebuild_display_curves();
            })));
        }
        model_list
    }
}

impl Render for App {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = Palette::resolve_theme(self.session.theme);
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(palette.page)
            .text_color(palette.text_primary)
            .font_family(theme::FONT_FAMILY)
            .child(self.header(&palette))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h(px(0.0))
                    .child(self.left_column(&palette, cx))
                    .child(div().flex_1().h_full().bg(palette.page)),
            )
    }
}
