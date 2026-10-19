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
        ui.add_space(metrics::GAP);
        // The reference's Compare switch: with it on (B) the source is played
        // through the equalizer, and with it off (A) it is played on its own.
        let mut compare = app.compare;
        let letter = if compare { "B" } else { "A" };
        if ui
            .checkbox(&mut compare, letter)
            .on_hover_text(
                "With B the source is played through the equalizer, and with A it is played \
                 on its own.",
            )
            .changed()
        {
            app.compare = compare;
            push_chain(app);
        }
        ui.label(
            RichText::new("Compare")
                .font(TypeStep::Label.font())
                .color(palette.text_tertiary),
        );
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

    // --- the music ---
    crate::music::show(ui, app);
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
pub fn card(ui: &mut Ui, title: &str, contents: impl FnOnce(&mut Ui)) {
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

/// The filters the source is played through: the shared ones, and the ones
/// the side banks hold.
fn chain(app: &App) -> squiglab_core::audio::Chain {
    use squiglab_core::equalizer::Biquad;
    use squiglab_core::session::EqBank;
    let rate = squiglab_core::equalizer::DEFAULT_SAMPLE_RATE;
    let mut built = squiglab_core::audio::Chain::default();
    let Some(phone) = app.session.eq_phone else {
        return built;
    };
    let Some(rows) = app.session.eq_filters.get(&phone) else {
        return built;
    };
    for row in rows.iter().filter(|row| row.enabled) {
        let filter = Biquad::from_filter(&row.filter, rate);
        match row.bank {
            EqBank::Both => built.shared.push(filter),
            EqBank::Left => built.left.push(filter),
            EqBank::Right => built.right.push(filter),
        }
    }
    built
}

/// Hand the filters to the player, so the sound runs through them.
pub fn push_chain(app: &mut App) {
    let Some(player) = &app.player else {
        return;
    };
    let chain = if app.compare {
        chain(app)
    } else {
        squiglab_core::audio::Chain::default()
    };
    player.set_chain(chain);
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
            push_chain(app);
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
