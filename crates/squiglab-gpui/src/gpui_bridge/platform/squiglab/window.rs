//! GPUI Window bridge.
//!
//! Adapts squiglab-platform's Winit window to GPUI's window requirements:
//! title, size, focus, visibility, theme, cursor, and IME.
//!
//! Windows are created through the `PlatformBuilder` before `run()`;
//! Winit creates them when the event loop's `resumed()` handler fires.

#![allow(dead_code)]

use squiglab_platform::window::{Window, WindowAttributes, WindowId};
use squiglab_platform::PlatformBuilder;

/// Queue a window for creation when the event loop starts.
/// Returns the window's identifier.
pub fn create_window(
    builder: &mut PlatformBuilder,
    title: &str,
    width: f32,
    height: f32,
) -> WindowId {
    let attrs = WindowAttributes {
        title: title.to_string(),
        inner_size: Some((width, height)),
        ..Default::default()
    };
    builder.create_window(attrs)
}
