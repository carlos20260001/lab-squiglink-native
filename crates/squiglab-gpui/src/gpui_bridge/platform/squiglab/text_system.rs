//! GPUI text system bridge.
//!
//! Adapts GPUI's text layout and glyph rendering to squiglab-platform's
//! text capabilities. GPUI owns text layout; squiglab-platform provides
//! font loading and IME integration.

#![allow(dead_code)]

/// Text system placeholder. GPUI's internal text system handles layout
/// and shaping; squiglab-platform provides font and IME support.
pub struct SquiglabTextSystem;

impl SquiglabTextSystem {
    pub fn new() -> Self {
        Self
    }
}
