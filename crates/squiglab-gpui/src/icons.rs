//! The marks the program's buttons carry, in the GPUI interface.
//!
//! Every one of them is the reference's own vector: the paths come from the
//! egui interface's `icons` module, which took them from the reference's
//! stylesheet masks and markup. They are served to GPUI as tiny SVG files
//! through an in-memory asset source, so the same rasteriser the rest of the
//! interface uses draws them.

use gpui::{px, svg, Rgba, SharedString, Styled};

/// A mark one of the row's buttons carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Eye,
    EyeCrossed,
    Pin,
    Download,
    Remove,
    Play,
    Stop,
    Plus,
    Settings,
    Reset,
    Volume,
    Squiggle,
    BaselineLine,
    LabelBox,
    Inspect,
}

/// Whether the mark is filled in, rather than drawn in outline.
fn filled(icon: Icon) -> bool {
    matches!(icon, Icon::Play | Icon::Stop | Icon::Remove | Icon::Plus)
}

/// The box the reference's mark is drawn in.
fn box_of(icon: Icon) -> f32 {
    match icon {
        Icon::Remove | Icon::Plus => 120.0,
        _ => 24.0,
    }
}

/// The reference's paths, one string per subpath.
fn paths_for(icon: Icon) -> &'static [&'static str] {
    match icon {
        Icon::Download => &[
            "M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2 -2v-2",
            "M7 11l5 5l5 -5",
            "M12 4l0 12",
        ],
        Icon::Eye => &[
            "M10 12a2 2 0 1 0 4 0a2 2 0 0 0 -4 0",
            "M21 12c-2.4 4 -5.4 6 -9 6c-3.6 0 -6.6 -2 -9 -6c2.4 -4 5.4 -6 9 -6c3.6 0 6.6 2 9 6",
        ],
        Icon::EyeCrossed => &[
            "M10.585 10.587a2 2 0 0 0 2.829 2.828",
            "M16.681 16.673a8.717 8.717 0 0 1 -4.681 1.327c-3.6 0 -6.6 -2 -9 -6c1.272 -2.12 2.712 -3.678 4.32 -4.674m2.86 -1.146a9.055 9.055 0 0 1 1.82 -.18c3.6 0 6.6 2 9 6c-.666 1.11 -1.379 2.067 -2.138 2.87",
            "M3 3l18 18",
        ],
        Icon::Pin => &[
            "M15 4.5l-4 4l-4 1.5l-1.5 1.5l7 7l1.5 -1.5l1.5 -4l4 -4",
            "M9 15l-4.5 4.5",
            "M14.5 4l5.5 5.5",
        ],
        Icon::Settings => &[
            "M10.325 4.317c.426 -1.756 2.924 -1.756 3.35 0a1.724 1.724 0 0 0 2.573 1.066c1.543 -.94 3.31 .826 2.37 2.37a1.724 1.724 0 0 0 1.065 2.572c1.756 .426 1.756 2.924 0 3.35a1.724 1.724 0 0 0 -1.066 2.573c.94 1.543 -.826 3.31 -2.37 2.37a1.724 1.724 0 0 0 -2.572 1.065c-.426 1.756 -2.924 1.756 -3.35 0a1.724 1.724 0 0 0 -2.573 -1.066c-1.543 .94 -3.31 -.826 -2.37 -2.37a1.724 1.724 0 0 0 -1.065 -2.572c-1.756 -.426 -1.756 -2.924 0 -3.35a1.724 1.724 0 0 0 1.066 -2.573c-.94 -1.543 .826 -3.31 2.37 -2.37c1 .608 2.296 .07 2.572 -1.065",
            "M9 12a3 3 0 1 0 6 0a3 3 0 0 0 -6 0",
        ],
        Icon::Reset => &["M9 14l-4 -4l4 -4", "M5 10h11a4 4 0 1 1 0 8h-1"],
        Icon::Squiggle => &[
            "M21 12h-2c-.894 0 -1.662 -.857 -1.761 -2c-.296 -3.45 -.749 -6 -2.749 -6s-2.5 3.582 -2.5 8s-.5 8 -2.5 8s-2.452 -2.547 -2.749 -6c-.1 -1.147 -.867 -2 -1.763 -2h-2",
        ],
        Icon::BaselineLine => &["M4 12h16"],
        Icon::LabelBox => &["M4 6h16v12h-16z"],
        Icon::Inspect => &["M6 4v16", "M6 8h9a3 3 0 0 1 0 6h-2"],
        Icon::Volume => &[
            "M15 8a5 5 0 0 1 0 8",
            "M17.7 5a9 9 0 0 1 0 14",
            "M6 15h-2a1 1 0 0 1 -1 -1v-4a1 1 0 0 1 1 -1h2l3.5 -4.5a.8 .8 0 0 1 1.5 .5v14a.8 .8 0 0 1 -1.5 .5l-3.5 -4.5",
        ],
        Icon::Play => &[
            "M6 4v16a1 1 0 0 0 1.524 .852l13 -8a1 1 0 0 0 0 -1.704l-13 -8a1 1 0 0 0 -1.524 .852z",
        ],
        Icon::Stop => &["M17 4h-10a3 3 0 0 0 -3 3v10a3 3 0 0 0 3 3h10a3 3 0 0 0 3 -3v-10a3 3 0 0 0 -3 -3z"],
        Icon::Remove => &[
            "M95.36,24.64h0a5,5,0,0,0-7.08,0L60,52.93,31.72,24.64a5,5,0,0,0-7.08,0h0a5,5,0,0,0,0,7.08L52.93,60,24.64,88.28a5,5,0,0,0,0,7.08h0a5,5,0,0,0,7.08,0L60,67.07,88.28,95.36a5,5,0,0,0,7.08,0h0a5,5,0,0,0,0-7.08L67.07,60,95.36,31.72A5,5,0,0,0,95.36,24.64Z",
        ],
        Icon::Plus => &[
            "M110,60h0a5,5,0,0,0-5-5H65V15a5,5,0,0,0-5-5h0a5,5,0,0,0-5,5V55H15a5,5,0,0,0-5,5h0a5,5,0,0,0,5,5H55v40a5,5,0,0,0,5,5h0a5,5,0,0,0,5-5V65h40A5,5,0,0,0,110,60Z",
        ],
    }
}

/// The asset path the icon's SVG is served under.
pub fn asset_path(icon: Icon) -> &'static str {
    match icon {
        Icon::Eye => "icons/eye.svg",
        Icon::EyeCrossed => "icons/eye-crossed.svg",
        Icon::Pin => "icons/pin.svg",
        Icon::Download => "icons/download.svg",
        Icon::Remove => "icons/remove.svg",
        Icon::Play => "icons/play.svg",
        Icon::Stop => "icons/stop.svg",
        Icon::Plus => "icons/plus.svg",
        Icon::Settings => "icons/settings.svg",
        Icon::Reset => "icons/reset.svg",
        Icon::Volume => "icons/volume.svg",
        Icon::Squiggle => "icons/squiggle.svg",
        Icon::BaselineLine => "icons/baseline-line.svg",
        Icon::LabelBox => "icons/label-box.svg",
        Icon::Inspect => "icons/inspect.svg",
    }
}

/// Build the SVG bytes for an icon, so an in-memory asset source can serve
/// them to GPUI's rasteriser.
pub fn svg_bytes(icon: Icon) -> Vec<u8> {
    let vb = box_of(icon);
    let filled = filled(icon);
    let mut body = String::new();
    for path in paths_for(icon) {
        if filled {
            body.push_str(&format!("<path d=\"{}\" fill=\"currentColor\"/>", path));
        } else {
            body.push_str(&format!(
                "<path d=\"{}\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>",
                path
            ));
        }
    }
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\">{}</svg>",
        vb, vb, body
    )
    .into_bytes()
}

/// Render a mark at the reference's size (16 points), in the given colour.
pub fn render(icon: Icon, color: Rgba) -> gpui::Svg {
    svg()
        .path(SharedString::from(asset_path(icon)))
        .size(px(16.0))
        .text_color(color)
}

/// An in-memory asset source that serves the icon SVGs.
pub struct IconAssets;

impl gpui::AssetSource for IconAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        if let Some(name) = path.strip_prefix("icons/") {
            // Map the file name back to an icon.
            let icon = match name {
                "eye.svg" => Icon::Eye,
                "eye-crossed.svg" => Icon::EyeCrossed,
                "pin.svg" => Icon::Pin,
                "download.svg" => Icon::Download,
                "remove.svg" => Icon::Remove,
                "play.svg" => Icon::Play,
                "stop.svg" => Icon::Stop,
                "plus.svg" => Icon::Plus,
                "settings.svg" => Icon::Settings,
                "reset.svg" => Icon::Reset,
                "volume.svg" => Icon::Volume,
                "squiggle.svg" => Icon::Squiggle,
                "baseline-line.svg" => Icon::BaselineLine,
                "label-box.svg" => Icon::LabelBox,
                "inspect.svg" => Icon::Inspect,
                _ => return Ok(None),
            };
            return Ok(Some(std::borrow::Cow::Owned(svg_bytes(icon))));
        }
        Ok(None)
    }

    fn list(&self, _path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}
