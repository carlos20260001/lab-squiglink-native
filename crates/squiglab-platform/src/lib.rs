//! squiglab-platform — shared compatibility glue between GPUI and the
//! underlying cross-platform runtime.
//!
//! ```text
//!   GPUI 0.2.2
//!       |
//!       v
//! squiglab-platform  (this crate)
//!       |
//!       v
//!     Winit  (windowing, event loop, input)
//!       |
//!   +---+---+---+
//!   |   |   | |
//! Lin Win Mac Wasm
//!
//! egui-winit is used only for permitted platform integration — never for
//! egui widgets, layout, or rendering. GPUI remains the only UI framework.
//!
//! # Architecture
//!
//! ```text
//!   platform event (winit WindowEvent / web DOM event)
//!         │
//!         ▼
//!   Platform::on_event()  ──►  PlatformEvent
//!         │
//!         ▼
//!   PlatformInput (accumulated per frame)
//!         │
//!         ▼
//!   Platform::take_input() ──► squiglab-core / squiglab-gpui
//! ```
use std::time::Instant;

pub use self::capabilities::PlatformCapabilities;
pub use self::cursor::{cursor_icon_from_squiglab, cursor_icon_to_squiglab};
pub use self::display::{displays, primary_display, Display};
pub use self::events::{
    CursorIcon, DroppedFile, Ime, ImePurpose, Key, KeyEvent, KeyModifiers, Modifiers,
    MouseWheelUnit, PlatformEvent, PlatformInput, PlatformOutput, PlatformOutputCommand,
    PlatformRawEvent, PointerButton, Pos2, Rect, Theme, TouchPhase, Vec2, ViewportId,
};
pub use self::ime::ImeState;
pub use self::platform::{Platform, PlatformBuilder, PlatformState};
pub use self::renderer::Renderer;
pub use self::text::TextSystem;
pub use self::window::{Window, WindowAttributes, WindowId};

// ---------------------------------------------------------------------------
// Modules
//

mod backends;
mod capabilities;
mod clipboard;
mod cursor;
mod display;
mod event_loop;
mod events;
mod ime;
mod input;
mod keyboard;
mod platform;
mod pointer;
pub mod renderer;
mod text;
pub mod window;

// Re-export submodules for internal use.
pub(crate) use event_loop::EventLoop;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use event_loop::EventLoopApp;
pub(crate) use input::InputState;
pub(crate) use keyboard::KeyboardState;
pub(crate) use pointer::PointerState;

// ---------------------------------------------------------------------------
// Global application state (used by the web backend and wasm-bindgen exports).
//

/// Returns the elapsed time since the application started.
pub fn start_time() -> Instant {
    Instant::now()
}

// ---------------------------------------------------------------------------
// Tests
//

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_input_default() {
        let input = PlatformInput::default();
        assert!(input.events.is_empty());
        assert_eq!(input.modifiers, Modifiers::default());
    }

    #[test]
    fn rect_width_height() {
        let rect = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(100.0, 200.0));
        assert_eq!(rect.width(), 100.0);
        assert_eq!(rect.height(), 200.0);
        assert_eq!(rect.max, Pos2::new(110.0, 220.0));
    }

    #[test]
    fn capabilities_default() {
        let caps = PlatformCapabilities::default();
        assert!(caps.clipboard);
        assert!(caps.ime);
        assert!(caps.drag_and_drop);
        assert!(!caps.fullscreen); // web cannot fullscreen without user gesture
    }
}
