//! The music player.
//!
//! The reference plays a music file, and offers the short previews the Apple
//! Music catalogue serves. The catalogue is the iTunes search service: it
//! needs no key, and every track comes with a preview of it.

use std::path::PathBuf;

use crate::app::App;
use crate::theme::{self, metrics, TypeStep};
use egui::{RichText, Ui};

/// One track the catalogue offers.
#[derive(Clone, Debug)]
pub struct Track {
    /// The track's name.
    pub title: String,
    /// Who recorded it.
    pub artist: String,
    /// The album it is on.
    pub album: String,
    /// Where a preview of it can be fetched.
    pub preview: String,
}

impl Track {
    /// How the track is written on one line.
    pub fn label(&self) -> String {
        format!("{} — {}", self.artist, self.title)
    }
}

/// What the player is doing.
#[derive(Clone, Default)]
pub struct Music {
    /// The file the user named.
    pub path: String,
    /// The track being played, and where it came from.
    pub playing: Option<Track>,
    /// The search being typed.
    pub query: String,
    /// The results of the last search.
    pub results: Vec<Track>,
    /// Whether a search is running.
    pub searching: bool,
    /// Whether the file's field is open.
    pub naming: bool,
    /// Whether the search's field is open.
    pub searching_for: bool,
    /// What the last search or play said.
    pub message: Option<String>,
}

/// The search thread's result.
pub type SearchHandle = std::thread::JoinHandle<Result<Vec<Track>, String>>;

/// The preview thread's result.
pub type PreviewHandle = std::thread::JoinHandle<Result<Vec<u8>, String>>;

/// The card, as the reference draws it.
/// A button with no border and no fill, the way the reference writes its
/// quiet actions.
pub fn text_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    text_button_in(ui, text, None)
}

/// The same, but spread across a width, with the words in the middle.
pub fn text_button_in(ui: &mut egui::Ui, text: &str, width: Option<f32>) -> egui::Response {
    let palette = crate::theme::current_palette();
    let font = egui::FontId::new(12.0, egui::FontFamily::Proportional);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font, palette.text_tertiary);
    let size = galley.size();
    let box_width = width.unwrap_or(size.x + 8.0);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(box_width, size.y + 12.0), egui::Sense::click());
    let colour = if response.hovered() {
        palette.accent_text
    } else {
        palette.text_tertiary
    };
    let at = if width.is_some() {
        rect.center() - egui::vec2(size.x * 0.5, size.y * 0.5)
    } else {
        rect.left_center() + egui::vec2(4.0, 0.0)
    };
    ui.painter().galley(at, galley, colour);
    if response.hovered() {
        let under = egui::Rect::from_min_max(
            egui::pos2(at.x, at.y + size.y),
            egui::pos2(at.x + size.x, at.y + size.y),
        );
        ui.painter().hline(
            under.min.x..=under.max.x,
            under.min.y,
            egui::Stroke::new(1.0, palette.accent_text),
        );
    }
    response
}

pub fn show(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();
    let path = app.music.path.clone();
    let query = app.music.query.clone();
    let results = app.music.results.clone();
    let searching = app.music.searching;
    let message = app.music.message.clone();
    let playing = app.music.playing.clone();

    crate::sound::card(ui, "Music", |ui| {
        // --- the two ways in ---
        // The reference's row: add a file, or search the catalogue. Both are
        // plain words, with no border around them.
        ui.horizontal(|ui| {
            if crate::music::text_button(ui, "+ Add Music")
                .on_hover_text("Play a file from this computer")
                .clicked()
            {
                app.music.naming = !app.music.naming;
                app.music.searching_for = false;
            }
            if crate::music::text_button(ui, "Search Apple")
                .on_hover_text("Search the Apple Music catalogue for a preview")
                .clicked()
            {
                app.music.searching_for = !app.music.searching_for;
                app.music.naming = false;
            }
        });

        // --- the file ---
        if app.music.naming {
            ui.add_space(metrics::GAP);
            ui.horizontal(|ui| {
                if ui.button("Play").on_hover_text("Play this file").clicked() {
                    play_file(app);
                }
                ui.add(
                    egui::TextEdit::singleline(&mut app.music.path)
                        .hint_text("A music file")
                        .desired_width(ui.available_width() - 60.0),
                );
            });
        }

        // --- the search ---
        if app.music.searching_for {
            ui.add_space(metrics::GAP);
            ui.horizontal(|ui| {
                if ui.button("Search").on_hover_text("Search").clicked() {
                    start_search(app);
                }
                if searching {
                    ui.spinner();
                }
                ui.add(
                    egui::TextEdit::singleline(&mut app.music.query)
                        .hint_text("A track or an artist")
                        .desired_width(ui.available_width() - 80.0),
                );
            });
        }

        // --- the results ---
        if !results.is_empty() {
            ui.add_space(metrics::GAP);
            egui::ScrollArea::vertical()
                .id_salt("apple_results")
                .max_height(150.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (index, track) in results.iter().enumerate() {
                        if ui
                            .selectable_label(false, track.label())
                            .on_hover_text(&track.album)
                            .clicked()
                        {
                            play_preview(app, index);
                        }
                    }
                });
        }

        // --- what is playing ---
        if let Some(track) = &playing {
            ui.add_space(metrics::GAP);
            ui.label(
                RichText::new(format!("Playing: {}", track.label()))
                    .font(TypeStep::Label.font())
                    .color(palette.text_secondary),
            );
        }
        if let Some(message) = &message {
            ui.add_space(metrics::GAP);
            ui.label(
                RichText::new(message.clone())
                    .font(TypeStep::Label.font())
                    .color(palette.text_secondary),
            );
        }
    });
    let _ = (path, query);
}

/// Take the results of a search that has finished.
pub fn take_results(app: &mut App) {
    let Some(handle) = app.pending_search.take() else {
        return;
    };
    if !handle.is_finished() {
        // Not yet: put it back and look again next frame.
        app.pending_search = Some(handle);
        return;
    }
    app.music.searching = false;
    match handle.join() {
        Ok(Ok(results)) => {
            let empty = results.is_empty();
            app.music.results = results;
            if empty {
                app.music.message = Some("Nothing came back for that.".to_owned());
            }
        }
        Ok(Err(problem)) => app.music.message = Some(problem),
        Err(_) => app.music.message = Some("The search failed.".to_owned()),
    }
}

/// Take the preview that has finished downloading.
pub fn take_preview(app: &mut App) {
    let Some((track, handle)) = app.pending_preview.take() else {
        return;
    };
    if !handle.is_finished() {
        app.pending_preview = Some((track, handle));
        return;
    }
    match handle.join() {
        Ok(Ok(bytes)) => match squiglab_core::audio::decode::decode_bytes(&bytes) {
            Ok(decoded) => {
                play_decoded(app, decoded);
                app.music.playing = Some(track);
                app.music.message = Some("Playing the preview.".to_owned());
            }
            Err(problem) => app.music.message = Some(problem),
        },
        Ok(Err(problem)) => app.music.message = Some(problem),
        Err(_) => app.music.message = Some("The preview could not be fetched.".to_owned()),
    }
}

/// Play the file the user named.
fn play_file(app: &mut App) {
    let path = app.music.path.trim().to_owned();
    if path.is_empty() {
        app.music.message = Some("Name the file to play.".to_owned());
        return;
    }
    match squiglab_core::audio::decode::decode_file(std::path::Path::new(&path)) {
        Ok(decoded) => {
            let seconds = decoded.seconds();
            play_decoded(app, decoded);
            app.music.message = Some(format!("Playing {path} ({seconds:.0} seconds)."));
        }
        Err(problem) => app.music.message = Some(problem),
    }
}

/// Search the catalogue.
fn start_search(app: &mut App) {
    let query = app.music.query.trim().to_owned();
    if query.is_empty() {
        app.music.message = Some("Type something to search for.".to_owned());
        return;
    }
    app.music.searching = true;
    app.music.message = None;
    // The search is a network call, so it runs on its own thread and the
    // window keeps drawing while it waits.
    app.pending_search = Some(std::thread::spawn(move || search_apple(&query)));
}

/// Play a preview from the results.
fn play_preview(app: &mut App, index: usize) {
    let Some(track) = app.music.results.get(index).cloned() else {
        return;
    };
    let preview = track.preview.clone();
    app.music.message = Some(format!("Fetching {}…", track.label()));
    app.pending_preview = Some((track, std::thread::spawn(move || fetch_preview(&preview))));
}

/// Search the catalogue for a term.
pub fn search_apple(query: &str) -> Result<Vec<Track>, String> {
    let url = format!(
        "https://itunes.apple.com/search?term={}&entity=song&limit=12",
        urlencode(query)
    );
    let body = ureq::get(&url)
        .call()
        .map_err(|error| format!("The catalogue could not be reached: {error}"))?
        .body_mut()
        .read_to_vec()
        .map_err(|error| format!("The catalogue's answer could not be read: {error}"))?;
    let value: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|error| format!("The answer was not understood: {error}"))?;
    let mut tracks = Vec::new();
    for result in value
        .get("results")
        .and_then(|results| results.as_array())
        .into_iter()
        .flatten()
    {
        let Some(preview) = result.get("previewUrl").and_then(|url| url.as_str()) else {
            continue;
        };
        tracks.push(Track {
            title: result
                .get("trackName")
                .and_then(|name| name.as_str())
                .unwrap_or("Unknown")
                .to_owned(),
            artist: result
                .get("artistName")
                .and_then(|name| name.as_str())
                .unwrap_or("Unknown")
                .to_owned(),
            album: result
                .get("collectionName")
                .and_then(|name| name.as_str())
                .unwrap_or("")
                .to_owned(),
            preview: preview.to_owned(),
        });
    }
    Ok(tracks)
}

/// Fetch a preview's bytes.
pub fn fetch_preview(url: &str) -> Result<Vec<u8>, String> {
    ureq::get(url)
        .call()
        .map_err(|error| format!("The preview could not be fetched: {error}"))?
        .body_mut()
        .read_to_vec()
        .map_err(|error| format!("The preview could not be read: {error}"))
}

/// Play a decoded file through the equalizer.
fn play_decoded(app: &mut App, decoded: squiglab_core::audio::decode::Decoded) {
    use squiglab_core::audio::{FilePlayback, Player, Settings, Source};
    let settings = Settings {
        source: Source::File,
        volume: app.volume,
    };
    let playback = FilePlayback {
        samples: decoded.samples,
        channels: decoded.channels,
        rate: decoded.rate,
        position: 0.0,
        looping: true,
    };
    match Player::start(settings) {
        Ok(player) => {
            player.set_file(playback);
            app.player = Some(player);
            app.source = Source::File;
            crate::sound::push_chain(app);
        }
        Err(problem) => app.say(format!("The music could not be started: {problem}")),
    }
}

/// A term, written the way a URL wants it.
fn urlencode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// The preview file, kept so a fetch that is running can be found again.
pub type PendingPreview = PathBuf;
