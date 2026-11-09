//! GPUI input bridge.
//!
//! Adapts squiglab-platform's Winit-translated input events to GPUI's input
//! model: keyboard, pointer, touch, scroll, IME, and focus.

#![allow(dead_code)]

use squiglab_platform::PlatformRawEvent;

/// Processes a raw platform event into GPUI-compatible input.
pub fn process_input_event(event: &PlatformRawEvent) {
    // Input translation is handled by squiglab-platform's PlatformState.
    // GPUI receives events through its own input dispatcher.
    let _ = event;
}
