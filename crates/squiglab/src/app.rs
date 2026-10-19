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
    /// What the sound tools last said.
    pub sound_message: Option<String>,
    /// Whether the sound is played through the equalizer.
    pub compare: bool,
    /// Where the pointer took hold of the graph's scale, so the drag can tell
    /// the mark from the circles.
    pub scale_grab: Option<egui::Pos2>,
    /// The music player: the file, the search, and what it found.
    pub music: crate::music::Music,
    /// The search that is running, if one is.
    pub pending_search: Option<crate::music::SearchHandle>,
    /// The preview that is downloading, if one is.
    pub pending_preview: Option<(crate::music::Track, crate::music::PreviewHandle)>,
    /// Whether a picture of the window was asked for.
    pub screenshot_requested: bool,
    /// Whether the constraints panel is open.
    pub constraints_open: bool,
    /// The file the upload panel is waiting to load.
    pub upload_path: String,
    /// The file the equalizer's import is waiting to load.
    pub import_path: String,
    /// What the upload panel last said.
    pub upload_message: Option<String>,
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
        // The equalizer works on one measurement and against one target: the
        // ones that are on the graph, if there are any.
        if session.eq_phone.is_none() {
            session.eq_phone = session.active.iter().next().copied();
        }
        if session.eq_target.is_none() {
            session.eq_target = session.active_targets.iter().next().copied();
        }
        session.rebuild_display_curves();
        let theme = session.theme;
        Self {
            session,
            tab: Tab::Brands,
            search: String::new(),
            status,
            tone_band: (20.0, 20_000.0),
            history: crate::equalizer::History::default(),
            brand: None,
            player: None,
            source: squiglab_core::audio::Source::Tone { freq: 1000.0 },
            volume: 0.2,
            frequency: 1000.0,
            sound_message: None,
            compare: false,
            scale_grab: None,
            music: crate::music::Music::default(),
            pending_search: None,
            pending_preview: None,
            screenshot_requested: false,
            constraints_open: false,
            upload_path: String::new(),
            import_path: String::new(),
            upload_message: None,
        }
        .with_theme(theme)
    }

    /// Set the theme, and record it.
    fn with_theme(mut self, theme: squiglab_core::theme::Theme) -> Self {
        self.session.theme = theme;
        self
    }

    /// Keep the view, so the window opens on it next time.
    pub fn save_preferences(&mut self) {
        let path = match squiglab_core::config::preferences_path() {
            Ok(path) => path,
            Err(_) => return,
        };
        if let Err(problem) =
            squiglab_core::config::write_preferences_to(&path, &self.session.preferences())
        {
            self.status = Some(format!("The view could not be saved: {problem}"));
        }
    }

    /// Say something at the foot of the window.
    pub fn say(&mut self, message: impl Into<String>) {
        self.status = Some(message.into());
    }

    /// The frame.
    pub fn draw(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        theme::apply(&ctx, self.session.theme);
        // What the music player was doing while the window was busy.
        crate::music::take_results(self);
        crate::music::take_preview(self);

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

        if self.screenshot_requested {
            self.screenshot_requested = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        // The picture comes back as an event, a frame later.
        let mut picture: Option<egui::ColorImage> = None;
        ctx.input(|input| {
            for event in &input.raw.events {
                if let egui::Event::Screenshot { image, .. } = event {
                    picture = Some((**image).clone());
                }
            }
        });
        if let Some(image) = picture {
            match crate::tools::save_picture("window.png", &image) {
                Ok(path) => {
                    self.status = Some(format!("The picture was saved to {}", path.display()))
                }
                Err(problem) => self.status = Some(format!("The picture was not saved: {problem}")),
            }
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
                        ui.separator();
                        crate::upload::show(ui, self);
                    });
            });
    }

    /// The column on the right: the tools, the graph, and what belongs to it.
    fn right_column(&mut self, ui: &mut Ui) {
        crate::tools::show(ui, self);
        crate::graph::show(ui, self);
        crate::curves::show(ui, self);
        self.status_line(ui);
    }

    /// What the program last said, at the foot of the window.
    fn status_line(&mut self, ui: &mut Ui) {
        let Some(message) = self.status.clone() else {
            return;
        };
        let palette = theme::current_palette();
        ui.add_space(metrics::GAP);
        ui.label(
            egui::RichText::new(message)
                .font(theme::TypeStep::Label.font())
                .color(palette.text_secondary),
        );
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }

    /// The view is kept between sessions, so the window opens on what was on
    /// the graph when it was last closed.
    fn on_exit(&mut self) {
        self.save_preferences();
    }
}

/// A panel that fills the width it is given.
pub fn full_width(ui: &mut Ui) -> f32 {
    ui.available_width()
}
