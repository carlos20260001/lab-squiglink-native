//! CrinGraph design tokens: typography, color, spacing, and state styling.
//!
//! Every UI decision in the application resolves to a token defined here. The
//! graph renders its own saturated D3 palette, so the chrome around it must stay
//! strictly neutral: the only saturated color in the interface is the accent,
//! which is reserved for primary actions and current selection.
//!
//! Type scale follows a fixed 1.15 ratio for product UI rather than a brand
//! surface ratio, and the family is Open Sans (regular / bold) with Hack for
//! monospace data. Open Sans is the family the reference web tool declares and
//! renders with: 400 for interface text, 700 for curve labels. Hack is
//! deliberate: dB, Hz, and phon values are measurements, so tabular figures
//! keep them aligned in columns.

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
pub const RADIUS_CONTROL: f32 = 4.0;

// ---------------------------------------------------------------------------
// Typography
// ---------------------------------------------------------------------------

/// One font size per step of the scale. Steps are ~1.15 apart so adjacent steps
/// read as clearly different without creating noise in a dense tool.
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
            TypeStep::Micro => 10.0,
            TypeStep::Caption => 11.5,
            TypeStep::Body => 13.0,
            TypeStep::Subtitle => 15.0,
            TypeStep::Title => 17.5,
            TypeStep::Heading => 20.0,
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

macro_rules! palette {
    ($($name:ident => $dark:expr, $light:expr;)*) => {
        /// A resolved color set for one theme.
        #[derive(Clone, Copy)]
        pub struct Palette {
            $(pub $name: Color32,)*
            pub dark: bool,
        }

        impl Palette {
            /// Palette for the active theme.
            pub fn resolve(dark: bool) -> Self {
                if dark { Self::dark() } else { Self::light() }
            }

            /// Dark theme palette, the default for this tool: users measure and
            /// compare curves in a dark room or for long sessions, and the graph
            /// palette reads better against a dark ground.
            pub fn dark() -> Self {
                Self {
                    dark: true,
                    $($name: $dark,)*
                }
            }

            pub fn light() -> Self {
                Self {
                    dark: false,
                    $($name: $light,)*
                }
            }
        }
    };
}

palette! {
    // Surfaces, ordered from deepest to nearest the viewer.
    surface_sunken => Color32::from_rgb(0x0F, 0x0F, 0x0F), Color32::from_rgb(0xE4, 0xE8, 0xEE);
    surface_base    => Color32::from_rgb(0x12, 0x12, 0x12), Color32::from_rgb(0xF6, 0xF7, 0xF9);
    surface_panel   => Color32::from_rgb(0x1E, 0x1E, 0x1E), Color32::from_rgb(0xEE, 0xF0, 0xF4);
    surface_raised  => Color32::from_rgb(0x2A, 0x2A, 0x2A), Color32::from_rgb(0xFF, 0xFF, 0xFF);

    // Text. Every step clears 4.5:1 on its own surface except tertiary, which is
    // reserved for labels that never carry information alone.
    text_primary    => Color32::from_rgb(0xE9, 0xEB, 0xEE), Color32::from_rgb(0x12, 0x15, 0x1A);
    text_secondary  => Color32::from_rgb(0xA6, 0xAC, 0xB8), Color32::from_rgb(0x4A, 0x52, 0x60);
    text_tertiary   => Color32::from_rgb(0x84, 0x8B, 0x98), Color32::from_rgb(0x5F, 0x67, 0x75);
    text_disabled   => Color32::from_rgb(0x4A, 0x50, 0x59), Color32::from_rgb(0x9A, 0xA1, 0xAD);

    // Accent: calm blue, matching the web tool. Reserved for primary actions,
    // current selection, and focus. Never decoration. The dark accent is
    // deliberately dimmer than it looks right at, because white text sitting on
    // an accent fill has to clear 4.5:1 and the brighter value does not.
    accent          => Color32::from_rgb(0x38, 0x70, 0xA6), Color32::from_rgb(0x2B, 0x6C, 0xB0);
    accent_hover    => Color32::from_rgb(0x42, 0x7F, 0xB9), Color32::from_rgb(0x33, 0x7B, 0xC4);
    accent_active   => Color32::from_rgb(0x2E, 0x5F, 0x8D), Color32::from_rgb(0x23, 0x5A, 0x99);
    on_accent       => Color32::from_rgb(0xFF, 0xFF, 0xFF), Color32::from_rgb(0xFF, 0xFF, 0xFF);

    // Structural strokes and fills.
    border          => Color32::from_rgb(0x2E, 0x33, 0x3C), Color32::from_rgb(0xD8, 0xDD, 0xE4);
    border_strong   => Color32::from_rgb(0x3C, 0x42, 0x4D), Color32::from_rgb(0xB4, 0xBC, 0xC7);
    divider         => Color32::from_rgb(0x22, 0x25, 0x2B), Color32::from_rgb(0xDE, 0xE2, 0xE8);
    hover_fill      => Color32::from_rgb(0x2C, 0x31, 0x3A), Color32::from_rgb(0xE9, 0xEC, 0xF1);
    active_fill     => Color32::from_rgb(0x33, 0x38, 0x42), Color32::from_rgb(0xDF, 0xE4, 0xEC);

    // Semantic status, used for imbalance and validation.
    warning         => Color32::from_rgb(0xE0, 0xA8, 0x40), Color32::from_rgb(0xA6, 0x6E, 0x10);
    error           => Color32::from_rgb(0xD9, 0x6B, 0x6B), Color32::from_rgb(0xB0, 0x38, 0x38);
    success         => Color32::from_rgb(0x5E, 0xB8, 0x7A), Color32::from_rgb(0x2E, 0x7D, 0x4E);
}

impl Palette {
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
/// The reference tool themes its scrollbars from CSS custom properties: thin,
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

    /// The themed scrollbar style, shared by both palettes.
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
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/OpenSans-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        FONT_BOLD.to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/OpenSans-Bold.ttf"
        ))),
    );

    fonts.families.insert(
        FontFamily::Proportional,
        vec![
            FONT_REGULAR.to_owned(),
            FONT_BOLD.to_owned(),
            "Ubuntu-Light".to_owned(),
            "Hack".to_owned(),
        ],
    );
    fonts.families.insert(
        FontFamily::Name(FONT_BOLD.into()),
        vec![
            FONT_BOLD.to_owned(),
            FONT_REGULAR.to_owned(),
            "Hack".to_owned(),
        ],
    );
    fonts.families.insert(
        FontFamily::Monospace,
        vec![
            "Hack".to_owned(),
            FONT_REGULAR.to_owned(),
            FONT_BOLD.to_owned(),
        ],
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
pub fn apply(ctx: &Context, dark: bool) {
    if dark {
        ctx.set_theme(egui::ThemePreference::Dark);
    } else {
        ctx.set_theme(egui::ThemePreference::Light);
    }
    ctx.set_style_of(egui::Theme::Dark, style_for(true));
    ctx.set_style_of(egui::Theme::Light, style_for(false));
}

fn style_for(dark: bool) -> Style {
    let palette = Palette::resolve(dark);
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
    let mut visuals = if palette.dark {
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
                    palette.dark
                );
                let secondary = contrast(palette.text_secondary, surface);
                assert!(
                    secondary >= 4.5,
                    "secondary text on surface fell to {secondary} (dark: {})",
                    palette.dark
                );
            }
        }
    }

    #[test]
    fn accent_text_clears_aa_on_its_fill() {
        for palette in [Palette::dark(), Palette::light()] {
            let ratio = contrast(palette.on_accent, palette.accent);
            assert!(ratio >= 4.5, "on_accent fell to {ratio}");
        }
    }

    #[test]
    fn type_scale_steps_are_ordered_and_fixed() {
        let steps = [
            TypeStep::Micro.size(),
            TypeStep::Caption.size(),
            TypeStep::Body.size(),
            TypeStep::Subtitle.size(),
            TypeStep::Title.size(),
            TypeStep::Heading.size(),
        ];
        for pair in steps.windows(2) {
            let ratio = pair[1] / pair[0];
            assert!(
                (1.1..=1.25).contains(&ratio),
                "scale step ratio {ratio} outside the product range"
            );
        }
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
