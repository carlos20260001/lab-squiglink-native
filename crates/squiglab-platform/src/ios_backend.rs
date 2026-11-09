//! iOS backend for squiglab-platform.
//!
//! Translates iOS objc2 events (UIApplication, UIDocumentPickerViewController,
//! touch events) into [`PlatformRawEvent`]s, following egui-winit's iOS pattern.
//!
//! This is compiled only on iOS with the `ios` feature.

use crate::{PlatformRawEvent, Pos2};

// Called once at application startup to set up the iOS document picker
// delegate and event handling.
pub fn install_event_handlers() {
    log::debug!("ios backend: install_event_handlers not yet implemented");
}

/// Convert an iOS UI event to squiglab platform events.
pub fn ui_event_to_platform_events() -> Vec<PlatformRawEvent> {
    Vec::new()
}

/// Convert an iOS touch position to squiglab [`Pos2`].
pub fn touch_position(x: f32, y: f32) -> Pos2 {
    Pos2::new(x, y)
}
