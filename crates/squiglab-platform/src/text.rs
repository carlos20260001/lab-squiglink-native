//! Text support — fonts, IME, and text editing integration.
//!
//! GPUI owns text layout and shaping; squiglab-platform provides font
//! loading and IME integration behind this boundary.

#![allow(dead_code)]

use crate::ime::ImeState;

/// Text system state. Holds IME state and font references.
pub struct TextSystem {
    ime: ImeState,
}

impl TextSystem {
    pub fn new() -> Self {
        Self {
            ime: ImeState::new(),
        }
    }

    /// Get the IME state.
    pub fn ime(&self) -> &ImeState {
        &self.ime
    }

    /// Get the IME state mutably.
    pub fn ime_mut(&mut self) -> &mut ImeState {
        &mut self.ime
    }

    /// Enable or disable IME input.
    pub fn set_ime_enabled(&mut self, enabled: bool) {
        self.ime.set_enabled(enabled);
    }

    /// Set the IME composition area and purpose.
    pub fn set_ime_composition(
        &mut self,
        rect: crate::events::Rect,
        purpose: crate::events::ImePurpose,
    ) {
        self.ime.set_composition(rect, purpose);
    }

    /// Clear the IME composition.
    pub fn clear_ime(&mut self) {
        self.ime.clear();
    }
}

impl Default for TextSystem {
    fn default() -> Self {
        Self::new()
    }
}
