//! The reference's design tokens, in the colours GPUI paints with.
//!
//! Every value here is the reference's own, measured from its stylesheet and
//! from the page it renders. The same tokens live in the egui interface
//! (`crates/squiglab/src/theme.rs`); until they are lifted into a crate both
//! interfaces can read, this is a copy of that file's values, kept in the
//! same order so the two can be read side by side.
//!
//! The only values that are the native's rather than the reference's are the
//! dark theme's accent (a blue, because the curves are the native's own
//! coloured palette) and the light theme's accent (a step darker, so white
//! text on it clears 4.5:1). `docs/design-system.md` is the extraction.

#![allow(dead_code)]

use std::borrow::Cow;

use gpui::{App, Rgba};
use squiglab_core::product;
use squiglab_core::theme::Theme;

/// The family the product's regular and bold faces register under.
pub const FONT_FAMILY: &str = "Open Sans";

/// Load the product's typefaces. Must run before the first window is made.
pub fn install(cx: &mut App) {
    cx.text_system()
        .add_fonts(vec![
            Cow::Borrowed(product::FONT_REGULAR),
            Cow::Borrowed(product::FONT_BOLD),
        ])
        .expect("the product's fonts load");
}

// ---------------------------------------------------------------------------
// The measure
// ---------------------------------------------------------------------------

/// The reference's measurements, in points.
pub mod metrics {
    /// The header's height.
    pub const HEADER_HEIGHT: f32 = 48.0;
    /// The left column's width.
    pub const COLUMN_WIDTH: f32 = 350.0;
    /// The padding down the sides of the left column.
    pub const COLUMN_PADDING: f32 = 16.0;
    /// The brand column's width, inside the left column.
    pub const BRAND_COLUMN_WIDTH: f32 = 70.0;
    /// The list's width, inside the left column.
    pub const LIST_WIDTH: f32 = COLUMN_WIDTH - COLUMN_PADDING * 2.0 - BRAND_COLUMN_WIDTH;
    /// A list row's height.
    pub const ROW_HEIGHT: f32 = 40.0;
    /// The gap between one list row and the next.
    pub const ROW_GAP: f32 = 6.0;
    /// An input's height.
    pub const INPUT_HEIGHT: f32 = 36.0;
    /// The standard corner radius.
    pub const RADIUS: f32 = 6.0;
    /// The gap between the fields of a row.
    pub const GAP: f32 = 4.0;
    /// The gap between the controls of a cluster.
    pub const GAP_LARGE: f32 = 8.0;
    /// The padding around a panel's contents.
    pub const PADDING: f32 = 16.0;
    /// The padding inside a field.
    pub const PADDING_FIELD: f32 = 10.0;
    /// The height of the tab row.
    pub const TAB_HEIGHT: f32 = 36.0;
}

// ---------------------------------------------------------------------------
// The type
// ---------------------------------------------------------------------------

/// The reference's type scale.
pub mod type_scale {
    /// The logo.
    pub const LOGO: f32 = 18.0;
    /// Panel titles.
    pub const TITLE: f32 = 14.0;
    /// Buttons and selects.
    pub const CONTROL: f32 = 12.0;
    /// Labels, inputs and chips.
    pub const LABEL: f32 = 11.0;
    /// The body.
    pub const BODY: f32 = 12.0;
    /// A panel's heading.
    pub const HEADING: f32 = 13.0;
}

// ---------------------------------------------------------------------------
// Colour
// ---------------------------------------------------------------------------

/// A resolved set of colours for one theme.
///
/// The names are the roles the reference's own variables fill, so a value can
/// be traced back to the line of CSS it came from.
#[derive(Clone, Copy)]
pub struct Palette {
    /// The page itself.
    pub page: Rgba,
    /// The graph's ground.
    pub graph: Rgba,
    /// The fields and the raised surfaces.
    pub field: Rgba,
    /// The sunken surfaces.
    pub sunken: Rgba,

    /// The header's bar, which is dark in every theme.
    pub header: Rgba,

    pub text_primary: Rgba,
    pub text_secondary: Rgba,
    /// The tab labels at rest, which the reference keeps quieter than the
    /// body but darker than a label.
    pub text_quiet: Rgba,
    pub text_tertiary: Rgba,
    pub text_disabled: Rgba,
    /// The colour of a figure typed into a field.
    pub text_input: Rgba,

    pub accent: Rgba,
    pub accent_text: Rgba,
    pub accent_hover: Rgba,
    pub accent_active: Rgba,
    pub on_accent: Rgba,
    /// The colour written on a fill of `text_primary`: the reference's
    /// `--font-color-secondary`, which is white on the light and contrast
    /// themes and black on the dark one.
    pub on_primary: Rgba,

    pub border: Rgba,
    pub border_strong: Rgba,
    pub divider: Rgba,
    pub hover_fill: Rgba,
    pub active_fill: Rgba,

    pub warning: Rgba,
    pub error: Rgba,
    pub success: Rgba,

    /// Which theme this palette is.
    pub theme: Theme,
}

impl Palette {
    /// The palette for a theme.
    pub fn resolve_theme(theme: Theme) -> Self {
        match theme {
            Theme::Light => Self::light(),
            Theme::Dark => Self::dark(),
            Theme::Contrast => Self::contrast(),
        }
    }

    /// The light theme: the reference's default.
    pub fn light() -> Self {
        Self {
            page: hsl(0, 0, 96),
            graph: hsl(0, 0, 100),
            field: hsl(0, 0, 100),
            sunken: hsl(0, 0, 90),

            header: hsl(215, 20, 10),
            text_primary: hsl(0, 0, 10),
            text_secondary: hsl(0, 0, 40),
            text_quiet: hsl(0, 0, 55),
            text_tertiary: hsl(0, 0, 65),
            text_disabled: hsl(0, 0, 70),
            text_input: hsl(0, 0, 50),

            accent: hsl(210, 70, 40),
            accent_text: hsl(210, 60, 34),
            accent_hover: hsl(210, 70, 34),
            accent_active: hsl(210, 70, 28),
            on_accent: hsl(0, 0, 100),
            on_primary: hsl(0, 0, 100),

            border: hsl(0, 0, 82),
            border_strong: hsl(0, 0, 65),
            divider: hsl(0, 0, 90),
            hover_fill: hsl(0, 0, 92),
            active_fill: hsl(0, 0, 86),

            warning: rgb(0xA6, 0x6E, 0x10),
            error: rgb(0xB0, 0x38, 0x38),
            success: rgb(0x2E, 0x7D, 0x4E),

            theme: Theme::Light,
        }
    }

    /// The dark theme. The accent is the native's blue, not the reference's
    /// yellow, because the curves are the native's own colours.
    pub fn dark() -> Self {
        Self {
            page: hsl(215, 14, 10),
            graph: hsl(215, 14, 7),
            field: hsl(215, 14, 5),
            sunken: hsl(215, 14, 10),

            header: rgb(0x00, 0x00, 0x00),
            text_primary: hsl(54, 11, 87),
            text_secondary: hsl(215, 14, 60),
            text_quiet: hsl(215, 14, 55),
            text_tertiary: hsl(215, 14, 45),
            text_disabled: hsl(215, 14, 30),
            text_input: hsl(215, 14, 55),

            accent: rgb(0x38, 0x70, 0xA6),
            accent_text: rgb(0x6B, 0x9F, 0xD2),
            accent_hover: rgb(0x42, 0x7F, 0xB9),
            accent_active: rgb(0x2E, 0x5F, 0x8D),
            on_accent: rgb(0xFF, 0xFF, 0xFF),
            on_primary: rgb(0x00, 0x00, 0x00),

            border: hsl(215, 14, 25),
            border_strong: hsl(215, 14, 35),
            divider: hsl(215, 14, 18),
            hover_fill: hsl(215, 14, 18),
            active_fill: hsl(215, 14, 25),

            warning: rgb(0xE0, 0xA8, 0x40),
            error: rgb(0xD9, 0x6B, 0x6B),
            success: rgb(0x5E, 0xB8, 0x7A),

            theme: Theme::Dark,
        }
    }

    /// The high contrast theme: black on white, and no greys between them.
    pub fn contrast() -> Self {
        Self {
            page: rgb(0xFF, 0xFF, 0xFF),
            graph: rgb(0xFF, 0xFF, 0xFF),
            field: rgb(0xFF, 0xFF, 0xFF),
            sunken: rgb(0x00, 0x00, 0x00),

            header: hsl(215, 20, 10),
            text_primary: rgb(0x00, 0x00, 0x00),
            text_secondary: rgb(0x00, 0x00, 0x00),
            text_quiet: rgb(0x00, 0x00, 0x00),
            text_tertiary: rgb(0x00, 0x00, 0x00),
            text_disabled: rgb(0x60, 0x60, 0x60),
            text_input: rgb(0x40, 0x40, 0x40),

            accent: rgb(0x00, 0x00, 0x00),
            accent_text: rgb(0x00, 0x00, 0x00),
            accent_hover: rgb(0x00, 0x00, 0x00),
            accent_active: rgb(0x00, 0x00, 0x00),
            on_accent: rgb(0xFF, 0xFF, 0xFF),
            on_primary: rgb(0xFF, 0xFF, 0xFF),

            border: rgb(0x00, 0x00, 0x00),
            border_strong: rgb(0x00, 0x00, 0x00),
            divider: rgb(0x00, 0x00, 0x00),
            hover_fill: rgb(0xE0, 0xE0, 0xE0),
            active_fill: rgb(0xC8, 0xC8, 0xC8),

            warning: rgb(0x00, 0x00, 0x00),
            error: rgb(0x00, 0x00, 0x00),
            success: rgb(0x00, 0x00, 0x00),

            theme: Theme::Contrast,
        }
    }

    /// Whether the theme is a dark one.
    pub fn is_dark(&self) -> bool {
        matches!(self.theme, Theme::Dark)
    }
}

/// An opaque colour from its channels.
pub fn rgb(red: u8, green: u8, blue: u8) -> Rgba {
    Rgba {
        r: f32::from(red) / 255.0,
        g: f32::from(green) / 255.0,
        b: f32::from(blue) / 255.0,
        a: 1.0,
    }
}

/// A colour from the reference's own notation: hue, saturation, lightness.
pub fn hsl(hue: u8, saturation: u8, lightness: u8) -> Rgba {
    let h = f32::from(hue) / 360.0;
    let s = f32::from(saturation) / 100.0;
    let l = f32::from(lightness) / 100.0;
    if s == 0.0 {
        let value = (l * 255.0).round() as u8;
        return rgb(value, value, value);
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
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        };
        (value * 255.0).round() as u8
    };
    rgb(channel(h + 1.0 / 3.0), channel(h), channel(h - 1.0 / 3.0))
}

/// The core's colour, as GPUI paints it.
pub fn color(color: squiglab_core::color::Rgb) -> Rgba {
    rgb(color.r, color.g, color.b)
}
