//! The shell: the window's frame, and the state the panels read and change.
//!
//! The arrangement is the reference's: a header across the top, a fixed
//! column down the left that holds the list and the panels under it, and a
//! column on the right that holds the graph and what belongs to it.

use egui::Ui;
use squiglab_core::session::{DataPaths, Session};

use crate::theme::{self, metrics};

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
    pub session: Session,
    /// The tab of the left column that is open.
    pub tab: Tab,
    /// The search in the list.
    pub search: String,
    /// The message shown at the foot of the window.
    pub status: Option<String>,
    /// The message shown in the window, over the graph.
    pub notice: Option<String>,
    /// Whether the graph's curves carry labels.
    pub show_labels: bool,
    /// Whether the pointer reads the curve under it.
    pub inspect: bool,
    /// The band the tone generator sweeps.
    pub tone_band: (f32, f32),
    /// The changes that can be undone, and the ones that can be redone.
    pub history: crate::equalizer::History,
    /// The brand whose measurements the list shows.
    pub brand: Option<String>,
    /// What is playing, and how loudly.
    pub player: Option<squiglab_core::audio::Player>,
    /// The source that is playing.
    pub source: squiglab_core::audio::Source,
    /// How loudly the source plays.
    pub volume: f32,
    /// The frequency the tone generator is playing.
    pub frequency: f32,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// The application, with the measurements loaded.
    pub fn new() -> Self {
        let mut session = Session::default();
        let paths = DataPaths {
            measurements: std::path::PathBuf::from("data"),
            headphones: std::path::PathBuf::from("data_hp"),
            corrections: std::path::PathBuf::from("data_delta"),
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
        // The preferences say what was on the graph when the window was last
        // closed. They are read after the load, because they name the curves
        // the load found.
        match squiglab_core::config::preferences_path() {
            Ok(path) => match squiglab_core::config::read_preferences_from(&path) {
                Ok(Some(preferences)) => {
                    if let Err(problem) = session.apply_preferences(preferences) {
                        status = Some(problem);
                    }
                }
                Ok(None) => {}
                Err(problem) => {
                    status = Some(format!("The preferences could not be read: {problem}"))
                }
            },
            Err(problem) => status = Some(format!("The preferences could not be found: {problem}")),
        }
        session.rebuild_display_curves();
        let theme = session.theme;
        Self {
            session,
            tab: Tab::Brands,
            search: String::new(),
            status,
            notice: None,
            show_labels: true,
            inspect: false,
            tone_band: (20.0, 20_000.0),
            history: crate::equalizer::History::default(),
            brand: None,
            player: None,
            source: squiglab_core::audio::Source::Tone { freq: 1000.0 },
            volume: 0.2,
            frequency: 1000.0,
        }
        .with_theme(theme)
    }

    /// Set the theme, and record it.
    fn with_theme(mut self, theme: squiglab_core::theme::Theme) -> Self {
        self.session.theme = theme;
        self
    }

    /// Say something at the foot of the window.
    pub fn say(&mut self, message: impl Into<String>) {
        self.status = Some(message.into());
    }

    /// The frame.
    pub fn draw(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        theme::apply(&ctx, self.session.theme);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::current_palette().page)
                    .inner_margin(egui::Margin::same(0)),
            )
            .show(ui, |ui| {
                self.header(ui);
                self.left_column(ui);
                self.right_column(ui);
            });

        if let Some(notice) = &self.notice {
            let area = ctx.content_rect();
            egui::Area::new(egui::Id::new("notice"))
                .fixed_pos(area.center() - egui::vec2(120.0, 20.0))
                .order(egui::Order::Foreground)
                .show(&ctx, |ui| {
                    egui::Frame::popup(ui.style())
                        .fill(theme::current_palette().field)
                        .corner_radius(egui::CornerRadius::same(metrics::RADIUS as u8))
                        .inner_margin(egui::Margin::same(12))
                        .show(ui, |ui| {
                            ui.label(notice);
                        });
                });
        }
    }

    /// The header across the top.
    fn header(&mut self, ui: &mut Ui) {
        egui::Panel::top("header")
            .exact_size(metrics::HEADER_HEIGHT)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::current_palette().page)
                    .inner_margin(egui::Margin::symmetric(metrics::PADDING as i8, 0)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    crate::header::show(ui, self);
                });
            });
    }

    /// The column down the left: the list, and the panels under it.
    fn left_column(&mut self, ui: &mut Ui) {
        egui::Panel::left("column")
            .exact_size(metrics::COLUMN_WIDTH)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::current_palette().page)
                    .inner_margin(egui::Margin::same(0)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("column_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        crate::selector::show(ui, self);
                        ui.separator();
                        match self.tab {
                            Tab::Equalizer => {
                                crate::equalizer::show(ui, self);
                                ui.separator();
                                crate::sound::show(ui, self);
                            }
                            _ => {
                                crate::equalizer::show_summary(ui, self);
                            }
                        }
                    });
            });
    }

    /// The column on the right: the tools, the graph, and what belongs to it.
    fn right_column(&mut self, ui: &mut Ui) {
        crate::tools::show(ui, self);
        crate::graph::show(ui, self);
        crate::curves::show(ui, self);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }
}

/// A panel that fills the width it is given.
pub fn full_width(ui: &mut Ui) -> f32 {
    ui.available_width()
}
