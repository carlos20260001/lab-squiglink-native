//! The design tokens: colour, type, spacing, and the way the toolkit is set up.
//!
//! Every value is the reference's own, read from its stylesheet, so the window
//! looks like the page it is modelled on. `docs/design-system.md` is the
//! extraction these values come from.
//!
//! Two of them are the native's rather than the reference's: the dark theme's
//! accent is a blue, because the curves are the native's own coloured palette
//! and the reference's yellow accent fights them; and the curve colours are
//! the native's golden-angle palette, because the reference greys its curves
//! for a page that carries one accent, and the window carries many.

use egui::{Color32, Context, FontFamily, FontId, Stroke, Style, TextStyle, Visuals};

/// Font family name for the bold face, used for emphasis in places where the
/// built-in text styles do not express enough.
pub const FONT_BOLD: &str = "open-sans-bold";
/// Font family name for the regular face, used for reading and control text.
pub const FONT_REGULAR: &str = "open-sans";

/// Physical spacing rhythm. All padding, gaps, and margins are multiples of
/// `SPACE_UNIT` so the interface keeps one consistent beat.
pub const SPACE_UNIT: f32 = 4.0;

/// Minimum corner radius for interactive controls.
pub const RADIUS_CONTROL: f32 = 6.0;

// ---------------------------------------------------------------------------
// The themes
// ---------------------------------------------------------------------------

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
// Typography
// ---------------------------------------------------------------------------

/// One font size per step of the scale. The sizes are the reference's own:
/// the logo at 18, the controls at 12, the labels at 11, the body at 13.
#[derive(Clone, Copy)]
pub enum TypeStep {
    /// Tiny meta text: counts, units, attribute labels.
    Micro,
    /// Secondary labels and attribute rows.
    Caption,
    /// Default reading and control size.
    Body,
    /// Emphasised labels inside controls.
    Subtitle,
    /// Panel headings and toolbar section titles.
    Title,
    /// App title and large readouts.
    Heading,
}

impl TypeStep {
    pub fn size(self) -> f32 {
        match self {
            TypeStep::Micro => 11.0,
            TypeStep::Caption => 11.0,
            TypeStep::Body => 13.0,
            TypeStep::Subtitle => 12.0,
            TypeStep::Title => 16.0,
            TypeStep::Heading => 18.0,
        }
    }
}

/// Font id for regular weight at a scale step.
pub fn font(step: TypeStep) -> FontId {
    FontId::new(step.size(), FontFamily::Proportional)
}

/// Font id for bold weight at a scale step.
pub fn font_bold(step: TypeStep) -> FontId {
    FontId::new(step.size(), FontFamily::Name(FONT_BOLD.into()))
}

/// Font id for monospace measurement data at a scale step.
pub fn font_mono(step: TypeStep) -> FontId {
    FontId::new(step.size(), FontFamily::Monospace)
}

// ---------------------------------------------------------------------------
// Color
// ---------------------------------------------------------------------------

/// A resolved color set for one theme.
///
/// The names are the roles the reference's own variables fill, so a value can
/// be traced back to the line of CSS it came from.
#[derive(Clone, Copy)]
pub struct Palette {
    // Surfaces, ordered from deepest to nearest the viewer.
    /// The page itself.
    pub surface_panel: Color32,
    /// The graph's ground.
    pub surface_base: Color32,
    /// The fields and the raised surfaces.
    pub surface_raised: Color32,
    /// The sunken surfaces: the code and the quiet fills.
    pub surface_sunken: Color32,

    // Text.
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_tertiary: Color32,
    pub text_disabled: Color32,

    // Accent: reserved for primary actions, current selection, and focus.
    pub accent: Color32,
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

    /// The palette for the light or the dark theme.
    pub fn resolve(dark: bool) -> Self {
        if dark {
            Self::dark()
        } else {
            Self::light()
        }
    }

    /// The light theme: the reference's default.
    pub fn light() -> Self {
        Self {
            surface_panel: hsl(54, 5, 96),
            surface_base: hsl(54, 0, 100),
            surface_raised: hsl(210, 10, 98),
            surface_sunken: hsl(54, 5, 90),

            text_primary: hsl(210, 20, 10),
            text_secondary: hsl(210, 20, 40),
            text_tertiary: hsl(210, 20, 55),
            text_disabled: hsl(210, 10, 70),

            // The reference's accent is hsl(210, 53%, 49%), which carries white
            // text at 4.3:1. The lightness is dropped a step so the white text
            // on an accent fill clears 4.5:1; the colour is otherwise the
            // reference's own.
            accent: hsl(210, 53, 46),
            accent_hover: hsl(210, 53, 40),
            accent_active: hsl(210, 53, 34),
            on_accent: hsl(0, 0, 100),

            border: hsl(54, 0, 82),
            border_strong: hsl(54, 0, 65),
            divider: hsl(54, 5, 88),
            hover_fill: hsl(54, 5, 90),
            active_fill: hsl(54, 5, 86),

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
            surface_panel: hsl(215, 14, 10),
            surface_base: hsl(215, 14, 7),
            surface_raised: hsl(215, 14, 5),
            surface_sunken: hsl(215, 14, 10),

            text_primary: hsl(54, 11, 87),
            text_secondary: hsl(215, 14, 60),
            text_tertiary: hsl(215, 14, 45),
            text_disabled: hsl(215, 14, 30),

            accent: Color32::from_rgb(0x38, 0x70, 0xA6),
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
            surface_panel: Color32::WHITE,
            surface_base: Color32::WHITE,
            surface_raised: Color32::WHITE,
            surface_sunken: Color32::BLACK,

            text_primary: Color32::BLACK,
            text_secondary: Color32::BLACK,
            text_tertiary: Color32::BLACK,
            text_disabled: Color32::from_rgb(0x60, 0x60, 0x60),

            accent: Color32::BLACK,
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

    /// Fill for a selected interactive control.
    pub fn selected_fill(&self) -> Color32 {
        self.accent
    }

    /// Stroke for a control border.
    pub fn control_stroke(&self) -> Stroke {
        Stroke::new(1.0, self.border)
    }

    /// Border on a control that currently has keyboard focus.
    pub fn focus_stroke(&self) -> Stroke {
        Stroke::new(1.5, self.accent)
    }

    /// Whether the theme is a dark one.
    pub fn is_dark(&self) -> bool {
        matches!(self.theme, Theme::Dark)
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
    // turn around the colour wheel; the sector only decides which of the three
    // sits at full strength.
    Color32::from_rgb(channel(h + 1.0 / 3.0), channel(h), channel(h - 1.0 / 3.0))
}

// ---------------------------------------------------------------------------
// Graph
// ---------------------------------------------------------------------------

/// Stroke weights and label sizes for the graph canvas.
///
/// The graph carries more saturated color than any other surface, so its own
/// chrome is the quietest thing in the application: a hairline grid, recessive
/// axis type, and bold curve labels that match the measurement colors.
pub mod graph {
    /// Width of the measurement and target curves.
    pub const CURVE_WIDTH: f32 = 2.0;
    /// Width of the hovered curve's emphasis stroke.
    pub const CURVE_WIDTH_HOVER: f32 = 4.0;
    /// Width of a grid line.
    pub const GRID_WIDTH: f32 = 0.7;
    /// Width of the inspector's vertical frequency line.
    pub const INSPECTOR_WIDTH: f32 = 1.5;
}

// ---------------------------------------------------------------------------
// Spacing
// ---------------------------------------------------------------------------

/// Spacing scale, all multiples of [`SPACE_UNIT`].
pub struct Spacing;

impl Spacing {
    pub const MICRO: f32 = SPACE_UNIT * 0.5;
    pub const TIGHT: f32 = SPACE_UNIT * 1.0;
    pub const COMPACT: f32 = SPACE_UNIT * 2.0;
    pub const NORMAL: f32 = SPACE_UNIT * 3.0;
    pub const RELAXED: f32 = SPACE_UNIT * 4.0;
    pub const LOOSE: f32 = SPACE_UNIT * 6.0;
    pub const SECTION: f32 = SPACE_UNIT * 8.0;
}

// ---------------------------------------------------------------------------
// Scrolling
// ---------------------------------------------------------------------------

/// Scrollbar geometry, applied through [`egui::style::Spacing::scroll`].
///
/// The reference themes its scrollbars from CSS custom properties: thin,
/// floating, and recoloured on hover. egui paints a scrollbar's handle from
/// the active widget visuals and its track from the extreme background, so the
/// themed equivalent is a floating bar with the handle drawn in the palette's
/// foreground stroke, which brightens on hover through the same widget states
/// every other control uses. Without this the scrollbars are stock egui grey,
/// which is a visible tell next to the rest of the interface.
pub mod scroll {
    use super::SPACE_UNIT;
    /// Width of the bar at rest.
    pub const BAR_WIDTH: f32 = 10.0;
    /// Thin width the floating bar collapses to when the pointer is elsewhere.
    pub const FLOATING_WIDTH: f32 = 4.0;
    /// Space always reserved for the floating bar, so it appears without
    /// covering content.
    pub const FLOATING_ALLOCATED_WIDTH: f32 = SPACE_UNIT;
    /// Shortest the handle is ever drawn, so a long list still shows a
    /// grabbable span.
    pub const HANDLE_MIN_LENGTH: f32 = 24.0;
    /// Gap between the content and the bar.
    pub const BAR_INNER_MARGIN: f32 = SPACE_UNIT * 0.5;

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
            // Dormant opacities keep the bar a hint rather than a permanent
            // stripe, and the hovered opacities answer the pointer arriving.
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

// ---------------------------------------------------------------------------
// Registration and application
// ---------------------------------------------------------------------------
/// Register the application fonts. Must be called once before the first frame.
pub fn register_fonts(ctx: &Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        FONT_REGULAR.to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(
            squiglab_core::product::FONT_REGULAR,
        )),
    );
    fonts.font_data.insert(
        FONT_BOLD.to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(
            squiglab_core::product::FONT_BOLD,
        )),
    );

    fonts.families.insert(
        FontFamily::Proportional,
        vec![FONT_REGULAR.to_owned(), FONT_BOLD.to_owned()],
    );
    fonts.families.insert(
        FontFamily::Name(FONT_BOLD.into()),
        vec![FONT_BOLD.to_owned(), FONT_REGULAR.to_owned()],
    );
    fonts.families.insert(
        FontFamily::Monospace,
        vec![FONT_REGULAR.to_owned(), FONT_BOLD.to_owned()],
    );

    ctx.set_fonts(fonts);
}

/// Apply the token set to the context for the active theme.
///
/// `Context::set_style_of` takes a whole [`Style`] per theme, so both themes are
/// installed and egui switches between them as the theme preference changes. This
/// must be the single place the application configures appearance: calling
/// `Context::set_visuals` anywhere else silently overrides the active theme.
///
/// Fonts are registered separately by [`register_fonts`] from the eframe
/// creation hook, because `set_fonts` is deferred to the next frame.
pub fn apply(ctx: &Context, theme: Theme) {
    set_current(theme);
    ctx.set_theme(match theme {
        Theme::Light | Theme::Contrast => egui::ThemePreference::Light,
        Theme::Dark => egui::ThemePreference::Dark,
    });
    ctx.set_style_of(egui::Theme::Dark, style_for(Theme::Dark));
    ctx.set_style_of(egui::Theme::Light, style_for(theme));
}

fn style_for(theme: Theme) -> Style {
    let palette = Palette::resolve_theme(theme);
    Style {
        text_styles: text_styles(),
        spacing: egui::Spacing {
            item_spacing: egui::vec2(Spacing::COMPACT, Spacing::COMPACT * 0.5),
            button_padding: egui::vec2(Spacing::COMPACT, Spacing::TIGHT),
            menu_margin: margin(Spacing::NORMAL, Spacing::COMPACT),
            window_margin: margin(Spacing::NORMAL, Spacing::NORMAL),
            indent: Spacing::COMPACT + Spacing::TIGHT,
            interact_size: egui::vec2(80.0, 22.0),
            combo_height: Spacing::COMPACT * 3.0,
            scroll: scroll::style(),
            ..Default::default()
        },
        visuals: visuals(&palette),
        ..Default::default()
    }
}

/// The core names colours in its own terms; the interface paints them.
pub fn color(color: squiglab_core::color::Rgb) -> egui::Color32 {
    egui::Color32::from_rgb(color.r, color.g, color.b)
}

/// The interface's own colour, named the way the core names colours.
pub fn core_color(color: egui::Color32) -> squiglab_core::color::Rgb {
    squiglab_core::color::Rgb::new(color.r(), color.g(), color.b())
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

/// Map each textual role onto the scale. `Body` and `Button` share a size because
/// `Button` is egui's fallback for plain labels, so keeping them equal stops
/// incidental labels from jumping a step. Emphasis comes from [`font_bold`],
/// size, and fill rather than from bolding everything.
fn text_styles() -> std::collections::BTreeMap<TextStyle, FontId> {
    use TextStyle::{Body, Button, Heading, Monospace, Small};
    [
        (Small, font(TypeStep::Micro)),
        (Body, font(TypeStep::Body)),
        (Button, font(TypeStep::Body)),
        (Heading, font(TypeStep::Title)),
        (Monospace, font_mono(TypeStep::Caption)),
        (
            TextStyle::Name("display".into()),
            font_bold(TypeStep::Heading),
        ),
        (
            TextStyle::Name("title".into()),
            font_bold(TypeStep::Subtitle),
        ),
        (TextStyle::Name("label".into()), font(TypeStep::Caption)),
        (TextStyle::Name("value".into()), font_mono(TypeStep::Body)),
        (TextStyle::Name("meta".into()), font(TypeStep::Micro)),
    ]
    .into_iter()
    .collect()
}

fn visuals(palette: &Palette) -> Visuals {
    let mut visuals = if palette.is_dark() {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    visuals.override_text_color = Some(palette.text_primary);
    visuals.panel_fill = palette.surface_panel;
    visuals.window_fill = palette.surface_raised;
    visuals.extreme_bg_color = palette.surface_base;
    visuals.faint_bg_color = palette.surface_sunken;
    visuals.code_bg_color = palette.surface_sunken;
    visuals.text_cursor.stroke.color = palette.accent;
    visuals.hyperlink_color = palette.accent;
    visuals.warn_fg_color = palette.warning;
    visuals.error_fg_color = palette.error;

    visuals.widgets = egui::style::Widgets {
        noninteractive: egui::style::WidgetVisuals {
            bg_fill: palette.surface_base,
            weak_bg_fill: palette.surface_base,
            bg_stroke: Stroke::new(1.0, palette.border),
            fg_stroke: Stroke::new(1.0, palette.text_primary),
            corner_radius: RADIUS_CONTROL.into(),
            expansion: 0.0,
        },
        inactive: egui::style::WidgetVisuals {
            bg_fill: palette.surface_raised,
            weak_bg_fill: palette.surface_raised,
            bg_stroke: Stroke::new(1.0, palette.border),
            fg_stroke: Stroke::new(1.0, palette.text_secondary),
            corner_radius: RADIUS_CONTROL.into(),
            expansion: 1.5,
        },
        hovered: egui::style::WidgetVisuals {
            bg_fill: palette.hover_fill,
            weak_bg_fill: palette.hover_fill,
            bg_stroke: Stroke::new(1.0, palette.border_strong),
            fg_stroke: Stroke::new(1.0, palette.text_primary),
            corner_radius: RADIUS_CONTROL.into(),
            expansion: 1.5,
        },
        active: egui::style::WidgetVisuals {
            bg_fill: palette.active_fill,
            weak_bg_fill: palette.active_fill,
            bg_stroke: Stroke::new(1.0, palette.accent),
            fg_stroke: Stroke::new(1.0, palette.text_primary),
            corner_radius: RADIUS_CONTROL.into(),
            expansion: 1.5,
        },
        open: egui::style::WidgetVisuals {
            bg_fill: palette.surface_raised,
            weak_bg_fill: palette.surface_raised,
            bg_stroke: Stroke::new(1.0, palette.accent),
            fg_stroke: Stroke::new(1.0, palette.text_primary),
            corner_radius: RADIUS_CONTROL.into(),
            expansion: 2.0,
        },
    };

    visuals.selection.bg_fill = palette.accent;
    visuals.selection.stroke = Stroke::new(1.0, palette.on_accent);
    visuals.widgets.inactive.fg_stroke.color = palette.text_secondary;
    visuals.widgets.hovered.fg_stroke.color = palette.text_primary;

    visuals
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
    fn body_text_clears_wcag_aa_on_every_surface() {
        for palette in [Palette::dark(), Palette::light()] {
            for surface in [
                palette.surface_base,
                palette.surface_panel,
                palette.surface_raised,
            ] {
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
        assert_eq!(TypeStep::Heading.size(), 18.0);
        assert_eq!(TypeStep::Subtitle.size(), 12.0);
        assert_eq!(TypeStep::Micro.size(), 11.0);
        assert_eq!(TypeStep::Caption.size(), 11.0);
        assert_eq!(TypeStep::Body.size(), 13.0);
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
    fn the_themes_are_the_reference_s_own() {
        // The light theme's page is the reference's warm off-white.
        assert_eq!(Palette::light().surface_panel, hsl(54, 5, 96));
        assert_eq!(Palette::light().surface_base, hsl(54, 0, 100));
        assert_eq!(Palette::light().accent, hsl(210, 53, 46));
        // The dark theme's accent is the native's blue.
        assert_eq!(Palette::dark().accent, Color32::from_rgb(0x38, 0x70, 0xA6));
        assert_eq!(Palette::dark().surface_panel, hsl(215, 14, 10));
        // The contrast theme is black on white.
        assert_eq!(Palette::contrast().surface_panel, Color32::WHITE);
        assert_eq!(Palette::contrast().text_primary, Color32::BLACK);
    }

    #[test]
    fn accent_stays_out_of_the_semantic_status_colors() {
        for palette in [Palette::dark(), Palette::light()] {
            assert_ne!(palette.accent, palette.warning);
            assert_ne!(palette.accent, palette.error);
            assert_ne!(palette.accent, palette.success);
        }
    }
}
