//! GPUI renderer bridge for squiglab.
//!
//! Adapts GPUI scene rendering to squiglab-platform's shared wgpu renderer.
//! GPUI generates scenes; the bridge translates them into wgpu draw calls.

use squiglab_platform::renderer::Renderer as PlatformRenderer;

/// Renders a GPUI scene to the squiglab-platform renderer.
pub fn render_scene(platform_renderer: &mut dyn PlatformRenderer, _scene: &gpui::Scene) {
    platform_renderer.begin_frame();
    // GPUI scene adaptation would go here when wgpu rendering is wired.
    platform_renderer.end_frame();
}
