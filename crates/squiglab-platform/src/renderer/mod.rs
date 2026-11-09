//! Shared GPU renderer abstraction.
//!
//! wgpu provides cross-platform GPU rendering. The renderer adapts GPUI's
//! scene and drawing contracts to wgpu primitives.
//!
//! Preserved capabilities: text and glyph atlases, textures, images, SVG,
//! rectangles, rounded corners, borders, clipping, opacity, shadows,
//! layer ordering, scrolling, fractional scaling.

#![allow(dead_code)]

pub mod resources;
pub mod surface;

#[cfg(feature = "renderer")]
pub mod wgpu;

/// The renderer trait. GPUI draws through this interface.
pub trait Renderer: Send + Sync {
    /// Begin a new frame.
    fn begin_frame(&mut self);

    /// End the frame and present to the surface.
    fn end_frame(&mut self);

    /// Resize the rendering surface.
    fn resize(&mut self, width: u32, height: u32);

    /// Get the native window handle for surface creation.
    fn window_handle(&self) -> Option<raw_window_handle::RawWindowHandle>;
}

/// A null renderer used when wgpu is not available (e.g. without the
/// `renderer` feature, or when wgpu cannot compile for the target).
pub struct NullRenderer;

impl Renderer for NullRenderer {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn resize(&mut self, _width: u32, _height: u32) {}
    fn window_handle(&self) -> Option<raw_window_handle::RawWindowHandle> {
        None
    }
}
