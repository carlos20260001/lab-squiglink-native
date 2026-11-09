//! GPUI renderer bridge.
//!
//! Adapts GPUI scene rendering to squiglab-platform's shared wgpu renderer.
//! GPUI generates scenes; the bridge translates them into wgpu draw calls.

#![allow(dead_code)]

use squiglab_platform::renderer::Renderer;

/// Renders a GPUI scene to the squiglab-platform renderer.
pub fn render_scene(renderer: &mut dyn Renderer) {
    renderer.begin_frame();
    // GPUI scene adaptation would go here when wgpu rendering is wired.
    renderer.end_frame();
}
