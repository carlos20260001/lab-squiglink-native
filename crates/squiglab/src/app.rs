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

/// What a file the user has chosen is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    /// A measurement to upload, as a measurement or as a target.
    Upload,
    /// An equalizer file to import.
    ImportParametric,
    /// A music file to play.
    Music,
}

/// The kind of file the device is asked for, as the system's own tables
/// name it.
#[cfg(target_os = "android")]
fn mime_for(extensions: &[&str]) -> &'static str {
    if extensions
        .iter()
        .any(|one| matches!(*one, "mp3" | "flac" | "wav" | "ogg" | "m4a"))
    {
        "audio/%"
    } else if extensions
        .iter()
        .any(|one| matches!(*one, "txt" | "csv" | "fr" | "cfg" | "apo"))
    {
        "text/%"
    } else {
        "%"
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
    /// Whether the sound tools' settings are open, behind their gear.
    pub sound_settings_open: bool,
    /// The files the device holds, when the phone is the one being asked.
    #[cfg(target_os = "android")]
    pub android_files: Option<(Pick, String, Vec<crate::filepick::android::File>)>,
    /// What the pointer is doing on the graph.
    pub graph_drag: Option<crate::graph::GraphDrag>,
    /// The file the user is being asked for, and the dialog that is asking.
    pub picker: Option<(Pick, crate::filepick::Picker)>,
    /// The measurement the user chose with the system's dialog, waiting to be
    /// loaded as a measurement or as a target.
    pub pending_upload: Option<crate::filepick::Chosen>,
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
            tab: Tab::Models,
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
            sound_settings_open: false,
            #[cfg(target_os = "android")]
            android_files: None,
            graph_drag: None,
            picker: None,
            pending_upload: None,
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
        // What the file picker has come back with.
        self.take_chosen_file();
        // The phone's own list of files.
        #[cfg(target_os = "android")]
        self.show_android_files(&ctx);

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
                    .fill(theme::current_palette().header)
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
                    // The reference's column carries sixteen points of
                    // padding down its sides, so the tabs and the panel
                    // beneath them sit in from the edge.
                    .inner_margin(egui::Margin::symmetric(metrics::COLUMN_PADDING as i8, 0)),
            )
            .show(ui, |ui| {
                // The tabs stay put and the panel beneath them scrolls, which
                // is how the reference's equalizer behaves: the extra panel
                // has its own scroll and the tabs above it do not move.
                crate::selector::show_tabs(ui, self);
                egui::ScrollArea::vertical()
                    .id_salt("column_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.tab {
                        Tab::Equalizer => {
                            crate::equalizer::show(ui, self);
                            ui.separator();
                            crate::sound::show(ui, self);
                            ui.separator();
                            crate::upload::show(ui, self);
                        }
                        _ => {
                            crate::selector::show_list(ui, self);
                            ui.separator();
                            crate::equalizer::show_summary(ui, self);
                            ui.separator();
                            crate::upload::show(ui, self);
                        }
                    });
            });
    }

    /// The column on the right: the tools, the graph, and what belongs to it.
    ///
    /// The reference's column scrolls: the graph stays where it is and the
    /// targets and the active curves move under it. The same arrangement here
    /// keeps the target strip from being pushed off the bottom of the window.
    fn right_column(&mut self, ui: &mut Ui) {
        crate::tools::show(ui, self);
        crate::graph::show(ui, self);
        egui::ScrollArea::vertical()
            .id_salt("right_column")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                crate::curves::show(ui, self);
                self.status_line(ui);
            });
    }

    /// Ask for a file, if nothing else is being asked for.
    pub fn ask_for_file(&mut self, what: Pick, title: &str, extensions: &[&str]) {
        if self.picker.is_some() {
            return;
        }
        #[cfg(target_os = "android")]
        {
            // The phone lists its own files: the system's picker hands its
            // answer back through the activity, which the game's activity
            // does not pass on.
            let mime = mime_for(extensions);
            let files = crate::filepick::android::list(&[mime]);
            self.android_files = Some((what, title.to_owned(), files));
        }
        #[cfg(not(target_os = "android"))]
        {
            self.picker = Some((what, crate::filepick::Picker::start(title, extensions)));
        }
    }

    /// The window that lists the device's files, where the phone is the one
    /// being asked.
    #[cfg(target_os = "android")]
    fn show_android_files(&mut self, ctx: &egui::Context) {
        let Some((what, title, files)) = self.android_files.clone() else {
            return;
        };
        let mut chosen: Option<usize> = None;
        egui::Window::new(title)
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                if files.is_empty() {
                    ui.label("There are no files of that kind on this device.");
                }
                for (index, file) in files.iter().enumerate() {
                    if ui
                        .selectable_label(false, format!("{}  ({} bytes)", file.name, file.size))
                        .clicked()
                    {
                        chosen = Some(index);
                    }
                }
            });
        if let Some(index) = chosen {
            let file = files[index].clone();
            self.android_files = None;
            let chosen = crate::filepick::android::chosen(&file);
            self.take_chosen(what, chosen);
        }
    }

    /// Take the file the user chose, and do what it was wanted for.
    fn take_chosen_file(&mut self) {
        let Some((what, picker)) = &mut self.picker else {
            return;
        };
        let Some(chosen) = picker.take() else {
            return;
        };
        let what = *what;
        self.picker = None;
        self.take_chosen(what, chosen);
    }

    /// Do what a file was wanted for.
    fn take_chosen(&mut self, what: Pick, chosen: crate::filepick::Chosen) {
        match what {
            Pick::Upload => {
                self.pending_upload = Some(chosen);
                self.upload_message = None;
            }
            Pick::ImportParametric => self.import_chosen(chosen),
            Pick::Music => self.play_chosen(chosen),
        }
    }

    /// Load the measurement the user chose with the dialog, if there is one,
    /// and otherwise the file they named.
    pub fn upload_chosen(&mut self, as_target: bool) {
        let chosen = self.pending_upload.take();
        let (name, text) = match chosen {
            Some(chosen) => {
                let name = if chosen.name.is_empty() {
                    "The file".to_owned()
                } else {
                    chosen.name.clone()
                };
                match chosen.text() {
                    Some(text) => (name, text),
                    None => {
                        self.upload_message = Some("That file could not be read.".to_owned());
                        return;
                    }
                }
            }
            None => {
                let path = self.upload_path.trim().to_owned();
                if path.is_empty() {
                    self.upload_message = Some("Name the file to load.".to_owned());
                    return;
                }
                let name = std::path::Path::new(&path)
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("The file")
                    .to_owned();
                match std::fs::read_to_string(&path) {
                    Ok(text) => (name, text),
                    Err(problem) => {
                        self.upload_message = Some(format!("The file was not loaded: {problem}"));
                        return;
                    }
                }
            }
        };
        match self.session.import_upload_text(&name, &text, as_target) {
            Ok(message) => {
                self.upload_message = Some(message);
                self.session.rebuild_display_curves();
            }
            Err(problem) => {
                self.upload_message = Some(format!("The file was not loaded: {problem}"))
            }
        }
    }

    /// Load the equalizer file the user chose.
    fn import_chosen(&mut self, chosen: crate::filepick::Chosen) {
        let Some(text) = chosen.text() else {
            self.status = Some("That file could not be read.".to_owned());
            return;
        };
        let Some(phone) = self.session.eq_phone else {
            self.status = Some("Choose a measurement to give it an equalizer.".to_owned());
            return;
        };
        match self.session.import_parametric(phone, &text) {
            Ok(count) => {
                self.status = Some(format!("{count} bands were imported."));
                self.session.rebuild_display_curves();
                crate::sound::push_chain(self);
            }
            Err(problem) => self.status = Some(format!("The file was not imported: {problem}")),
        }
    }

    /// Play the music the user chose.
    fn play_chosen(&mut self, chosen: crate::filepick::Chosen) {
        let Some(bytes) = chosen.contents() else {
            self.sound_message = Some("That file could not be read.".to_owned());
            return;
        };
        crate::music::play_bytes(self, &chosen.name, &bytes);
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
