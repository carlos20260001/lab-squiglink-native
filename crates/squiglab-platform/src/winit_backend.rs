//! winit backend for squiglab-platform.
//!
//! Translates winit `WindowEvent`s into [`PlatformRawEvent`]s, following
//! egui-winit's `State::on_window_event` pattern.
//!
//! This is a stub: `winit` is not in the offline lock file yet, so the
//! backend cannot be compiled. Enable the `winit` feature and add
//! `winit` to Cargo.toml to activate it.
//!
//! When implemented, this backend will handle:
//! * Keyboard input (key_down / key_up / modifiers)
//! * Pointer input (mouse move, button press/release, scroll)
//! * Touch input
//! * Window events (resize, focus, scale factor, theme)
//! * Drag and drop
//! * Clipboard (via arboard)
//! * IME input

use crate::{PlatformRawEvent, Pos2};

// Called once at application startup to register winit event handlers.
pub fn install_event_handlers() {
    log::debug!("winit backend: install_event_handlers not yet implemented");
}

/// Convert a winit `WindowEvent` to squiglab platform events.
pub fn window_event_to_platform_events() -> Vec<PlatformRawEvent> {
    Vec::new()
}

/// Convert a winit touch position to squiglab [`Pos2`].
pub fn touch_position(x: f32, y: f32) -> Pos2 {
    Pos2::new(x, y)
}
