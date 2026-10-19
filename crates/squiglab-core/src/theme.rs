//! The theme the user chose.
//!
//! The choice is a preference, so it lives in the core with the rest of them;
//! the colours each theme draws with belong to the interface, which is the
//! only thing that knows what a colour is for.

use serde::{Deserialize, Serialize};

/// The themes the program offers.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// A light theme, for a bright room.
    Light,
    /// The dark theme, for a dim one.
    #[default]
    Dark,
    /// A high contrast theme, for a screen that is hard to read.
    Contrast,
}

impl Theme {
    /// The next theme, in the order the picker lists them.
    pub fn next(self) -> Self {
        match self {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Contrast,
            Theme::Contrast => Theme::Light,
        }
    }

    /// Whether the theme is a dark one.
    pub fn is_dark(self) -> bool {
        matches!(self, Theme::Dark)
    }

    /// The theme for a light or a dark ground.
    pub fn from_dark(dark: bool) -> Self {
        if dark {
            Theme::Dark
        } else {
            Theme::Light
        }
    }

    /// A short name for the picker.
    pub fn name(self) -> &'static str {
        match self {
            Theme::Light => "Light",
            Theme::Dark => "Dark",
            Theme::Contrast => "Contrast",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_themes_turn_over() {
        assert_eq!(Theme::Light.next(), Theme::Dark);
        assert_eq!(Theme::Dark.next(), Theme::Contrast);
        assert_eq!(Theme::Contrast.next(), Theme::Light);
    }

    #[test]
    fn only_the_dark_theme_is_dark() {
        assert!(Theme::Dark.is_dark());
        assert!(!Theme::Light.is_dark());
        assert!(!Theme::Contrast.is_dark());
    }

    #[test]
    fn the_theme_survives_a_round_trip_through_the_config() {
        for theme in [Theme::Light, Theme::Dark, Theme::Contrast] {
            let text = serde_json::to_string(&theme).expect("the theme serializes");
            let back: Theme = serde_json::from_str(&text).expect("the theme reads back");
            assert_eq!(back, theme);
        }
    }
}
