//! IME (input method editor) integration.
//!
//! Preserves GPUI's text-editing semantics: Unicode, selection, clipboard
//! and IME composition. Does not translate raw keyboard events directly
//! into characters — uses Winit's IME events where available.

#![allow(dead_code)]

use crate::events::{Ime, ImePurpose, Rect};

/// IME state for a window.
pub struct ImeState {
    /// Whether IME input is currently enabled.
    pub enabled: bool,
    /// The current composition rect and purpose.
    pub composition: Option<Ime>,
}

impl ImeState {
    pub fn new() -> Self {
        Self {
            enabled: false,
            composition: None,
        }
    }

    /// Enable or disable IME input.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Set the IME composition area and purpose.
    pub fn set_composition(&mut self, rect: Rect, purpose: ImePurpose) {
        self.composition = Some(Ime {
            rect,
            purpose,
            should_interrupt_composition: false,
        });
    }

    /// Clear the IME composition.
    pub fn clear(&mut self) {
        self.composition = None;
    }
}
