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
        // The reference's Compare switch sits at the right of the heading:
        // the word, the switch, and the gear.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(metrics::GAP);
            compare_switch(ui, app);
            ui.label(
                RichText::new("Compare")
                    .font(TypeStep::Label.font())
                    .color(palette.text_tertiary),
            );
        });
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
        // The reference's row: the slider, and the frequency it is at.
        let mut frequency = app.frequency;
        let slider_width = (ui.available_width() - 70.0).max(40.0);
        ui.horizontal(|ui| {
            let (slider, moved) = frequency_slider(ui, slider_width, &mut frequency, app.tone_band);
            ui.label(
                RichText::new(format!("{frequency:.0} Hz"))
                    .font(TypeStep::Label.font())
                    .color(palette.text_tertiary),
            );
            slider.on_hover_text("The frequency being played");
            if moved {
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
    });
    ui.add_space(metrics::GAP);

    // --- the band the tone generator sweeps ---
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Range")
                .font(TypeStep::Label.font())
                .color(palette.text_tertiary),
        );
        // The reference writes the word and the arrow that bends back.
        let response = crate::music::text_button(ui, "Reset");
        let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
        crate::icons::draw(
            ui.painter(),
            rect,
            crate::icons::Icon::Reset,
            palette.text_tertiary,
        );
        if response.on_hover_text("Sweep the whole axis").clicked() {
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

/// The reference's Compare switch: a track with A and B in it, and the chosen
/// one on a filled thumb. With B the source is played through the equalizer,
/// and with A it is played on its own.
fn compare_switch(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(54.0, 24.0), egui::Sense::click());
    ui.painter().rect_filled(rect, 12.0, palette.field);
    ui.painter().rect_stroke(
        rect,
        12.0,
        egui::Stroke::new(1.0, palette.border_strong),
        egui::StrokeKind::Inside,
    );
    let font = egui::FontId::new(11.0, egui::FontFamily::Proportional);
    let a = rect.left_center() + egui::vec2(13.0, 0.0);
    let b = rect.right_center() - egui::vec2(13.0, 0.0);
    let (chosen, other, letter, other_letter) = if app.compare {
        (b, a, "B", "A")
    } else {
        (a, b, "A", "B")
    };
    ui.painter().circle_filled(chosen, 7.0, palette.accent);
    ui.painter().text(
        chosen,
        egui::Align2::CENTER_CENTER,
        letter,
        font.clone(),
        palette.on_accent,
    );
    ui.painter().text(
        other,
        egui::Align2::CENTER_CENTER,
        other_letter,
        font,
        palette.text_tertiary,
    );
    if response
        .on_hover_text(
            "With B the source is played through the equalizer, and with A it is played \
             on its own.",
        )
        .clicked()
    {
        app.compare = !app.compare;
        push_chain(app);
    }
}

/// The reference's slider: a thin track with a round thumb on it.
fn frequency_slider(
    ui: &mut Ui,
    width: f32,
    value: &mut f32,
    band: (f32, f32),
) -> (egui::Response, bool) {
    let palette = theme::current_palette();
    let height = 16.0;
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click_and_drag());
    let mut changed = false;
    let track = egui::Rect::from_center_size(rect.center(), egui::vec2(rect.width(), 8.0));
    ui.painter().rect_filled(track, 4.0, palette.field);
    ui.painter().rect_stroke(
        track,
        4.0,
        egui::Stroke::new(1.0, palette.border),
        egui::StrokeKind::Inside,
    );
    let span = (band.1 - band.0).max(1.0);
    let place = ((*value - band.0) / span).clamp(0.0, 1.0);
    let thumb = egui::Rect::from_center_size(
        egui::pos2(track.left() + place * track.width(), track.center().y),
        egui::vec2(12.0, 12.0),
    );
    ui.painter()
        .circle_filled(thumb.center(), 6.0, palette.text_tertiary);
    if response.dragged() || response.clicked() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let place = ((pointer.x - track.left()) / track.width()).clamp(0.0, 1.0);
            *value = band.0 + place * span;
            changed = true;
        }
    }
    (response, changed)
}

/// A source in its own card, as the reference keeps them.
pub fn card(ui: &mut Ui, title: &str, contents: impl FnOnce(&mut Ui)) {
    let palette = theme::current_palette();
    let width = ui.available_width();
    egui::Frame::new()
        .fill(palette.graph)
        .corner_radius(egui::CornerRadius::same(10))
        .stroke(egui::Stroke::new(1.0, palette.border))
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(width - 24.0);
            ui.label(
                RichText::new(title)
                    .font(egui::FontId::new(13.0, egui::FontFamily::Proportional))
                    .color(palette.text_tertiary),
            );
            ui.add_space(metrics::GAP_LARGE);
            contents(ui);
        });
}

/// The play button of a source: a wide pill with the reference's triangle in
/// it, and no lettering.
pub fn play_button(ui: &mut Ui, playing: bool) -> bool {
    let palette = theme::current_palette();
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 44.0), egui::Sense::click());
    let fill = if playing {
        palette.accent
    } else {
        palette.sunken
    };
    let mark = if playing {
        palette.on_accent
    } else {
        palette.text_tertiary
    };
    ui.painter().rect_filled(rect, 22.0, fill);
    ui.painter().rect_stroke(
        rect,
        22.0,
        egui::Stroke::new(1.0, palette.border_strong),
        egui::StrokeKind::Inside,
    );
    let icon = egui::Rect::from_center_size(rect.center(), egui::vec2(20.0, 20.0));
    crate::icons::play(ui.painter(), icon, mark);
    response.clicked()
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
