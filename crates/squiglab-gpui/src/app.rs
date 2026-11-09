//! The shell: the window's frame, and the state the column reads.
//!
//! This is the first milestone of the GPUI interface: the header, the tab row,
//! the search, and the list of brands and measurements. The graph, the
//! equalizer and the sound tools are still drawn by the egui interface.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use gpui::prelude::*;
use gpui::{
    div, Entity, img, px, AnyElement, Context, FontWeight, Image, ImageFormat, MouseButton, Render,
    Rgba, Window,
};
use squiglab_core::color::get_curve_color_with_offset;
use squiglab_core::equalizer::{Filter, FilterType};
use squiglab_core::product;
use squiglab_core::session::{
    matches_search, ChannelMode, CurveSource, DataPaths, EqBank, EqRow, Session, ZoomRegion,
    MAX_EQ_ROWS,
};

use crate::eq_input::EqInput;
use crate::graph;
use crate::graph::GraphData;
use crate::icons;
use crate::icons::Icon;
use crate::theme::{self, metrics, Palette};

/// Which field of a filter row an editable input is editing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
enum EqField {
    Freq,
    Gain,
    Q,
}

/// Which equalizer dropdown is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EqDropdown {
    Measurement,
    Target,
}

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

/// The changes that can be undone, and the ones that can be redone.
#[derive(Clone, Default)]
pub struct History {
    past: Vec<Snapshot>,
    future: Vec<Snapshot>,
}

/// One state of the equalizer, as the history keeps it.
#[derive(Clone, PartialEq)]
pub struct Snapshot {
    filters: std::collections::BTreeMap<usize, Vec<EqRow>>,
    constraints: squiglab_core::constraints::Constraints,
}

impl History {
    /// Record a state, so it can be come back to.
    #[allow(dead_code)]
    pub fn record(&mut self, snapshot: Snapshot) {
        self.past.push(snapshot);
        self.future.clear();
    }

    /// Whether there is anything to undo.
    #[allow(dead_code)]
    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    /// Whether there is anything to redo.
    #[allow(dead_code)]
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    /// Step back, returning the state to come back to.
    #[allow(dead_code)]
    pub fn undo(&mut self) -> Option<Snapshot> {
        let state = self.past.pop()?;
        self.future.push(state.clone());
        Some(state)
    }

    /// Step forward, returning the state to come back to.
    #[allow(dead_code)]
    pub fn redo(&mut self) -> Option<Snapshot> {
        let state = self.future.pop()?;
        self.past.push(state.clone());
        Some(state)
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
    /// The equalizer's change history.
    history: History,
    /// Whether the constraints panel is open.
    #[allow(dead_code)]
    constraints_open: bool,
    /// Which equalizer dropdown is open, if any.
    eq_dropdown: Option<EqDropdown>,
    /// Editable input entities for the filter table, keyed by
    /// (phone, row index within the edited bank, field). They persist across
    /// frames so that typing and focus survive re-renders.
    eq_inputs: BTreeMap<(usize, usize, EqField), Entity<EqInput>>,
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
            history: History::default(),
            constraints_open: false,
            eq_dropdown: None,
            eq_inputs: BTreeMap::new(),
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

    fn left_column(&mut self, palette: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
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

    fn list(&mut self, palette: &Palette, cx: &mut Context<Self>) -> AnyElement {
        match self.tab {
            Tab::Brands => self.brands(palette, cx).into_any_element(),
            Tab::Models => self.models(palette, cx).into_any_element(),
            Tab::Equalizer => self.equalizer_panel(palette, cx).into_any_element(),
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

    fn equalizer_panel(&mut self, palette: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let phone = self.session.eq_phone;
        let target = self.session.eq_target;
        let target_name = target
            .and_then(|idx| self.session.groups.get(idx))
            .map(|g| g.name.clone())
            .unwrap_or_else(|| "None selected".to_string());

        let measurements: Vec<usize> = self.session.active_measurements();
        let measurement_names: Vec<String> = measurements
            .iter()
            .map(|&index| self.session.groups[index].name.clone())
            .collect();
        let targets: Vec<usize> = self.session.active_targets.iter().copied().collect();

        let bank = self.session.eq_bank;
        // The rows of the edited bank, with their index into the full vec so
        // edits can be written back to the right place.
        let mut bank_rows: Vec<(usize, EqRow)> = self
            .session
            .eq_filters
            .get(&phone.unwrap_or(0))
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .filter(|(_, row)| row.bank == bank)
            .collect();

        let mut panel = div()
            .id("eq_panel")
            .flex()
            .flex_col()
            .flex_1()
            .w_full()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .gap(px(10.0))
            .p(px(8.0));

        // Header with title and Reset
        panel = panel.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .w_full()
                .child(
                    div()
                        .text_size(px(theme::type_scale::HEADING))
                        .font_weight(FontWeight::BOLD)
                        .text_color(palette.text_primary)
                        .child("Parametric Equalizer"),
                )
                .child(
                    div()
                        .id("eq_reset")
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(4.0))
                        .cursor_pointer()
                        .text_size(px(theme::type_scale::LABEL))
                        .text_color(palette.accent)
                        .child(icons::render(Icon::Reset, palette.accent))
                        .child("Reset")
                        .on_click(cx.listener(move |this, _, _, _| {
                            if let Some(phone) = this.session.eq_phone {
                                let before = this.snapshot();
                                this.session.eq_filters.insert(phone, Vec::new());
                                this.history.record(before);
                                this.session.rebuild_display_curves();
                            }
                        })),
                ),
        );

        // Measurement and target selectors + Auto EQ
        panel = panel.child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .p(px(8.0))
                .rounded(px(metrics::RADIUS))
                .bg(palette.field)
                .border_1()
                .border_color(palette.border)
                .child(self.eq_measurement_selector(palette, cx, &measurements, &measurement_names))
                .child(self.eq_target_row(palette, cx, &targets, &target_name)),
        );

        // 2-channel bank selector (only when constraints allow it)
        if self.session.eq_constraints.two_channel {
            panel = panel.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .p(px(8.0))
                    .rounded(px(metrics::RADIUS))
                    .bg(palette.field)
                    .border_1()
                    .border_color(palette.border)
                    .child(
                        div()
                            .text_size(px(theme::type_scale::LABEL))
                            .text_color(palette.text_tertiary)
                            .child("Bank"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(4.0))
                            .child(
                                self.eq_bank_button(
                                    palette,
                                    cx,
                                    EqBank::Left,
                                    bank == EqBank::Left,
                                    "L",
                                ),
                            )
                            .child(
                                self.eq_bank_button(
                                    palette,
                                    cx,
                                    EqBank::Both,
                                    bank == EqBank::Both,
                                    "L + R",
                                ),
                            )
                            .child(
                                self.eq_bank_button(
                                    palette,
                                    cx,
                                    EqBank::Right,
                                    bank == EqBank::Right,
                                    "R",
                                ),
                            ),
                    ),
            );
        }

        // Filter table header
        panel = panel.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .text_size(px(theme::type_scale::LABEL))
                .text_color(palette.text_tertiary)
                .px(px(4.0))
                .child(div().w(px(20.0)).child("✓"))
                .child(div().w(px(52.0)).child("Type"))
                .child(div().flex_1().child("Frequency"))
                .child(div().flex_1().child("Gain"))
                .child(div().flex_1().child("Q")),
        );

        // Filter rows with editable inputs.
        let phone = phone.unwrap_or(0);
        let mut row_list = div().flex().flex_col().gap(px(4.0));
        for (list_i, (full_i, row)) in bank_rows.iter_mut().enumerate() {
            let full_i = *full_i;
            let filter = row.filter;
            let enabled = row.enabled;
            let type_str = match filter.filter_type {
                FilterType::Peaking => "PK",
                FilterType::LowShelf => "LSQ",
                FilterType::HighShelf => "HSQ",
            };

            // Ensure an input entity exists for each field, initialised from
            // the current filter value.
            let freq_input = self.eq_input_entity(
                cx,
                (phone, list_i, EqField::Freq),
                filter.freq as f64,
                0,
                " Hz",
                Some(20.0),
                Some(20_000.0),
            );
            let gain_input = self.eq_input_entity(
                cx,
                (phone, list_i, EqField::Gain),
                filter.gain as f64,
                1,
                " dB",
                Some(-40.0),
                Some(40.0),
            );
            let q_input = self.eq_input_entity(
                cx,
                (phone, list_i, EqField::Q),
                filter.q as f64,
                2,
                "",
                Some(0.0),
                Some(20.0),
            );

            row_list = row_list.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .p(px(4.0))
                    .rounded(px(metrics::RADIUS))
                    .bg(if enabled {
                        palette.field
                    } else {
                        palette.sunken
                    })
                    .text_size(px(theme::type_scale::BODY))
                    .border_1()
                    .border_color(palette.border)
                    .child(
                        div()
                            .id(("eq_row_toggle", list_i))
                            .w(px(20.0))
                            .h(px(20.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_color(if enabled {
                                palette.accent
                            } else {
                                palette.text_disabled
                            })
                            .child(if enabled { "●" } else { "○" })
                            .on_click(cx.listener(move |this, _, _, _| {
                                let before = this.snapshot();
                                if let Some(filters) = this.session.eq_filters.get_mut(&phone) {
                                    if let Some(r) = filters.get_mut(full_i) {
                                        r.enabled = !r.enabled;
                                        this.history.record(before);
                                        this.session.rebuild_display_curves();
                                    }
                                }
                            })),
                    )
                    .child(
                        div()
                            .id(("eq_row_type", list_i))
                            .w(px(52.0))
                            .cursor_pointer()
                            .font_weight(FontWeight::BOLD)
                            .text_color(palette.text_primary)
                            .child(type_str)
                            .on_click(cx.listener(move |this, _, _, _| {
                                let before = this.snapshot();
                                if let Some(filters) = this.session.eq_filters.get_mut(&phone) {
                                    if let Some(r) = filters.get_mut(full_i) {
                                        r.filter.filter_type = match r.filter.filter_type {
                                            FilterType::Peaking => FilterType::LowShelf,
                                            FilterType::LowShelf => FilterType::HighShelf,
                                            FilterType::HighShelf => FilterType::Peaking,
                                        };
                                        this.history.record(before);
                                        this.session.rebuild_display_curves();
                                    }
                                }
                            })),
                    )
                    // Frequency input
                    .child(div().flex_1().child(freq_input))
                    // Gain input
                    .child(div().flex_1().child(gain_input))
                    // Q input
                    .child(div().flex_1().child(q_input)),
            );
        }
        panel = panel.child(row_list);

        // Sync committed input values back into the session's filters.
        self.sync_eq_inputs(cx, phone, &bank_rows);

        // Controls row: + Add, − Remove, Sort
        panel = panel.child(
            div()
                .flex()
                .flex_row()
                .gap(px(8.0))
                .mt(px(4.0))
                .child(
                    div()
                        .id("eq_add_filter")
                        .px(px(10.0))
                        .py(px(4.0))
                        .rounded(px(metrics::RADIUS))
                        .bg(palette.field)
                        .border_1()
                        .border_color(palette.border_strong)
                        .text_size(px(theme::type_scale::CONTROL))
                        .cursor_pointer()
                        .child("+ Add Filter")
                        .on_click(cx.listener(move |this, _, _, _| {
                            let Some(phone) = this.session.eq_phone else {
                                return;
                            };
                            let before = this.snapshot();
                            let entry = this.session.eq_filters.entry(phone).or_default();
                            if entry.len() < MAX_EQ_ROWS {
                                entry.push(EqRow {
                                    filter: Filter::new(FilterType::Peaking, 1000.0, 1.4, 0.0),
                                    enabled: true,
                                    bank: this.session.eq_bank,
                                });
                                this.history.record(before);
                                this.session.rebuild_display_curves();
                            }
                        })),
                )
                .child(
                    div()
                        .id("eq_remove_filter")
                        .px(px(10.0))
                        .py(px(4.0))
                        .rounded(px(metrics::RADIUS))
                        .bg(palette.field)
                        .border_1()
                        .border_color(palette.border_strong)
                        .text_size(px(theme::type_scale::CONTROL))
                        .cursor_pointer()
                        .child("− Remove")
                        .on_click(cx.listener(move |this, _, _, _| {
                            let Some(phone) = this.session.eq_phone else {
                                return;
                            };
                            let before = this.snapshot();
                            if let Some(entry) = this.session.eq_filters.get_mut(&phone) {
                                entry.retain(|row| row.bank == this.session.eq_bank);
                                while entry.len() > 1
                                    && entry.last().is_some_and(|row| {
                                        row.filter.gain == 0.0
                                            && row.filter.freq == 0.0
                                            && row.filter.q == 0.0
                                    })
                                {
                                    entry.pop();
                                }
                                if entry.len() > 1 {
                                    entry.pop();
                                }
                                this.history.record(before);
                                this.session.rebuild_display_curves();
                            }
                        })),
                )
                .child(
                    div()
                        .id("eq_sort")
                        .px(px(10.0))
                        .py(px(4.0))
                        .rounded(px(metrics::RADIUS))
                        .bg(palette.field)
                        .border_1()
                        .border_color(palette.border_strong)
                        .text_size(px(theme::type_scale::CONTROL))
                        .cursor_pointer()
                        .child("Sort")
                        .on_click(cx.listener(move |this, _, _, _| {
                            let Some(phone) = this.session.eq_phone else {
                                return;
                            };
                            let before = this.snapshot();
                            if let Some(entry) = this.session.eq_filters.get_mut(&phone) {
                                entry.retain(|row| row.bank == this.session.eq_bank);
                                entry.sort_by(|a, b| a.filter.freq.total_cmp(&b.filter.freq));
                                this.history.record(before);
                                this.session.rebuild_display_curves();
                            }
                        })),
                ),
        );

        panel
    }

    /// A snapshot of the equalizer state, for the undo history.
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            filters: self.session.eq_filters.clone(),
            constraints: self.session.eq_constraints.clone(),
        }
    }

    /// Get or create the editable input entity for a filter field.
    fn eq_input_entity(
        &mut self,
        cx: &mut Context<Self>,
        key: (usize, usize, EqField),
        value: f64,
        decimals: usize,
        unit: &'static str,
        min: Option<f64>,
        max: Option<f64>,
    ) -> impl IntoElement {
        if !self.eq_inputs.contains_key(&key) {
            let entity = cx.new(|eq_cx| {
                EqInput::new(
                    eq_cx.focus_handle(),
                    value,
                    decimals,
                    unit,
                    min,
                    max,
                    |_| {},
                )
            });
            self.eq_inputs.insert(key, entity);
        }
        self.eq_inputs.get(&key).unwrap().clone()
    }

    /// Sync committed input values back into the session's filters.
    fn sync_eq_inputs(
        &mut self,
        cx: &mut Context<Self>,
        phone: usize,
        bank_rows: &[(usize, EqRow)],
    ) {
        let filters = self.session.eq_filters.get_mut(&phone);
        let Some(filters) = filters else {
            return;
        };
        for (list_i, (full_i, _row)) in bank_rows.iter().enumerate() {
            for field in [EqField::Freq, EqField::Gain, EqField::Q] {
                let key = (phone, list_i, field);
                let Some(entity) = self.eq_inputs.get(&key) else {
                    continue;
                };
                let value = entity.read(&*cx).value();
                let filter = filters.get_mut(*full_i);
                let Some(filter) = filter else {
                    continue;
                };
                match field {
                    EqField::Freq => filter.filter.freq = value,
                    EqField::Gain => filter.filter.gain = value,
                    EqField::Q => filter.filter.q = value,
                }
            }
        }
    }

    /// The measurement selector: current choice as a button, with the list of
    /// active measurements shown below when open.
    fn eq_measurement_selector(
        &mut self,
        palette: &Palette,
        cx: &mut Context<Self>,
        measurements: &[usize],
        names: &[String],
    ) -> impl IntoElement {
        let current = self.session.eq_phone;
        let current_label = current
            .and_then(|idx| self.session.groups.get(idx))
            .map(|g| format!("{} {}", g.brand, g.name))
            .unwrap_or_else(|| "Choose measurement".to_string());
        let open = matches!(self.eq_dropdown, Some(EqDropdown::Measurement));

        let mut row = div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(
                div()
                    .id("eq_measurement_btn")
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px(px(8.0))
                    .py(px(3.0))
                    .rounded(px(metrics::RADIUS))
                    .bg(if open {
                        palette.accent
                    } else {
                        palette.field
                    })
                    .border_1()
                    .border_color(if open {
                        palette.accent
                    } else {
                        palette.border_strong
                    })
                    .text_color(if open {
                        palette.on_accent
                    } else {
                        palette.text_primary
                    })
                    .text_size(px(theme::type_scale::CONTROL))
                    .cursor_pointer()
                    .child(div().flex_1().truncate().child(current_label.clone()))
                    .child(if open { "▲" } else { "▼" })
                    .on_mouse_up(MouseButton::Left, cx.listener(move |this, _, _, _| {
                        this.eq_dropdown = if matches!(this.eq_dropdown, Some(EqDropdown::Measurement)) {
                            None
                        } else {
                            Some(EqDropdown::Measurement)
                        };
                    })),
            );

        if open {
            let mut list = div()
                .absolute()
                .mt(px(2.0))
                .w_full()
                .bg(palette.field)
                .border_1()
                .border_color(palette.border_strong)
                .rounded(px(metrics::RADIUS))
                .flex()
                .flex_col();
            for (i, &index) in measurements.iter().enumerate() {
                let selected = current == Some(index);
                let name = names[i].clone();
                list = list.child(
                    div()
                        .px(px(8.0))
                        .py(px(3.0))
                        .text_size(px(theme::type_scale::CONTROL))
                        .text_color(if selected {
                            palette.on_accent
                        } else {
                            palette.text_primary
                        })
                        .bg(if selected {
                            palette.accent
                        } else {
                            palette.field
                        })
                        .cursor_pointer()
                        .child(name)
                        .on_mouse_up(MouseButton::Left, cx.listener(move |this, _, _, _| {
                            this.session.eq_phone = Some(index);
                            this.session.rebuild_display_curves();
                            this.eq_dropdown = None;
                        })),
                );
            }
            row = row.child(list);
        }

        row
    }

    /// The target selector row: target choice plus the Auto EQ button.
    fn eq_target_row(
        &mut self,
        palette: &Palette,
        cx: &mut Context<Self>,
        targets: &[usize],
        target_name: &str,
    ) -> impl IntoElement {
        let current = self.session.eq_target;
        let open = matches!(self.eq_dropdown, Some(EqDropdown::Target));

        let mut row = div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px(px(8.0))
                    .py(px(3.0))
                    .rounded(px(metrics::RADIUS))
                    .bg(if open {
                        palette.accent
                    } else {
                        palette.field
                    })
                    .border_1()
                    .border_color(if open {
                        palette.accent
                    } else {
                        palette.border_strong
                    })
                    .text_color(if open {
                        palette.on_accent
                    } else {
                        palette.text_primary
                    })
                    .text_size(px(theme::type_scale::CONTROL))
                    .cursor_pointer()
                    .child(div().flex_1().truncate().child(target_name.to_string()))
                    .child(if open { "▲" } else { "▼" })
                    .on_mouse_up(MouseButton::Left, cx.listener(move |this, _, _, _| {
                        this.eq_dropdown = if matches!(this.eq_dropdown, Some(EqDropdown::Target)) {
                            None
                        } else {
                            Some(EqDropdown::Target)
                        };
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(6.0))
                    .mt(px(4.0))
                    .child(
                        div()
                            .id("auto_eq_btn")
                            .px(px(12.0))
                            .py(px(4.0))
                            .rounded(px(metrics::RADIUS))
                            .bg(palette.accent)
                            .text_color(palette.on_accent)
                            .text_size(px(theme::type_scale::CONTROL))
                            .font_weight(FontWeight::BOLD)
                            .cursor_pointer()
                            .child("Auto EQ")
                            .on_click(cx.listener(move |this, _, _, _| {
                                let before = this.snapshot();
                                let _ = this.session.run_auto_eq();
                                this.history.record(before);
                                this.session.rebuild_display_curves();
                            })),
                    ),
            );

        if open {
            let mut list = div()
                .absolute()
                .mt(px(2.0))
                .w_full()
                .bg(palette.field)
                .border_1()
                .border_color(palette.border_strong)
                .rounded(px(metrics::RADIUS))
                .flex()
                .flex_col();
            for &index in targets {
                let selected = current == Some(index);
                let name = self.session.groups[index].name.clone();
                list = list.child(
                    div()
                        .px(px(8.0))
                        .py(px(3.0))
                        .text_size(px(theme::type_scale::CONTROL))
                        .text_color(if selected {
                            palette.on_accent
                        } else {
                            palette.text_primary
                        })
                        .bg(if selected {
                            palette.accent
                        } else {
                            palette.field
                        })
                        .cursor_pointer()
                        .child(name)
                        .on_mouse_up(MouseButton::Left, cx.listener(move |this, _, _, _| {
                            this.session.eq_target = Some(index);
                            this.eq_dropdown = None;
                        })),
                );
            }
            row = row.child(list);
        }

        row
    }

    /// One bank toggle button (L / L + R / R).
    fn eq_bank_button(
        &self,
        palette: &Palette,
        cx: &mut Context<Self>,
        bank: EqBank,
        selected: bool,
        label: &'static str,
    ) -> impl IntoElement {
        div()
            .px(px(8.0))
            .py(px(3.0))
            .rounded(px(metrics::RADIUS))
            .bg(if selected {
                palette.accent
            } else {
                palette.field
            })
            .text_color(if selected {
                palette.on_accent
            } else {
                palette.text_primary
            })
            .border_1()
            .border_color(if selected {
                palette.accent
            } else {
                palette.border_strong
            })
            .text_size(px(theme::type_scale::CONTROL))
            .cursor_pointer()
            .child(label)
            .on_mouse_up(MouseButton::Left, cx.listener(move |this, _, _, _| {
                this.session.eq_bank = bank;
            }))
    }

    fn tools_row(&self, palette: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let zoom = self.session.zoom;
        let smooth = self.session.smooth_level;
        let phon = self.session.phon_normalization;
        let has_baseline = self.session.baseline.is_some();

        div()
            .flex()
            .flex_row()
            .items_center()
            .flex_wrap()
            .gap(px(8.0))
            .px(px(12.0))
            .py(px(6.0))
            .bg(palette.page)
            .border_b_1()
            .border_color(palette.border)
            // Zoom Cluster
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(theme::type_scale::LABEL))
                            .font_weight(FontWeight::BOLD)
                            .text_color(palette.text_tertiary)
                            .child("ZOOM:"),
                    )
                    .child(self.tool_button(
                        "zoom_full",
                        "Full",
                        zoom == ZoomRegion::Full,
                        palette,
                        cx,
                        |this| {
                            this.session.zoom = ZoomRegion::Full;
                        },
                    ))
                    .child(self.tool_button(
                        "zoom_bass",
                        "Bass",
                        zoom == ZoomRegion::Bass,
                        palette,
                        cx,
                        |this| {
                            this.session.zoom = ZoomRegion::Bass;
                        },
                    ))
                    .child(self.tool_button(
                        "zoom_mids",
                        "Mids",
                        zoom == ZoomRegion::Mids,
                        palette,
                        cx,
                        |this| {
                            this.session.zoom = ZoomRegion::Mids;
                        },
                    ))
                    .child(self.tool_button(
                        "zoom_treble",
                        "Treble",
                        zoom == ZoomRegion::Treble,
                        palette,
                        cx,
                        |this| {
                            this.session.zoom = ZoomRegion::Treble;
                        },
                    )),
            )
            .child(div().w(px(1.0)).h(px(16.0)).bg(palette.border_strong))
            // Normalize Cluster
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(theme::type_scale::LABEL))
                            .font_weight(FontWeight::BOLD)
                            .text_color(palette.text_tertiary)
                            .child("NORMALIZE:"),
                    )
                    .child(
                        self.tool_button("norm_phon", "dB (Phon)", phon, palette, cx, |this| {
                            this.session.phon_normalization = true;
                        }),
                    )
                    .child(
                        self.tool_button("norm_hz", "Hz (Freq)", !phon, palette, cx, |this| {
                            this.session.phon_normalization = false;
                        }),
                    ),
            )
            .child(div().w(px(1.0)).h(px(16.0)).bg(palette.border_strong))
            // Smooth Cluster
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(theme::type_scale::LABEL))
                            .font_weight(FontWeight::BOLD)
                            .text_color(palette.text_tertiary)
                            .child("SMOOTH:"),
                    )
                    .child(
                        self.tool_button("smooth_0", "RAW", smooth == 0, palette, cx, |this| {
                            this.session.smooth_level = 0;
                        }),
                    )
                    .child(
                        self.tool_button("smooth_5", "5", smooth == 5, palette, cx, |this| {
                            this.session.smooth_level = 5;
                        }),
                    )
                    .child(self.tool_button(
                        "smooth_10",
                        "10",
                        smooth == 10,
                        palette,
                        cx,
                        |this| {
                            this.session.smooth_level = 10;
                        },
                    )),
            )
            .child(div().w(px(1.0)).h(px(16.0)).bg(palette.border_strong))
            // Recolor & Baseline
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .id("recolor_btn")
                            .px(px(8.0))
                            .py(px(3.0))
                            .rounded(px(metrics::RADIUS))
                            .bg(palette.field)
                            .border_1()
                            .border_color(palette.border_strong)
                            .text_size(px(theme::type_scale::CONTROL))
                            .text_color(palette.text_primary)
                            .cursor_pointer()
                            .child("Recolor")
                            .on_click(cx.listener(|this, _, _, _| {
                                this.session.color_offset = (this.session.color_offset + 1) % 10;
                                this.session.rebuild_display_curves();
                            })),
                    )
                    .child(
                        div()
                            .id("baseline_btn")
                            .px(px(8.0))
                            .py(px(3.0))
                            .rounded(px(metrics::RADIUS))
                            .bg(if has_baseline {
                                palette.accent
                            } else {
                                palette.field
                            })
                            .border_1()
                            .border_color(palette.border_strong)
                            .text_size(px(theme::type_scale::CONTROL))
                            .text_color(if has_baseline {
                                palette.on_accent
                            } else {
                                palette.text_primary
                            })
                            .cursor_pointer()
                            .child("Clear Baseline")
                            .on_click(cx.listener(|this, _, _, _| {
                                this.session.baseline = None;
                                this.session.rebuild_display_curves();
                            })),
                    ),
            )
    }

    fn tool_button<F>(
        &self,
        id_str: &'static str,
        label: &'static str,
        active: bool,
        palette: &Palette,
        cx: &mut Context<Self>,
        on_click: F,
    ) -> impl IntoElement
    where
        F: Fn(&mut Self) + 'static + Send + Sync,
    {
        div()
            .id(id_str)
            .px(px(8.0))
            .py(px(3.0))
            .rounded(px(metrics::RADIUS))
            .bg(if active {
                palette.accent
            } else {
                palette.field
            })
            .text_color(if active {
                palette.on_accent
            } else {
                palette.text_primary
            })
            .border_1()
            .border_color(if active {
                palette.accent
            } else {
                palette.border_strong
            })
            .text_size(px(theme::type_scale::CONTROL))
            .cursor_pointer()
            .child(label)
            .on_click(cx.listener(move |this, _, _, _| {
                on_click(this);
                this.session.rebuild_display_curves();
            }))
    }

    fn target_strip(&self, palette: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let mut row = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap(px(6.0))
            .px(px(12.0))
            .py(px(4.0))
            .w_full();

        row = row.child(
            div()
                .text_size(px(theme::type_scale::LABEL))
                .font_weight(FontWeight::BOLD)
                .text_color(palette.text_tertiary)
                .child("TARGETS:"),
        );

        for (index, group) in self.session.groups.iter().enumerate() {
            if !group.target {
                continue;
            }
            let active = self.session.active_targets.contains(&index);
            let label = if group.name.contains("Delta") || group.name.contains("delta") {
                format!("Δ {}", group.name)
            } else {
                group.name.clone()
            };

            row = row.child(
                div()
                    .id(("target_chip", index))
                    .px(px(8.0))
                    .py(px(3.0))
                    .rounded(px(metrics::RADIUS))
                    .bg(if active {
                        palette.text_primary
                    } else {
                        palette.field
                    })
                    .text_color(if active {
                        palette.on_primary
                    } else {
                        palette.text_primary
                    })
                    .border_1()
                    .border_color(if active {
                        palette.text_primary
                    } else {
                        palette.border_strong
                    })
                    .text_size(px(theme::type_scale::CONTROL))
                    .cursor_pointer()
                    .child(label)
                    .on_click(cx.listener(move |this, _, _, _| {
                        if this.session.active_targets.contains(&index) {
                            this.session.active_targets.remove(&index);
                        } else {
                            this.session.active_targets.insert(index);
                        }
                        this.session.rebuild_display_curves();
                    })),
            );
        }
        row
    }

    fn manager_rows(&self, palette: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let mut list = div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .px(px(12.0))
            .py(px(4.0))
            .w_full();

        for &index in &self.session.active {
            let Some(group) = self.session.groups.get(index) else {
                continue;
            };

            let position = self
                .session
                .display_curves
                .iter()
                .position(|c| {
                    c.group_index == index && matches!(c.source, CurveSource::Measurement)
                })
                .unwrap_or(index);
            let c = get_curve_color_with_offset(position, self.session.color_offset);
            let color = gpui::rgb(((c.r as u32) << 16) | ((c.g as u32) << 8) | (c.b as u32));

            let channel_mode = self
                .session
                .channel_modes
                .get(&index)
                .copied()
                .unwrap_or(ChannelMode::Both);
            let offset = self.session.offsets.get(&index).copied().unwrap_or(0.0);
            let is_baseline = self.session.baseline == Some(index);

            let row = div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .px(px(8.0))
                .py(px(4.0))
                .rounded(px(metrics::RADIUS))
                .bg(palette.field)
                .border_1()
                .border_color(palette.border)
                .text_size(px(theme::type_scale::BODY))
                // Color swatch + Name
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .flex_1()
                        .truncate()
                        .child(div().w(px(12.0)).h(px(12.0)).rounded(px(2.0)).bg(color))
                        .child(
                            div()
                                .truncate()
                                .font_weight(FontWeight::MEDIUM)
                                .child(format!("{} {}", group.brand, group.name)),
                        ),
                )
                // Controls cluster: Channel, Offset, Baseline, Remove
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        // Channel Mode
                        .child(
                            div()
                                .id(("mgr_chan", index))
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(metrics::RADIUS))
                                .border_1()
                                .border_color(palette.border_strong)
                                .text_size(px(theme::type_scale::LABEL))
                                .cursor_pointer()
                                .child(channel_mode.label())
                                .on_click(cx.listener(move |this, _, _, _| {
                                    let next_mode = match channel_mode {
                                        ChannelMode::Both => ChannelMode::Left,
                                        ChannelMode::Left => ChannelMode::Right,
                                        ChannelMode::Right => ChannelMode::Both,
                                        ChannelMode::Average => ChannelMode::Both,
                                    };
                                    this.session.channel_modes.insert(index, next_mode);
                                    this.session.rebuild_display_curves();
                                })),
                        )
                        // Offset: - / Value / +
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .id(("mgr_off_minus", index))
                                        .px(px(4.0))
                                        .cursor_pointer()
                                        .child("−")
                                        .on_click(cx.listener(move |this, _, _, _| {
                                            let cur = this
                                                .session
                                                .offsets
                                                .get(&index)
                                                .copied()
                                                .unwrap_or(0.0);
                                            this.session.offsets.insert(index, cur - 1.0);
                                            this.session.rebuild_display_curves();
                                        })),
                                )
                                .child(
                                    div()
                                        .min_w(px(38.0))
                                        .text_size(px(theme::type_scale::LABEL))
                                        .child(format!("{:+1.0} dB", offset)),
                                )
                                .child(
                                    div()
                                        .id(("mgr_off_plus", index))
                                        .px(px(4.0))
                                        .cursor_pointer()
                                        .child("+")
                                        .on_click(cx.listener(move |this, _, _, _| {
                                            let cur = this
                                                .session
                                                .offsets
                                                .get(&index)
                                                .copied()
                                                .unwrap_or(0.0);
                                            this.session.offsets.insert(index, cur + 1.0);
                                            this.session.rebuild_display_curves();
                                        })),
                                ),
                        )
                        // Baseline
                        .child(
                            div()
                                .id(("mgr_base", index))
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(metrics::RADIUS))
                                .bg(if is_baseline {
                                    palette.accent
                                } else {
                                    palette.field
                                })
                                .text_color(if is_baseline {
                                    palette.on_accent
                                } else {
                                    palette.text_primary
                                })
                                .border_1()
                                .border_color(palette.border_strong)
                                .text_size(px(theme::type_scale::LABEL))
                                .cursor_pointer()
                                .child("Base")
                                .on_click(cx.listener(move |this, _, _, _| {
                                    if this.session.baseline == Some(index) {
                                        this.session.baseline = None;
                                    } else {
                                        this.session.baseline = Some(index);
                                    }
                                    this.session.rebuild_display_curves();
                                })),
                        )
                        // Remove
                        .child(
                            div()
                                .id(("mgr_remove", index))
                                .w(px(24.0))
                                .h(px(24.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .child(icons::render(Icon::Remove, palette.text_quiet))
                                .on_click(cx.listener(move |this, _, _, _| {
                                    this.session.active.remove(&index);
                                    this.session.rebuild_display_curves();
                                })),
                        ),
                );

            list = list.child(row);
        }

        list
    }

    fn bottom_strip(&self, palette: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("bottom_strip")
            .flex()
            .flex_col()
            .w_full()
            .max_h(px(240.0))
            .overflow_y_scroll()
            .border_t_1()
            .border_color(palette.border)
            .py(px(4.0))
            .child(self.target_strip(palette, cx))
            .child(self.manager_rows(palette, cx))
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
                    .child({
                        let data = GraphData::from_session(
                            &self.session,
                            palette.border_strong,
                            palette.text_primary,
                        );
                        div()
                            .flex_1()
                            .h_full()
                            .flex()
                            .flex_col()
                            .min_w(px(0.0))
                            .bg(palette.page)
                            .child(self.tools_row(&palette, cx))
                            .child(
                                div().flex_1().min_h(px(200.0)).w_full().child(
                                    gpui::canvas(
                                        move |bounds, _window, _cx| (bounds, data),
                                        |_bounds, state, window, _cx| {
                                            let (bounds, data) = state;
                                            graph::show(&data, bounds, window);
                                        },
                                    )
                                    .size_full(),
                                ),
                            )
                            .child(self.bottom_strip(&palette, cx))
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_initializes_with_models_tab() {
        let app = App::new();
        assert_eq!(app.tab, Tab::Models);
    }

    #[test]
    fn tab_labels_match_reference() {
        assert_eq!(Tab::Brands.label(), "Brands");
        assert_eq!(Tab::Models.label(), "Models");
        assert_eq!(Tab::Equalizer.label(), "Equalizer");
    }
}
