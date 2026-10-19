//! The theme: the three palettes, the type scale, and the metrics.
//!
//! Every value here is the reference's own, read from its stylesheet. The one
//! departure is the dark theme's accent, which is the native's blue rather
//! than the reference's yellow: the curves are the native's own coloured
//! palette, and a yellow accent fights them.
//!
//! The measure is `docs/design-system.md`.

use egui::{Color32, FontFamily, FontId, Stroke, TextStyle};
use squiglab_core::color::Rgb;

// ---------------------------------------------------------------------------
// Colour
// ---------------------------------------------------------------------------

/// The product's regular typeface.
pub const REGULAR_FAMILY: &str = "open-sans";
/// The product's bold typeface.
pub const BOLD_FAMILY: &str = "open-sans-bold";

/// The three themes the reference offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
    Contrast,
}

/// A resolved set of colours for one theme.
///
/// The names are the reference's variable names, so a value can be traced back
/// to the line of CSS it came from.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub accent: Color32,
    pub accent_contrast: Color32,
    pub on_accent: Color32,
    pub background: Color32,
    pub background_contrast: Color32,
    pub background_contrast_more: Color32,
    pub background_graph: Color32,
    pub background_inputs: Color32,
    pub font_primary: Color32,
    pub font_secondary: Color32,
    pub font_inputs: Color32,
    pub header: Color32,
    pub header_links: Color32,
    pub theme: Theme,
}

impl Palette {
    /// The palette for a theme.
    pub fn resolve(theme: Theme) -> Self {
        match theme {
            Theme::Light => Self::light(),
            Theme::Dark => Self::dark(),
            Theme::Contrast => Self::contrast(),
        }
    }

    /// The light theme, the reference's default.
    pub fn light() -> Self {
        Self {
            accent: hsl(210, 53, 49),
            accent_contrast: hsl(210, 50, 40),
            on_accent: hsl(210, 20, 100),
            background: hsl(54, 5, 96),
            background_contrast: hsl(54, 5, 90),
            background_contrast_more: hsl(54, 0, 65),
            background_graph: hsl(54, 0, 100),
            background_inputs: hsl(210, 10, 98),
            font_primary: hsl(210, 20, 10),
            font_secondary: hsl(210, 20, 100),
            font_inputs: hsl(210, 20, 40),
            header: hsl(215, 20, 10),
            header_links: hsl(0, 0, 100),
            theme: Theme::Light,
        }
    }

    /// The dark theme. The accent is the native's blue, not the reference's
    /// yellow, because the curves are the native's own colours.
    pub fn dark() -> Self {
        Self {
            accent: Color32::from_rgb(0x38, 0x70, 0xA6),
            accent_contrast: hsl(54, 100, 40),
            on_accent: hsl(0, 0, 100),
            background: hsl(215, 14, 10),
            background_contrast: hsl(215, 14, 10),
            background_contrast_more: hsl(215, 14, 25),
            background_graph: hsl(215, 14, 7),
            background_inputs: hsl(215, 14, 5),
            font_primary: hsl(54, 11, 87),
            font_secondary: Color32::BLACK,
            font_inputs: hsl(215, 14, 40),
            header: hsl(215, 20, 10),
            header_links: hsl(0, 0, 100),
            theme: Theme::Dark,
        }
    }

    /// The high contrast theme.
    pub fn contrast() -> Self {
        Self {
            accent: hsl(0, 0, 0),
            accent_contrast: hsl(0, 0, 0),
            on_accent: hsl(0, 0, 100),
            background: hsl(0, 0, 100),
            background_contrast: hsl(0, 0, 0),
            background_contrast_more: hsl(0, 0, 0),
            background_graph: hsl(0, 0, 100),
            background_inputs: hsl(0, 0, 100),
            font_primary: hsl(0, 0, 0),
            font_secondary: hsl(0, 0, 100),
            font_inputs: hsl(0, 0, 0),
            header: hsl(215, 20, 10),
            header_links: hsl(0, 0, 100),
            theme: Theme::Contrast,
        }
    }

    /// Whether the theme is a dark one.
    pub fn is_dark(&self) -> bool {
        !matches!(self.theme, Theme::Light)
    }
}

/// A colour from the reference's own notation.
fn hsl(hue: u8, saturation: u8, lightness: u8) -> Color32 {
    let h = f32::from(hue) / 360.0;
    let s = f32::from(saturation) / 100.0;
    let l = f32::from(lightness) / 100.0;
    if s == 0.0 {
        let value = (l * 255.0).round() as u8;
        return Color32::from_rgb(value, value, value);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let channel = |mut t: f32| -> u8 {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        let value = if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 1.0 / 2.0 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        };
        (value * 255.0).round() as u8
    };
    let mut hue = h;
    if hue < 0.0 {
        hue += 1.0;
    }
    if hue > 1.0 {
        hue -= 1.0;
    }
    match (hue * 6.0).floor() as u8 {
        0 => Color32::from_rgb(
            channel(hue + 1.0 / 3.0),
            channel(hue),
            channel(hue - 1.0 / 3.0),
        ),
        1 => Color32::from_rgb(
            channel(hue),
            channel(hue + 1.0 / 3.0),
            channel(hue - 1.0 / 3.0),
        ),
        2 => Color32::from_rgb(
            channel(hue - 1.0 / 3.0),
            channel(hue),
            channel(hue + 1.0 / 3.0),
        ),
        3 => Color32::from_rgb(
            channel(hue - 1.0 / 3.0),
            channel(hue),
            channel(hue + 1.0 / 3.0),
        ),
        4 => Color32::from_rgb(
            channel(hue),
            channel(hue + 1.0 / 3.0),
            channel(hue - 1.0 / 3.0),
        ),
        _ => Color32::from_rgb(
            channel(hue + 1.0 / 3.0),
            channel(hue),
            channel(hue - 1.0 / 3.0),
        ),
    }
}

/// The core's colour, as the interface paints it.
pub fn color(color: Rgb) -> Color32 {
    Color32::from_rgb(color.r, color.g, color.b)
}

// ---------------------------------------------------------------------------
// Type
// ---------------------------------------------------------------------------

/// The type scale, by the reference's sizes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeStep {
    /// The logo.
    Logo,
    /// Buttons and selects.
    Control,
    /// Labels, inputs and chips.
    Label,
    /// The graph's axis labels.
    Axis,
    /// Body text.
    Body,
}

impl TypeStep {
    /// The size in points.
    pub fn size(self) -> f32 {
        match self {
            TypeStep::Logo => 18.0,
            TypeStep::Control => 12.0,
            TypeStep::Label => 11.0,
            TypeStep::Axis => 11.0,
            TypeStep::Body => 13.0,
        }
    }

    /// The font for this step.
    pub fn font(self) -> FontId {
        FontId::new(self.size(), FontFamily::Proportional)
    }

    /// The bold font for this step.
    pub fn font_bold(self) -> FontId {
        FontId::new(self.size(), FontFamily::Name(BOLD_FAMILY.into()))
    }

    /// The monospaced font for this step, for figures that line up.
    pub fn font_mono(self) -> FontId {
        FontId::new(self.size(), FontFamily::Monospace)
    }
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

/// The reference's measurements.
pub mod metrics {
    /// The header's height.
    pub const HEADER_HEIGHT: f32 = 48.0;
    /// The left column's width.
    pub const COLUMN_WIDTH: f32 = 350.0;
    /// The brand column's width, inside the left column.
    pub const BRAND_COLUMN_WIDTH: f32 = 94.0;
    /// A list row's height.
    pub const ROW_HEIGHT: f32 = 36.0;
    /// An input's height.
    pub const INPUT_HEIGHT: f32 = 36.0;
    /// The standard corner radius.
    pub const RADIUS: f32 = 6.0;
    /// The smaller corner radius.
    pub const RADIUS_SMALL: f32 = 4.0;
    /// A slider's track height.
    pub const SLIDER_HEIGHT: f32 = 16.0;
    /// A slider's thumb.
    pub const THUMB: f32 = 12.0;
    /// The gap between the fields of a row.
    pub const GAP: f32 = 4.0;
    /// The gap between the controls of a cluster.
    pub const GAP_LARGE: f32 = 8.0;
    /// The padding around a panel's contents.
    pub const PADDING: f32 = 16.0;
    /// The padding inside a field.
    pub const PADDING_FIELD: f32 = 10.0;
    /// The width of the graph's column at the narrowest.
    pub const NARROW_VIEWPORT: f32 = 1000.0;
}

// ---------------------------------------------------------------------------
// Setting the toolkit up
// ---------------------------------------------------------------------------

/// Register the product's typefaces. Must be called before the first frame.
pub fn register_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        REGULAR_FAMILY.to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(
            squiglab_core::product::FONT_REGULAR,
        )),
    );
    fonts.font_data.insert(
        BOLD_FAMILY.to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(
            squiglab_core::product::FONT_BOLD,
        )),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .push(REGULAR_FAMILY.to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .push(BOLD_FAMILY.to_owned());
    // The bold face is also a family of its own, so a step can ask for the bold
    // weight directly rather than the toolkit picking it out of the
    // proportional stack.
    fonts.families.insert(
        egui::FontFamily::Name(BOLD_FAMILY.into()),
        vec![BOLD_FAMILY.to_owned()],
    );
    ctx.set_fonts(fonts);
}

/// Apply a theme to the toolkit.
///
/// The toolkit knows of a light and a dark preference, so the reference's
/// three themes are the preference plus the colours on top: the high contrast
/// theme is the light one with its own colours.
pub fn apply(ctx: &egui::Context, theme: Theme) {
    let palette = Palette::resolve(theme);
    ctx.set_theme(match theme {
        Theme::Light | Theme::Contrast => egui::ThemePreference::Light,
        Theme::Dark => egui::ThemePreference::Dark,
    });
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    style.visuals = visuals(&palette);
    style.text_styles = text_styles();
    style.spacing.item_spacing = egui::vec2(metrics::GAP_LARGE, metrics::GAP_LARGE);
    style.spacing.button_padding = egui::vec2(metrics::PADDING_FIELD, 6.0);
    style.spacing.interact_size = egui::vec2(80.0, metrics::INPUT_HEIGHT);
    style.visuals.window_corner_radius = 0.0.into();
    style.visuals.menu_corner_radius = metrics::RADIUS.into();
    // The style is set for both preferences, so the theme does not change under
    // the window's own light or dark setting.
    ctx.set_style_of(egui::Theme::Dark, style.clone());
    ctx.set_style_of(egui::Theme::Light, style);
}

/// The toolkit's colours for a theme.
fn visuals(palette: &Palette) -> egui::Visuals {
    let mut visuals = if palette.is_dark() {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.override_text_color = Some(palette.font_primary);
    visuals.panel_fill = palette.background;
    visuals.window_fill = palette.background;
    visuals.extreme_bg_color = palette.background_graph;
    visuals.faint_bg_color = palette.background_contrast;
    visuals.hyperlink_color = palette.accent;
    visuals.selection.bg_fill = palette.accent.gamma_multiply(0.25);
    visuals.widgets.active.bg_fill = palette.accent;
    visuals.widgets.active.weak_bg_fill = palette.accent;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, palette.on_accent);
    visuals.widgets.hovered.bg_fill = palette.background_contrast_more;
    visuals.widgets.hovered.weak_bg_fill = palette.background_contrast_more;
    visuals.widgets.inactive.bg_fill = palette.background_inputs;
    visuals.widgets.inactive.weak_bg_fill = palette.background_inputs;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette.background_contrast_more);
    visuals.widgets.open.bg_fill = palette.background_contrast;
    visuals.error_fg_color = palette.accent_contrast;
    visuals.warn_fg_color = palette.accent_contrast;
    visuals
}

/// The toolkit's type styles for the reference's scale.
fn text_styles() -> std::collections::BTreeMap<TextStyle, FontId> {
    let mut styles = std::collections::BTreeMap::new();
    styles.insert(TextStyle::Heading, TypeStep::Logo.font_bold());
    styles.insert(TextStyle::Body, TypeStep::Body.font());
    styles.insert(TextStyle::Button, TypeStep::Control.font());
    styles.insert(TextStyle::Small, TypeStep::Label.font());
    styles.insert(TextStyle::Monospace, TypeStep::Body.font_mono());
    styles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_palettes_are_the_reference_s_own() {
        // Light: the reference's default, a warm off-white page.
        let light = Palette::light();
        assert_eq!(light.background, hsl(54, 5, 96));
        assert_eq!(light.accent, hsl(210, 53, 49));

        // Dark: a blue accent, the native's, over the reference's ground.
        let dark = Palette::dark();
        assert_eq!(dark.background, hsl(215, 14, 10));
        assert_eq!(dark.accent, Color32::from_rgb(0x38, 0x70, 0xA6));
        assert_ne!(
            dark.accent, light.accent,
            "the accent changes with the theme"
        );

        // Contrast: black on white, with a black accent.
        let contrast = Palette::contrast();
        assert_eq!(contrast.background, Color32::WHITE);
        assert_eq!(contrast.font_primary, Color32::BLACK);
        assert_eq!(contrast.accent, Color32::BLACK);
    }

    #[test]
    fn the_hue_conversion_is_the_standard_one() {
        assert_eq!(hsl(0, 100, 50), Color32::from_rgb(255, 0, 0));
        assert_eq!(hsl(120, 100, 50), Color32::from_rgb(0, 255, 0));
        assert_eq!(hsl(240, 100, 50), Color32::from_rgb(0, 0, 255));
        assert_eq!(hsl(0, 0, 50), Color32::from_rgb(128, 128, 128));
        assert_eq!(hsl(54, 5, 96), Color32::from_rgb(245, 245, 244));
    }

    #[test]
    fn the_type_scale_is_the_reference_s() {
        assert_eq!(TypeStep::Logo.size(), 18.0);
        assert_eq!(TypeStep::Control.size(), 12.0);
        assert_eq!(TypeStep::Label.size(), 11.0);
    }
}
