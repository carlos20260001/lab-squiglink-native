//! GPUI Window bridge.
//!
//! Adapts squiglab-platform's Winit window to GPUI's window requirements:
//! title, size, focus, visibility, theme, cursor, and IME.

#![allow(dead_code)]

use squiglab_platform::{
    window::{Window, WindowAttributes},
    Platform,
};

/// Creates a squiglab-platform window from GPUI window parameters.
pub fn create_window(
    platform: &mut dyn Platform,
    title: &str,
    width: f32,
    height: f32,
) -> Box<dyn Window> {
    let attrs = WindowAttributes {
        title: title.to_string(),
        inner_size: Some((width, height)),
        ..Default::default()
    };
    platform.create_window(&attrs)
}
