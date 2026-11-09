//! Keyboard state tracking.
//!
//! Tracks the current modifier state and key state across frames.

use crate::events::{Key, KeyEvent, Modifiers};

/// Tracks keyboard state for input translation.
#[derive(Clone, Debug, Default)]
pub struct KeyboardState {
    /// The currently pressed keys.
    pub pressed_keys: std::collections::HashSet<Key>,
    /// The current modifier state.
    pub modifiers: Modifiers,
}

impl KeyboardState {
    pub fn new() -> Self {
        Self {
            pressed_keys: std::collections::HashSet::new(),
            modifiers: Modifiers::default(),
        }
    }

    /// Process a key event and return the translated PlatformRawEvent.
    pub fn process_key_event(&mut self, event: KeyEvent) -> crate::events::PlatformRawEvent {
        if event.pressed {
            self.pressed_keys.insert(event.key);
        } else {
            self.pressed_keys.remove(&event.key);
        }
        self.modifiers = event.modifiers;
        crate::events::PlatformRawEvent::Key(event)
    }
}
