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
    ui.label(
        RichText::new("Sound Tools")
            .font(TypeStep::Title.font_bold())
            .color(palette.text_primary),
    );
    ui.add_space(metrics::GAP);

    // --- the sources ---
    ui.horizontal(|ui| {
        if ui
            .button("Pink noise")
            .on_hover_text("Play pink noise")
            .clicked()
        {
            play(app, Source::PinkNoise);
        }
        if ui.button("Tone").on_hover_text("Play a tone").clicked() {
            play(app, Source::Tone { freq: 1000.0 });
        }
        if app.player.is_some() && ui.button("Stop").clicked() {
            stop(app);
        }
    });
    ui.add_space(metrics::GAP);

    // --- the volume ---
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Volume")
                .font(TypeStep::Label.font())
                .color(palette.text_tertiary),
        );
        let mut volume = app.volume;
        if ui
            .add(egui::Slider::new(&mut volume, 0.0..=1.0).show_value(false))
            .on_hover_text("How loud the source plays")
            .changed()
        {
            app.volume = volume;
            if let Some(player) = &app.player {
                player.set_settings(Settings {
                    source: app.source,
                    volume: app.volume,
                });
            }
        }
    });
    ui.add_space(metrics::GAP);

    // --- the tone generator, and the band it sweeps ---
    if app.source.is_tone() {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Band")
                    .font(TypeStep::Label.font())
                    .color(palette.text_tertiary),
            );
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
            if ui
                .button("Reset")
                .on_hover_text("Sweep the whole axis")
                .clicked()
            {
                app.tone_band = (20.0, 20_000.0);
            }
        });
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Frequency")
                    .font(TypeStep::Label.font())
                    .color(palette.text_tertiary),
            );
            let mut frequency = app.frequency;
            if ui
                .add(
                    egui::Slider::new(&mut frequency, app.tone_band.0..=app.tone_band.1)
                        .suffix(" Hz"),
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
    }
}

/// Start a source playing.
fn play(app: &mut App, source: Source) {
    if source.is_tone() {
        app.frequency = app.frequency.clamp(app.tone_band.0, app.tone_band.1);
    }
    app.source = source;
    let settings = Settings {
        source,
        volume: app.volume,
    };
    match Player::start(settings) {
        Ok(player) => {
            app.player = Some(player);
            app.say(format!("{} is playing.", source.name()));
        }
        Err(problem) => app.say(format!("The sound could not be started: {problem}")),
    }
}

/// Stop whatever is playing.
fn stop(app: &mut App) {
    if let Some(mut player) = app.player.take() {
        player.stop();
    }
    app.say("The sound has stopped.");
}
