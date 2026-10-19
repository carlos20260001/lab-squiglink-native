//! The sound tools: the two sources the program can play.
//!
//! The reference has three: music, pink noise, and a tone generator whose
//! slider sweeps a band of frequencies rather than the whole axis. Music is
//! the reference's own and does not travel, so what is here is pink noise and
//! the tone generator, and the band the generator sweeps.

use egui::{RichText, Ui};
use squiglab_core::audio::{Player, Settings, Source};

use crate::app::App;
use crate::theme::{self, metrics, TypeStep};

/// The sound tools.
pub fn show(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Sound Tools")
                .font(TypeStep::Title.font_bold())
                .color(palette.text_primary),
        );
        // The reference's Compare switch, which plays the source through the
        // equalizer or not. The native plays the source on its own, so the
        // switch is not offered.
    });
    ui.add_space(metrics::GAP);

    // --- the volume ---
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Volume")
                .font(TypeStep::Label.font())
                .color(palette.text_tertiary),
        );
        ui.add_sized(
            [160.0, 24.0],
            egui::Slider::new(&mut app.volume, 0.0..=1.0).show_value(false),
        );
    });
    ui.add_space(metrics::GAP);

    // --- the sources ---
    card(ui, "Pink Noise", |ui| {
        if play_button(ui, app.source == Source::PinkNoise && app.player.is_some()) {
            play(app, Source::PinkNoise);
        }
    });
    ui.add_space(metrics::GAP);
    card(ui, "Tone Generator", |ui| {
        if play_button(ui, app.source.is_tone() && app.player.is_some()) {
            play(
                app,
                Source::Tone {
                    freq: f64::from(app.frequency),
                },
            );
        }
        ui.add_space(metrics::GAP);
        let mut frequency = app.frequency;
        if ui
            .add_sized(
                [ui.available_width(), 24.0],
                egui::Slider::new(&mut frequency, app.tone_band.0..=app.tone_band.1).suffix(" Hz"),
            )
            .on_hover_text("The frequency being played")
            .changed()
        {
            app.frequency = frequency;
            if let Some(player) = &app.player {
                player.set_settings(Settings {
                    source: Source::Tone {
                        freq: f64::from(frequency),
                    },
                    volume: app.volume,
                });
            }
        }
    });
    ui.add_space(metrics::GAP);

    // --- the band the tone generator sweeps ---
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Range")
                .font(TypeStep::Label.font())
                .color(palette.text_tertiary),
        );
        if ui
            .button("Reset")
            .on_hover_text("Sweep the whole axis")
            .clicked()
        {
            app.tone_band = (20.0, 20_000.0);
        }
        let mut low = app.tone_band.0;
        let mut high = app.tone_band.1;
        if ui
            .add(
                egui::DragValue::new(&mut low)
                    .range(20.0..=20_000.0)
                    .speed(10.0)
                    .suffix(" Hz"),
            )
            .on_hover_text("The lowest frequency the slider reaches")
            .changed()
        {
            app.tone_band.0 = low;
        }
        if ui
            .add(
                egui::DragValue::new(&mut high)
                    .range(20.0..=20_000.0)
                    .speed(10.0)
                    .suffix(" Hz"),
            )
            .on_hover_text("The highest frequency the slider reaches")
            .changed()
        {
            app.tone_band.1 = high;
        }
        // The high bound never falls below the low one.
        app.tone_band.1 = app.tone_band.1.max(app.tone_band.0);
    });
    if let Some(message) = &app.sound_message {
        ui.add_space(metrics::GAP);
        ui.label(
            RichText::new(message.clone())
                .font(TypeStep::Label.font())
                .color(palette.text_secondary),
        );
    }
}

/// A source in its own card, as the reference keeps them.
fn card(ui: &mut Ui, title: &str, contents: impl FnOnce(&mut Ui)) {
    let palette = theme::current_palette();
    egui::Frame::group(ui.style())
        .fill(palette.sunken)
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.label(
                RichText::new(title)
                    .font(TypeStep::Label.font())
                    .color(palette.text_secondary),
            );
            ui.add_space(metrics::GAP);
            contents(ui);
        });
}

/// The play button of a source: a wide pill, as the reference draws it.
fn play_button(ui: &mut Ui, playing: bool) -> bool {
    let palette = theme::current_palette();
    let (fill, color) = if playing {
        (palette.accent, palette.on_accent)
    } else {
        (palette.sunken, palette.text_secondary)
    };
    ui.add_sized(
        [ui.available_width(), 28.0],
        egui::Button::new(RichText::new(if playing { "Stop" } else { "Play" }).color(color))
            .fill(fill)
            .corner_radius(egui::CornerRadius::same(14)),
    )
    .clicked()
}

/// Start a source playing.
fn play(app: &mut App, source: Source) {
    if source.is_tone() {
        app.frequency = app.frequency.clamp(app.tone_band.0, app.tone_band.1);
    }
    if app.player.is_some() {
        stop(app);
    }
    app.source = source;
    let settings = Settings {
        source,
        volume: app.volume,
    };
    match Player::start(settings) {
        Ok(player) => {
            app.player = Some(player);
            app.sound_message = Some(format!("{} is playing.", source.name()));
        }
        Err(problem) => {
            app.sound_message = Some(format!("The sound could not be started: {problem}"))
        }
    }
}

/// Stop whatever is playing.
fn stop(app: &mut App) {
    if let Some(mut player) = app.player.take() {
        player.stop();
    }
    app.sound_message = Some("The sound has stopped.".to_owned());
}
