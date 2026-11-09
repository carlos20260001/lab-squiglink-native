//! GPU surface management.
//!
//! Wraps wgpu's Surface for presenting rendered frames to the window.

#![allow(dead_code)]

/// A GPU surface backed by a native window or browser canvas.
pub struct Surface;

impl Surface {
    /// Create a new surface for a window.
    pub fn new() -> Self {
        Self
    }

    /// Configure the surface with device, queue, and surface config.
    pub fn configure(&mut self, _width: u32, _height: u32) {}

    /// Present the current frame.
    pub fn present(&mut self) {}
}
