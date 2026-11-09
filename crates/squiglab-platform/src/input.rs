//! Input state — accumulates input for one frame.
//!
//! This is the per-frame accumulation layer between raw platform events
//! and PlatformInput.

use crate::events::PlatformInput;

/// Holds input state for a single frame.
#[derive(Clone, Debug, Default)]
pub struct InputState {
    /// The accumulated input for this frame.
    pub input: PlatformInput,
}

impl InputState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Take the accumulated input, starting a new frame.
    pub fn take(&mut self, screen_rect: Option<crate::events::Rect>) -> PlatformInput {
        let mut input = std::mem::take(&mut self.input);
        input.screen_rect = screen_rect;
        input
    }
}
