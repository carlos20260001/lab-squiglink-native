//! Pointer state tracking.
//!
//! Tracks the current pointer position and button state.

use crate::events::{PointerButton, Pos2};

/// Tracks pointer state for input translation.
#[derive(Clone, Debug, Default)]
pub struct PointerState {
    /// The current pointer position in points.
    pub position: Option<Pos2>,
    /// Whether any pointer button is currently down.
    pub any_button_down: bool,
    /// The currently pressed buttons.
    pub pressed_buttons: std::collections::HashSet<PointerButton>,
}

impl PointerState {
    pub fn new() -> Self {
        Self {
            position: None,
            any_button_down: false,
            pressed_buttons: std::collections::HashSet::new(),
        }
    }

    /// Update the pointer position.
    pub fn set_position(&mut self, pos: Pos2) {
        self.position = Some(pos);
    }

    /// Handle a pointer button event.
    pub fn button_event(
        &mut self,
        button: PointerButton,
        pressed: bool,
    ) -> crate::events::PlatformRawEvent {
        if pressed {
            self.pressed_buttons.insert(button);
            self.any_button_down = true;
        } else {
            self.pressed_buttons.remove(&button);
            self.any_button_down = !self.pressed_buttons.is_empty();
        }
        let pos = self.position.unwrap_or(Pos2::ZERO);
        crate::events::PlatformRawEvent::PointerButton {
            pos,
            button,
            pressed,
            modifiers: crate::events::Modifiers::default(),
        }
    }
}
