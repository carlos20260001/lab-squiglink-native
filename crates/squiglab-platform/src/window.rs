//! Window abstraction over Winit.
//!
//! Provides a squiglab-specific window type that wraps Winit's `Window`
//! and exposes the platform operations GPUI needs (title, size, focus,
//! cursor, IME, etc.) without exposing Winit types to GPUI application code.

#![allow(dead_code)]

use crate::events::{Rect, Theme};

/// An identifier for a window / viewport.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(pub u64);

impl WindowId {
    pub const ROOT: Self = Self(0);
}

/// Attributes for creating a new window.
#[derive(Clone, Debug)]
pub struct WindowAttributes {
    pub title: String,
    pub inner_size: Option<(f32, f32)>,
    pub min_inner_size: Option<(f32, f32)>,
    pub resizable: bool,
    pub transparent: bool,
}

impl Default for WindowAttributes {
    fn default() -> Self {
        Self {
            title: "squiglab".to_string(),
            inner_size: None,
            min_inner_size: None,
            resizable: true,
            transparent: false,
        }
    }
}

/// The window abstraction. One per viewport.
pub trait Window: Send + Sync {
    /// The window's identifier.
    fn id(&self) -> WindowId;

    /// Set the window title.
    fn set_title(&self, title: &str);

    /// Set the window's inner size in points.
    fn set_inner_size(&self, size: crate::events::Vec2);

    /// Set whether the window is resizable.
    fn set_resizable(&self, resizable: bool);

    /// Request the window's attention (focus).
    fn set_focus(&self);

    /// Show or hide the window.
    fn set_visible(&self, visible: bool);

    /// Maximize or restore the window.
    fn set_maximized(&self, maximized: bool);

    /// Minimize or restore the window.
    fn set_minimized(&self, minimized: bool);

    /// Set the window theme.
    fn set_theme(&self, theme: Theme);

    /// Set the cursor icon.
    fn set_cursor_icon(&self, icon: crate::events::CursorIcon);

    /// Set IME allowed state.
    fn set_ime_allowed(&self, allowed: bool);

    /// Set the IME cursor rectangle and purpose.
    fn set_ime_cursor_area(&self, rect: Rect, purpose: crate::events::ImePurpose);

    /// Get the window's inner size in physical pixels.
    fn inner_size(&self) -> (u32, u32);

    /// Get the scale factor (DPR).
    fn scale_factor(&self) -> f32;

    /// Request a redraw.
    fn request_redraw(&self);
}
