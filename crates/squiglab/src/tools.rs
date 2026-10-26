//! The row of tools above the graph.
//!
//! The reference's tools sit in a row that scrolls sideways when it does not
//! fit, and this is that row: the reading controls on the left, the actions
//! on the right.

use egui::{RichText, Ui};

use crate::app::App;
use crate::theme::{self, metrics, TypeStep};

/// The tools.
pub fn show(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();

    egui::ScrollArea::horizontal()
        .id_salt("tools")
        .auto_shrink([false, false])
        .max_height(40.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // The reference puts the actions first, then the reading
                // controls, then the view toggles.
                actions(ui, app);
                rule(ui);
                normalize(ui, app);
                rule(ui);
                smooth(ui, app);
                rule(ui);
                zoom(ui, app);
                rule(ui);
                toggles(ui, app);
                rule(ui);
                theme(ui, app);
            });
        });
    ui.add_space(metrics::GAP);
    let _ = palette;
}

/// A divider between the clusters.
fn rule(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 20.0), egui::Sense::hover());
    let palette = theme::current_palette();
    ui.painter().vline(
        rect.center().x,
        rect.min.y..=rect.max.y,
        egui::Stroke::new(1.0, palette.border),
    );
    ui.add_space(metrics::GAP);
}

/// A cluster's name.
fn label(ui: &mut Ui, text: &str) {
    let palette = theme::current_palette();
    ui.label(
        RichText::new(text.to_uppercase())
            .font(TypeStep::Label.font())
            .color(palette.text_tertiary),
    );
}

/// The zoom: which part of the frequency axis the graph shows.
///
/// The reference's three buttons are Bass, Mids and Treble; there is no
/// "Full", because the whole axis is what the graph shows until one of the
/// three is pressed.
fn zoom(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();
    label(ui, "Zoom");
    for (region, name) in [
        (squiglab_core::graph::ZoomRegion::Bass, "Bass"),
        (squiglab_core::graph::ZoomRegion::Mids, "Mids"),
        (squiglab_core::graph::ZoomRegion::Treble, "Treble"),
    ] {
        let selected = app.session.zoom == region;
        let resp = ui
            .selectable_label(
                selected,
                RichText::new(name)
                    .font(TypeStep::Control.font())
                    .color(if selected {
                        palette.on_accent
                    } else {
                        palette.accent_text
                    }),
            )
            .on_hover_text("Show this part of the frequency axis");
        if resp.clicked() {
            if app.session.zoom == region {
                app.session.zoom = squiglab_core::graph::ZoomRegion::Full;
            } else {
                app.session.zoom = region;
            }
        }
    }
}

/// The normalization: the level the curves are read at.
///
/// The reference writes the unit as a button beside the figure it belongs
/// to, and the two together are one control: pressing the unit says which
/// of the two the figure is read as.
fn normalize(ui: &mut Ui, app: &mut App) {
    label(ui, "Normalize");
    let palette = theme::current_palette();
    let mut by_phon = app.session.phon_normalization;

    // The dB figure and its unit. The reference writes the unit first, and
    // the two are one control.
    ui.horizontal(|ui| {
        unit_button(ui, "dB", by_phon, &mut by_phon, &palette);
        let mut phon = app.session.target_phon;
        if ui
            .add(
                egui::DragValue::new(&mut phon)
                    .range(0.0..=100.0)
                    .speed(1.0),
            )
            .changed()
        {
            app.session.target_phon = phon;
        }
    });
    // The Hz figure and its unit.
    ui.horizontal(|ui| {
        unit_button(ui, "Hz", !by_phon, &mut by_phon, &palette);
        let mut hertz = app.session.target_hz;
        if ui
            .add(
                egui::DragValue::new(&mut hertz)
                    .range(20.0..=20_000.0)
                    .speed(10.0),
            )
            .changed()
        {
            app.session.target_hz = hertz;
        }
    });
    app.session.phon_normalization = by_phon;
}

/// The unit of a figure, pressed to say which of the two the figure is
/// read as.
fn unit_button(
    ui: &mut Ui,
    text: &str,
    chosen: bool,
    by_phon: &mut bool,
    palette: &theme::Palette,
) {
    let button = egui::Button::new(RichText::new(text).font(TypeStep::Body.font()).color(
        if chosen {
            palette.on_accent
        } else {
            palette.accent_text
        },
    ))
    .fill(if chosen { palette.accent } else { palette.page })
    .stroke(egui::Stroke::new(1.0, palette.border))
    .corner_radius(metrics::RADIUS);
    if ui
        .add_sized([44.0, metrics::INPUT_HEIGHT], button)
        .on_hover_text("Read the curves at this, rather than at the other")
        .clicked()
    {
        *by_phon = text == "dB";
    }
}

/// A button that reads as a word, and that is pressed in when the thing it
/// turns on is on.
fn toggle_button<'a>(text: &str, on: bool, palette: &theme::Palette) -> egui::Button<'a> {
    let colour = if on {
        palette.on_accent
    } else {
        palette.text_secondary
    };
    // The reference writes a mark before the word. The native's font does
    // not carry the reference's characters, so the word is given room for
    // the mark and the mark is painted over it.
    egui::Button::new(
        RichText::new(format!("   {text}"))
            .font(TypeStep::Control.font())
            .color(colour),
    )
    .fill(if on { palette.accent } else { palette.page })
    .stroke(egui::Stroke::new(1.0, palette.border))
    .corner_radius(metrics::RADIUS)
}

/// Paint the mark that goes before a button's word.
///
/// The reference writes the mark as a character and the word after it, so
/// the mark is put where the character would stand: just to the left of the
/// word, and a little above its middle.
fn mark(
    ui: &Ui,
    response: &egui::Response,
    icon: crate::icons::Icon,
    word: &str,
    colour: egui::Color32,
) {
    let rect = response.rect;
    // The word's own place, so that the mark is set against it.
    let galley = ui
        .painter()
        .layout_no_wrap(word.to_owned(), TypeStep::Control.font(), colour);
    let text_width = galley.size().x;
    let word_left = rect.right() - 8.0 - text_width;
    let mark = egui::Rect::from_min_size(
        egui::pos2(word_left - 15.0, rect.center().y - 5.0),
        egui::vec2(11.0, 11.0),
    );
    crate::icons::draw(ui.painter(), mark, icon, colour);
}

/// The smoothing: how much of the curve's roughness is taken off.
fn smooth(ui: &mut Ui, app: &mut App) {
    label(ui, "Smooth");
    let mut level = app.session.smooth_level as f64;
    if ui
        .add_sized(
            [56.0, 22.0],
            egui::DragValue::new(&mut level)
                .range(0.0..=squiglab_core::config::MAX_SMOOTH_LEVEL as f64)
                .speed(1.0),
        )
        .on_hover_text("Fractional octaves of smoothing")
        .changed()
    {
        app.session.smooth_level =
            level.clamp(0.0, squiglab_core::config::MAX_SMOOTH_LEVEL as f64) as usize;
    }
}

/// The view toggles.
fn toggles(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();
    // The labels and the inspector are the session's, so they are kept with
    // the rest of the view. The reference writes them as ordinary buttons
    // with a mark before the word, not as boxes to be ticked.
    let mut labels = app.session.show_labels;
    let response = ui
        .add(toggle_button("label", labels, &palette))
        .on_hover_text("Name each curve at its end");
    mark(
        ui,
        &response,
        crate::icons::Icon::LabelBox,
        "label",
        if labels {
            palette.on_accent
        } else {
            palette.text_secondary
        },
    );
    if response.clicked() {
        labels = !labels;
        app.session.show_labels = labels;
    }
    let mut inspect = app.session.inspect_mode;
    let response = ui
        .add(toggle_button("inspect", inspect, &palette))
        .on_hover_text("Read the curve under the pointer");
    mark(
        ui,
        &response,
        crate::icons::Icon::Inspect,
        "inspect",
        if inspect {
            palette.on_accent
        } else {
            palette.text_secondary
        },
    );
    if response.clicked() {
        inspect = !inspect;
        app.session.inspect_mode = inspect;
    }
    if ui
        .button("recolor")
        .on_hover_text("Turn the curve colours around the wheel")
        .clicked()
    {
        app.session.color_offset = (app.session.color_offset + 1) % 16;
        app.session.rebuild_display_curves();
    }
    let _ = palette;
}

/// The theme, named the way the reference names it: the mode the button
/// would switch to.
fn theme(ui: &mut Ui, app: &mut App) {
    let palette = theme::current_palette();
    let next = app.session.theme.next();
    if ui
        .add(
            egui::Button::new(
                RichText::new(next.next_name())
                    .font(TypeStep::Control.font())
                    .color(palette.text_secondary),
            )
            .fill(egui::Color32::TRANSPARENT)
            .stroke(egui::Stroke::NONE)
            .min_size(egui::vec2(0.0, 24.0)),
        )
        .on_hover_text("Switch to the next theme")
        .clicked()
    {
        app.session.theme = next;
    }
}

/// The actions that apply to the window.
fn actions(ui: &mut Ui, app: &mut App) {
    if ui
        .button("Copy URL")
        .on_hover_text("Copy a link to this view")
        .clicked()
    {
        let uri = squiglab_core::config::encode_share_uri(&app.session.preferences());
        match uri {
            Ok(uri) => {
                ui.ctx().copy_text(uri.clone());
                app.say(format!("Link copied: {uri}"));
            }
            Err(error) => app.say(format!("The link could not be made: {error}")),
        }
    }
    if ui
        .button("Screenshot")
        .on_hover_text("Save a picture of the window")
        .clicked()
    {
        app.screenshot_requested = true;
    }
    if ui
        .button("SVG")
        .on_hover_text("Save the graph as a vector picture")
        .clicked()
    {
        let svg = crate::graph::export_svg(app, 1200.0, 360.0);
        match save("graph.svg", &svg) {
            Ok(path) => app.say(format!("The graph was saved to {}", path.display())),
            Err(problem) => app.say(format!("The graph was not saved: {problem}")),
        }
    }
}

/// Write a file into the user's pictures directory, or their home directory if
/// the operating system does not have one.
fn save(name: &str, contents: &str) -> anyhow::Result<std::path::PathBuf> {
    let directory = dirs::picture_dir()
        .or_else(dirs::home_dir)
        .ok_or_else(|| anyhow::anyhow!("the operating system did not provide a directory"))?;
    std::fs::create_dir_all(&directory)?;
    let path = directory.join(name);
    std::fs::write(&path, contents)?;
    Ok(path)
}

/// Write a picture of the window into the user's pictures directory, or their
/// home directory if the operating system does not have one.
pub fn save_picture(name: &str, image: &egui::ColorImage) -> anyhow::Result<std::path::PathBuf> {
    let directory = dirs::picture_dir()
        .or_else(dirs::home_dir)
        .ok_or_else(|| anyhow::anyhow!("the operating system did not provide a directory"))?;
    std::fs::create_dir_all(&directory)?;
    let path = directory.join(name);
    // The toolkit's pixels are premultiplied; a picture on disk is not.
    let pixels: Vec<u8> = image
        .as_raw()
        .chunks(4)
        .flat_map(|pixel| {
            let alpha = pixel[3];
            if alpha == 0 {
                [0, 0, 0, 0]
            } else {
                [
                    (u16::from(pixel[0]) * 255 / u16::from(alpha)).min(255) as u8,
                    (u16::from(pixel[1]) * 255 / u16::from(alpha)).min(255) as u8,
                    (u16::from(pixel[2]) * 255 / u16::from(alpha)).min(255) as u8,
                    alpha,
                ]
            }
        })
        .collect();
    let picture = image::RgbaImage::from_raw(image.width() as u32, image.height() as u32, pixels)
        .ok_or_else(|| anyhow::anyhow!("the picture has no pixels"))?;
    picture.save(&path)?;
    Ok(path)
}
