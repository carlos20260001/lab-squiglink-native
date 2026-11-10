//! GPUI text system bridge for squiglab.
//!
//! Adapts GPUI's text layout and glyph rendering to squiglab-platform's
//! text capabilities. GPUI owns text layout; squiglab-platform provides
//! font loading and IME integration.

use std::borrow::Cow;
use std::sync::Arc;

use gpui::{
    Font, FontId, FontMetrics, GlyphId, LineLayout, Pixels, PlatformTextSystem, RenderGlyphParams,
    ShapedRun, Size,
};

/// Text system backed by GPUI's NoopTextSystem.
pub struct SquiglabTextSystem {
    inner: Arc<dyn PlatformTextSystem>,
}

impl SquiglabTextSystem {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(gpui::NoopTextSystem),
        }
    }
}
