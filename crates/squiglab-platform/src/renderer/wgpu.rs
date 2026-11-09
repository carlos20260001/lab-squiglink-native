//! wgpu-based renderer implementation.
//!
//! Adapts GPUI scenes to wgpu primitives. Compiled only when the `renderer`
//! feature is enabled and wgpu can build for the target.

#![allow(dead_code)]

use crate::renderer::Renderer;

/// The wgpu renderer. Holds the device, queue, surface, and resources.
pub struct WgpuRenderer;

impl WgpuRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Renderer for WgpuRenderer {
    fn begin_frame(&mut self) {}

    fn end_frame(&mut self) {}

    fn resize(&mut self, _width: u32, _height: u32) {}

    fn window_handle(&self) -> Option<raw_window_handle::RawWindowHandle> {
        None
    }
}
