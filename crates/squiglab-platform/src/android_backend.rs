//! Android backend for squiglab-platform.
//!
//! Translates Android JNI / NDK events (AInputQueue, AAssetManager,
//! native activity) into [`PlatformRawEvent`]s, following egui-winit's
//! Android pattern.
//!
//! This is compiled only on Android with the `android` feature.

use crate::{PlatformRawEvent, Pos2};

// Called once at application startup to set up the native activity
// input queue and asset manager.
pub fn install_event_handlers() {
    log::debug!("android backend: install_event_handlers not yet implemented");
}

/// Convert an Android input event to squiglab platform events.
pub fn input_event_to_platform_events() -> Vec<PlatformRawEvent> {
    Vec::new()
}

/// Convert an Android touch position to squiglab [`Pos2`].
pub fn touch_position(x: f32, y: f32) -> Pos2 {
    Pos2::new(x, y)
}
