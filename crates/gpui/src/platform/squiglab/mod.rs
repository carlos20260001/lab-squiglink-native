//! squiglab platform bridge for GPUI.
//!
//! Adapts squiglab-platform's Winit-based windowing and input to GPUI's
//! platform requirements. Each file corresponds to a GPUI platform concern:
//!
//! - application.rs — GPUI Application lifecycle (run, activate, quit)
//! - window.rs — GPUI Window management (create, resize, focus, title)
//! - renderer.rs — GPUI scene rendering (adapt scenes to wgpu)
//! - input.rs — GPUI input events (keyboard, pointer, IME, touch)
//! - text_system.rs — GPUI text layout and glyph rendering

#![allow(dead_code)]

pub mod application;
pub mod input;
pub mod renderer;
pub mod text_system;
pub mod window;

pub use application::{SquiglabPlatform, SquiglabDispatcher};

/// Platform-specific screen capture frame type.
/// Not yet implemented for squiglab.
pub(crate) type PlatformScreenCaptureFrame = ();
