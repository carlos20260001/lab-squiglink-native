//! The design tokens: colour, type, and the measure.
//!
//! Every value is the reference's own, read from its stylesheet, so the window
//! looks like the page it is modelled on. `docs/design-system.md` is the
//! extraction these values come from.
//!
//! Two of them are the native's rather than the reference's: the dark theme's
//! accent is a blue, because the curves are the native's own coloured palette
//! and the reference's yellow accent fights them; and the light theme's accent
//! is a step darker than the reference's, because white text on the
//! reference's own accent does not clear 4.5:1.

use egui::{Color32, Context, FontFamily, FontId, Stroke, Style, TextStyle, Visuals};

/// The three themes the reference offers. The choice is a preference, so it
/// lives in the core with the rest of them; only the colours are the
/// interface's.
pub use squiglab_core::theme::Theme;

/// The theme in force, so that anything drawing can ask for it.
///
/// The toolkit knows only of a light and a dark preference, and the reference's
/// third theme is the light one with its own colours, so the toolkit cannot
/// tell it apart from the light theme. The interface keeps the answer here.
static CURRENT: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(1);

/// The theme in force.
pub fn current() -> Theme {
    match CURRENT.load(std::sync::atomic::Ordering::Relaxed) {
        0 => Theme::Light,
        2 => Theme::Contrast,
        _ => Theme::Dark,
    }
}

/// The palette of the theme in force.
pub fn current_palette() -> Palette {
    Palette::resolve_theme(current())
}

/// Record the theme in force.
pub fn set_current(theme: Theme) {
    CURRENT.store(
        match theme {
            Theme::Light => 0,
            Theme::Dark => 1,
            Theme::Contrast => 2,
        },
        std::sync::atomic::Ordering::Relaxed,
    );
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
    /// The padding down the sides of the left column. The reference's
    /// panel and its tab row both carry it, so the column's contents sit
    /// sixteen points in from its edge.
    pub const COLUMN_PADDING: f32 = 16.0;
    /// The brand column's width, inside the left column.
    pub const BRAND_COLUMN_WIDTH: f32 = 70.0;
    /// The list's width, inside the left column.
    pub const LIST_WIDTH: f32 = COLUMN_WIDTH - COLUMN_PADDING * 2.0 - BRAND_COLUMN_WIDTH;
    /// A list row's height.
    pub const ROW_HEIGHT: f32 = 36.0;
    /// An input's height.
    pub const INPUT_HEIGHT: f32 = 36.0;
    /// The standard corner radius.
    pub const RADIUS: f32 = 6.0;
    /// The smaller corner radius.
    pub const RADIUS_SMALL: f32 = 4.0;
    /// A chip's radius, half its height.
    pub const RADIUS_CHIP: f32 = 11.0;
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
    /// The graph's proportion: its width for every unit of its height. The
    /// reference's own plot is 800 by 360, and its sizing rule caps the width
    /// at 2.2 for every unit of height, so the two agree.
    pub const PLOT_ASPECT_RATIO: f32 = 800.0 / 360.0;
    /// The chrome the graph's width is measured against: the window's height
    /// less this is what the graph may be wide by.
    pub const GRAPH_CHROME_HEIGHT: f32 = 300.0;
    /// How many units across the graph may be for every unit of height.
    pub const GRAPH_ASPECT: f32 = 2.2;
    /// Below this width the columns stack.
    pub const NARROW_VIEWPORT: f32 = 1000.0;
}

// ---------------------------------------------------------------------------
// Typography
// ---------------------------------------------------------------------------

/// The reference's type scale: the logo at 18, the controls at 12, the labels
/// at 11, the body at 13.
#[derive(Clone, Copy)]
pub enum TypeStep {
    /// The logo.
    Logo,
    /// Panel headings.
    Title,
    /// Buttons and selects.
    Control,
    /// Labels, inputs and chips.
    Label,
    /// The body.
    Body,
}

impl TypeStep {
    /// The size in points.
    pub fn size(self) -> f32 {
        match self {
            TypeStep::Logo => 18.0,
            TypeStep::Title => 14.0,
            TypeStep::Control => 12.0,
            TypeStep::Label => 11.0,
            TypeStep::Body => 12.0,
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

/// The product's regular typeface.
pub const REGULAR_FAMILY: &str = "open-sans";
/// The product's bold typeface.
pub const BOLD_FAMILY: &str = "open-sans-bold";

// ---------------------------------------------------------------------------
// Colour
// ---------------------------------------------------------------------------

/// A resolved set of colours for one theme.
///
/// The names are the roles the reference's own variables fill, so a value can
/// be traced back to the line of CSS it came from.
#[derive(Clone, Copy)]
pub struct Palette {
    // Surfaces, ordered from deepest to nearest the viewer.
    /// The page itself.
    pub page: Color32,
    /// The graph's ground.
    pub graph: Color32,
    /// The fields and the raised surfaces.
    pub field: Color32,
    /// The sunken surfaces: the quiet fills and the code.
    pub sunken: Color32,

    /// The header's bar, which the reference keeps dark in every theme.
    pub header: Color32,

    // Text.
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_tertiary: Color32,
    pub text_disabled: Color32,

    // Accent: reserved for primary actions, current selection, and focus.
    pub accent: Color32,
    /// The accent as text: darkened for a light page, lightened for a dark
    /// one, so that an outlined chip's label clears 4.5:1 on either.
    pub accent_text: Color32,
    pub accent_hover: Color32,
    pub accent_active: Color32,
    pub on_accent: Color32,

    // Structural strokes and fills.
    pub border: Color32,
    pub border_strong: Color32,
    pub divider: Color32,
    pub hover_fill: Color32,
    pub active_fill: Color32,

    // Semantic status, used for imbalance and validation.
    pub warning: Color32,
    pub error: Color32,
    pub success: Color32,

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
            // The reference's neutrals are plain greys: the page at ninety-six
            // per cent, the fields on white, and the sunken surfaces at ninety.
            page: hsl(0, 0, 96),
            graph: hsl(0, 0, 100),
            field: hsl(0, 0, 100),
            sunken: hsl(0, 0, 90),

            header: hsl(215, 20, 10),
            text_primary: hsl(0, 0, 10),
            text_secondary: hsl(0, 0, 40),
            text_tertiary: hsl(0, 0, 65),
            text_disabled: hsl(0, 0, 70),

            // The reference's accent is hsl(210, 70%, 40%), which carries
            // white text at 5.9:1.
            accent: hsl(210, 70, 40),
            accent_text: hsl(210, 60, 34),
            accent_hover: hsl(210, 70, 34),
            accent_active: hsl(210, 70, 28),
            on_accent: hsl(0, 0, 100),

            border: hsl(0, 0, 82),
            border_strong: hsl(0, 0, 65),
            divider: hsl(0, 0, 90),
            hover_fill: hsl(0, 0, 92),
            active_fill: hsl(0, 0, 86),

            warning: Color32::from_rgb(0xA6, 0x6E, 0x10),
            error: Color32::from_rgb(0xB0, 0x38, 0x38),
            success: Color32::from_rgb(0x2E, 0x7D, 0x4E),

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

            header: Color32::BLACK,
            text_primary: hsl(54, 11, 87),
            text_secondary: hsl(215, 14, 60),
            text_tertiary: hsl(215, 14, 45),
            text_disabled: hsl(215, 14, 30),

            accent: Color32::from_rgb(0x38, 0x70, 0xA6),
            accent_text: Color32::from_rgb(0x6B, 0x9F, 0xD2),
            accent_hover: Color32::from_rgb(0x42, 0x7F, 0xB9),
            accent_active: Color32::from_rgb(0x2E, 0x5F, 0x8D),
            on_accent: Color32::from_rgb(0xFF, 0xFF, 0xFF),

            border: hsl(215, 14, 25),
            border_strong: hsl(215, 14, 35),
            divider: hsl(215, 14, 18),
            hover_fill: hsl(215, 14, 18),
            active_fill: hsl(215, 14, 25),

            warning: Color32::from_rgb(0xE0, 0xA8, 0x40),
            error: Color32::from_rgb(0xD9, 0x6B, 0x6B),
            success: Color32::from_rgb(0x5E, 0xB8, 0x7A),

            theme: Theme::Dark,
        }
    }

    /// The high contrast theme: black on white, and no greys between them.
    pub fn contrast() -> Self {
        Self {
            page: Color32::WHITE,
            graph: Color32::WHITE,
            field: Color32::WHITE,
            sunken: Color32::BLACK,

            header: hsl(215, 20, 10),
            text_primary: Color32::BLACK,
            text_secondary: Color32::BLACK,
            text_tertiary: Color32::BLACK,
            text_disabled: Color32::from_rgb(0x60, 0x60, 0x60),

            accent: Color32::BLACK,
            accent_text: Color32::BLACK,
            accent_hover: Color32::BLACK,
            accent_active: Color32::BLACK,
            on_accent: Color32::WHITE,

            border: Color32::BLACK,
            border_strong: Color32::BLACK,
            divider: Color32::BLACK,
            hover_fill: Color32::from_rgb(0xE0, 0xE0, 0xE0),
            active_fill: Color32::from_rgb(0xC8, 0xC8, 0xC8),

            warning: Color32::BLACK,
            error: Color32::BLACK,
            success: Color32::BLACK,

            theme: Theme::Contrast,
        }
    }

    /// Whether the theme is a dark one.
    pub fn is_dark(&self) -> bool {
        matches!(self.theme, Theme::Dark)
    }

    /// The fill of a selected control.
    pub fn selected_fill(&self) -> Color32 {
        self.accent
    }

    /// The stroke of a control's border.
    pub fn control_stroke(&self) -> Stroke {
        Stroke::new(1.0, self.border)
    }

    /// The border of a control that has keyboard focus.
    pub fn focus_stroke(&self) -> Stroke {
        Stroke::new(1.5, self.accent)
    }
}

/// A colour from the reference's own notation: hue, saturation, lightness, each
/// a percentage.
pub fn hsl(hue: u8, saturation: u8, lightness: u8) -> Color32 {
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
    // Each channel takes the hue at its own offset, and the offsets are what
    // turn around the colour wheel.
    Color32::from_rgb(channel(h + 1.0 / 3.0), channel(h), channel(h - 1.0 / 3.0))
}

/// The core's colour, as the interface paints it.
pub fn color(color: squiglab_core::color::Rgb) -> Color32 {
    Color32::from_rgb(color.r, color.g, color.b)
}

/// The interface's own colour, named the way the core names colours.
pub fn core_color(color: Color32) -> squiglab_core::color::Rgb {
    squiglab_core::color::Rgb::new(color.r(), color.g(), color.b())
}

// ---------------------------------------------------------------------------
// Setting the toolkit up
// ---------------------------------------------------------------------------

/// Register the product's typefaces. Must be called before the first frame.
pub fn register_fonts(ctx: &Context) {
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
    fonts.families.insert(
        FontFamily::Proportional,
        vec![REGULAR_FAMILY.to_owned(), BOLD_FAMILY.to_owned()],
    );
    // The bold face is a family of its own, so a step can ask for the bold
    // weight directly.
    fonts.families.insert(
        FontFamily::Name(BOLD_FAMILY.into()),
        vec![BOLD_FAMILY.to_owned(), REGULAR_FAMILY.to_owned()],
    );
    fonts.families.insert(
        FontFamily::Monospace,
        vec![REGULAR_FAMILY.to_owned(), BOLD_FAMILY.to_owned()],
    );
    ctx.set_fonts(fonts);
}

/// Apply a theme to the toolkit.
///
/// The toolkit knows of a light and a dark preference, so the reference's
/// three themes are the preference plus the colours on top: the high contrast
/// theme is the light one with its own colours.
pub fn apply(ctx: &Context, theme: Theme) {
    set_current(theme);
    ctx.set_theme(match theme {
        Theme::Light | Theme::Contrast => egui::ThemePreference::Light,
        Theme::Dark => egui::ThemePreference::Dark,
    });
    let style = style_for(theme);
    // The style is set for both preferences, so the theme does not change under
    // the window's own light or dark setting.
    ctx.set_style_of(egui::Theme::Dark, style.clone());
    ctx.set_style_of(egui::Theme::Light, style);
}

fn style_for(theme: Theme) -> Style {
    let palette = Palette::resolve_theme(theme);
    Style {
        text_styles: text_styles(),
        spacing: egui::Spacing {
            item_spacing: egui::vec2(metrics::GAP_LARGE, metrics::GAP_LARGE),
            button_padding: egui::vec2(metrics::PADDING_FIELD, 6.0),
            menu_margin: margin(metrics::PADDING, metrics::GAP_LARGE),
            window_margin: margin(metrics::PADDING, metrics::PADDING),
            indent: metrics::GAP_LARGE + metrics::GAP,
            interact_size: egui::vec2(80.0, metrics::INPUT_HEIGHT),
            combo_height: metrics::INPUT_HEIGHT,
            scroll: scroll::style(),
            ..Default::default()
        },
        visuals: visuals(&palette),
        ..Default::default()
    }
}

/// The text styles for the reference's scale.
fn text_styles() -> std::collections::BTreeMap<TextStyle, FontId> {
    use TextStyle::{Body, Button, Heading, Monospace, Small};
    [
        (Small, TypeStep::Label.font()),
        (Body, TypeStep::Body.font()),
        (Button, TypeStep::Control.font()),
        (Heading, TypeStep::Title.font_bold()),
        (Monospace, TypeStep::Label.font_mono()),
        (TextStyle::Name("logo".into()), TypeStep::Logo.font_bold()),
        (TextStyle::Name("title".into()), TypeStep::Title.font_bold()),
        (TextStyle::Name("label".into()), TypeStep::Label.font()),
        (TextStyle::Name("value".into()), TypeStep::Body.font_mono()),
    ]
    .into_iter()
    .collect()
}

/// The toolkit's colours for a theme.
fn visuals(palette: &Palette) -> Visuals {
    let mut visuals = if palette.is_dark() {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    visuals.override_text_color = Some(palette.text_primary);
    visuals.panel_fill = palette.page;
    visuals.window_fill = palette.field;
    visuals.extreme_bg_color = palette.graph;
    visuals.faint_bg_color = palette.sunken;
    visuals.code_bg_color = palette.sunken;
    visuals.text_cursor.stroke.color = palette.accent;
    visuals.hyperlink_color = palette.accent;
    visuals.warn_fg_color = palette.warning;
    visuals.error_fg_color = palette.error;
    visuals.selection.bg_fill = palette.accent;
    visuals.selection.stroke = Stroke::new(1.0, palette.on_accent);
    visuals.widgets.noninteractive.bg_fill = palette.graph;
    visuals.widgets.noninteractive.weak_bg_fill = palette.graph;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.border);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text_primary);
    visuals.widgets.inactive.bg_fill = palette.field;
    visuals.widgets.inactive.weak_bg_fill = palette.field;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette.border);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette.text_secondary);
    visuals.widgets.hovered.bg_fill = palette.hover_fill;
    visuals.widgets.hovered.weak_bg_fill = palette.hover_fill;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette.border_strong);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, palette.text_primary);
    visuals.widgets.active.bg_fill = palette.active_fill;
    visuals.widgets.active.weak_bg_fill = palette.active_fill;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, palette.accent);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, palette.text_primary);
    visuals.widgets.open.bg_fill = palette.field;
    visuals.widgets.open.weak_bg_fill = palette.field;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, palette.accent);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, palette.text_primary);
    for visuals in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
    ] {
        visuals.corner_radius = metrics::RADIUS.into();
    }
    for visuals in [
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        visuals.corner_radius = metrics::RADIUS.into();
    }
    visuals
}

/// egui's [`egui::Margin`] stores whole points, so spacing steps round to it.
fn margin(horizontal: f32, vertical: f32) -> egui::Margin {
    egui::Margin {
        left: horizontal as i8,
        right: horizontal as i8,
        top: vertical as i8,
        bottom: vertical as i8,
    }
}

/// Scrollbar geometry, applied through [`egui::style::Spacing::scroll`].
///
/// The reference themes its scrollbars from CSS custom properties: thin,
/// floating, and recoloured on hover. egui paints a scrollbar's handle from
/// the active widget visuals and its track from the extreme background, so the
/// themed equivalent is a floating bar with the handle drawn in the palette's
/// foreground stroke, which brightens on hover through the same widget states
/// every other control uses.
pub mod scroll {
    use super::metrics;
    /// Width of the bar at rest.
    pub const BAR_WIDTH: f32 = 10.0;
    /// Thin width the floating bar collapses to when the pointer is elsewhere.
    pub const FLOATING_WIDTH: f32 = 4.0;
    /// Space always reserved for the floating bar, so it appears without
    /// covering content.
    pub const FLOATING_ALLOCATED_WIDTH: f32 = metrics::GAP;
    /// Shortest the handle is ever drawn, so a long list still shows a
    /// grabbable span.
    pub const HANDLE_MIN_LENGTH: f32 = 24.0;
    /// Gap between the content and the bar.
    pub const BAR_INNER_MARGIN: f32 = metrics::GAP / 2.0;

    /// The themed scrollbar style, shared by every palette.
    pub fn style() -> egui::style::ScrollStyle {
        egui::style::ScrollStyle {
            floating: true,
            bar_width: BAR_WIDTH,
            floating_width: FLOATING_WIDTH,
            floating_allocated_width: FLOATING_ALLOCATED_WIDTH,
            handle_min_length: HANDLE_MIN_LENGTH,
            bar_inner_margin: BAR_INNER_MARGIN,
            // The handle is painted with the widget's foreground stroke, so it
            // uses the palette's text colors rather than stock egui grey.
            foreground_color: true,
            dormant_handle_opacity: 0.35,
            active_handle_opacity: 0.6,
            interact_handle_opacity: 1.0,
            dormant_background_opacity: 0.0,
            active_background_opacity: 0.1,
            interact_background_opacity: 0.2,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(color: Color32) -> f32 {
        let channel = |value: u8| {
            let value = value as f32 / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
    }

    fn contrast(a: Color32, b: Color32) -> f32 {
        let (a, b) = (luminance(a), luminance(b));
        let (hi, lo) = if a > b { (a, b) } else { (b, a) };
        (hi + 0.05) / (lo + 0.05)
    }

    #[test]
    fn the_hue_conversion_is_the_standard_one() {
        assert_eq!(hsl(0, 100, 50), Color32::from_rgb(255, 0, 0));
        assert_eq!(hsl(120, 100, 50), Color32::from_rgb(0, 255, 0));
        assert_eq!(hsl(240, 100, 50), Color32::from_rgb(0, 0, 255));
        assert_eq!(hsl(0, 0, 50), Color32::from_rgb(128, 128, 128));
        assert_eq!(hsl(0, 0, 100), Color32::WHITE);
        // The reference's light accent, hsl(210, 53%, 49%).
        assert_eq!(hsl(210, 53, 49), Color32::from_rgb(59, 125, 191));
        // The reference's page, hsl(54, 5%, 96%).
        assert_eq!(hsl(54, 5, 96), Color32::from_rgb(245, 245, 244));
    }

    #[test]
    fn a_chip_is_legible_on_the_page() {
        // The target chips are outlined boxes: the page's fill, the accent's
        // darker text, and the accent's fill with chalk text when the target
        // is on the graph.
        for palette in [Palette::light(), Palette::dark(), Palette::contrast()] {
            let outline = contrast(palette.accent_text, palette.page);
            assert!(
                outline >= 4.5,
                "a chip's text on the page fell to {outline} (dark: {})",
                palette.is_dark()
            );
            let selected = contrast(palette.on_accent, palette.accent);
            assert!(
                selected >= 4.5,
                "a selected chip's text fell to {selected} (dark: {})",
                palette.is_dark()
            );
        }
    }

    #[test]
    fn the_themes_are_the_reference_s_own() {
        // The light theme's page is the reference's plain grey, and its
        // accent is the reference's own blue.
        assert_eq!(Palette::light().page, hsl(0, 0, 96));
        assert_eq!(Palette::light().graph, hsl(0, 0, 100));
        assert_eq!(Palette::light().accent, hsl(210, 70, 40));
        // The dark theme's accent is the native's blue.
        assert_eq!(Palette::dark().accent, Color32::from_rgb(0x38, 0x70, 0xA6));
        assert_eq!(Palette::dark().page, hsl(215, 14, 10));
        // The contrast theme is black on white.
        assert_eq!(Palette::contrast().page, Color32::WHITE);
        assert_eq!(Palette::contrast().text_primary, Color32::BLACK);
    }

    #[test]
    fn body_text_clears_wcag_aa_on_every_surface() {
        for palette in [Palette::dark(), Palette::light()] {
            for surface in [palette.graph, palette.page, palette.field] {
                let ratio = contrast(palette.text_primary, surface);
                assert!(
                    ratio >= 4.5,
                    "primary text on surface fell to {ratio} (dark: {})",
                    palette.is_dark()
                );
                let secondary = contrast(palette.text_secondary, surface);
                assert!(
                    secondary >= 4.5,
                    "secondary text on surface fell to {secondary} (dark: {})",
                    palette.is_dark()
                );
            }
        }
    }

    #[test]
    fn accent_text_clears_aa_on_its_fill() {
        for palette in [Palette::dark(), Palette::light(), Palette::contrast()] {
            let ratio = contrast(palette.on_accent, palette.accent);
            assert!(ratio >= 4.5, "on_accent fell to {ratio}");
        }
    }

    #[test]
    fn the_type_scale_is_the_reference_s_own() {
        assert_eq!(TypeStep::Logo.size(), 18.0);
        assert_eq!(TypeStep::Control.size(), 12.0);
        assert_eq!(TypeStep::Label.size(), 11.0);
        assert_eq!(TypeStep::Body.size(), 12.0);
    }

    #[test]
    fn the_measure_is_the_reference_s_own() {
        assert_eq!(metrics::HEADER_HEIGHT, 48.0);
        assert_eq!(metrics::COLUMN_WIDTH, 350.0);
        assert_eq!(metrics::BRAND_COLUMN_WIDTH, 70.0);
        assert_eq!(metrics::LIST_WIDTH, 248.0);
        assert_eq!(metrics::ROW_HEIGHT, 36.0);
        assert_eq!(metrics::PLOT_ASPECT_RATIO, 800.0 / 360.0);
    }
}
