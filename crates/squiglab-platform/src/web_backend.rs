//! Web backend for squiglab-platform.
//!
//! Translates web DOM events (via `web_sys`) into [`PlatformRawEvent`]s,
//! following eframe's web backend pattern.
//!
//! This is compiled only on `wasm32` with the `web` feature.

use crate::{PlatformRawEvent, Pos2, Vec2};

// Installs DOM event listeners on the canvas and translates them into
// squiglab platform events. Called once at application startup.
pub fn install_event_handlers() {
    // TODO: implement using web_sys event listeners, following
    // eframe's web/events.rs pattern (pointer, keyboard, touch, wheel,
    // clipboard, drag-and-drop, focus/blur, resize).
    log::debug!("web backend: install_event_handlers not yet implemented");
}

/// Convert a web key string to a squiglab [`Key`].
pub fn translate_key(key: &str) -> Option<crate::Key> {
    let _ = key;
    None
}

/// Convert a web mouse event position to squiglab [`Pos2`].
pub fn pos_from_mouse_event(client_x: f32, client_y: f32) -> Pos2 {
    Pos2::new(client_x, client_y)
}

/// Convert a web wheel delta to squiglab [`Vec2`] and [`MouseWheelUnit`].
pub fn wheel_delta(delta_x: f32, delta_y: f32, unit: crate::MouseWheelUnit) -> Vec2 {
    let _ = unit;
    Vec2::new(delta_x, delta_y)
}
