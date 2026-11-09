//! Display / monitor information.
//!
//! Provides display metrics (scale factor, size, etc.) for platform scaling.

#![allow(dead_code)]

use crate::events::Vec2;

/// A display (monitor).
#[derive(Clone, Debug)]
pub struct Display {
    /// The display's identifier.
    pub id: u64,
    /// The display's name, if available.
    pub name: Option<String>,
    /// The display's size in physical pixels.
    pub size: Vec2,
    /// The display's scale factor (DPR).
    pub scale_factor: f32,
    /// Whether this is the primary display.
    pub is_primary: bool,
}

impl Display {
    pub fn new(
        id: u64,
        name: Option<String>,
        size: Vec2,
        scale_factor: f32,
        is_primary: bool,
    ) -> Self {
        Self {
            id,
            name,
            size,
            scale_factor,
            is_primary,
        }
    }
}

/// Returns the list of available displays.
pub fn displays() -> Vec<Display> {
    // In practice this queries Winit's `available_monitors()`.
    // For now return a single default display.
    vec![Display::new(0, None, Vec2::new(1440.0, 900.0), 1.0, true)]
}

/// Returns the primary display.
pub fn primary_display() -> Option<Display> {
    displays().into_iter().find(|d| d.is_primary)
}
