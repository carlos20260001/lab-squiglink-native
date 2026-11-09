//! Cursor management.
//!
//! Cursor icons are set through squiglab-platform's [`CursorIcon`](crate::events::CursorIcon)
//! and translated to Winit's `CursorIcon` by the backend.

#![allow(dead_code)]

use crate::events::CursorIcon;

/// Translates a squiglab [`CursorIcon`] to a Winit cursor icon.
pub fn cursor_icon_from_squiglab(icon: CursorIcon) -> winit::window::CursorIcon {
    match icon {
        CursorIcon::Default => winit::window::CursorIcon::Default,
        CursorIcon::Crosshair => winit::window::CursorIcon::Crosshair,
        CursorIcon::Hand => winit::window::CursorIcon::Pointer,
        CursorIcon::Text => winit::window::CursorIcon::Text,
        CursorIcon::ResizeHorizontal => winit::window::CursorIcon::ColResize,
        CursorIcon::ResizeVertical => winit::window::CursorIcon::RowResize,
        CursorIcon::Grab => winit::window::CursorIcon::Grab,
        CursorIcon::Grabbing => winit::window::CursorIcon::Grabbing,
        CursorIcon::None => winit::window::CursorIcon::Default,
    }
}

/// Translates a Winit cursor icon to a squiglab [`CursorIcon`].
pub fn cursor_icon_to_squiglab(icon: &winit::window::CursorIcon) -> CursorIcon {
    match icon {
        winit::window::CursorIcon::Default => CursorIcon::Default,
        winit::window::CursorIcon::Crosshair => CursorIcon::Crosshair,
        winit::window::CursorIcon::Pointer => CursorIcon::Hand,
        winit::window::CursorIcon::Text => CursorIcon::Text,
        winit::window::CursorIcon::ColResize => CursorIcon::ResizeHorizontal,
        winit::window::CursorIcon::RowResize => CursorIcon::ResizeVertical,
        winit::window::CursorIcon::Grab => CursorIcon::Grab,
        winit::window::CursorIcon::Grabbing => CursorIcon::Grabbing,
        _ => CursorIcon::Default,
    }
}
