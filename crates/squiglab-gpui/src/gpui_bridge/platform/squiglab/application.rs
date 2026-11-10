//! GPUI Application bridge.
//!
//! Adapts GPUI's application lifecycle (run, activate, quit, open_window) to
//! squiglab-platform's Winit-based event loop.
//!
//! GPUI's `Platform` trait is `pub(crate)`, so this module defines the bridge
//! interface. When GPUI is vendored, the implementation lives in
//! `gpui/src/platform/squiglab/application.rs`.

#![allow(dead_code)]

use squiglab_platform::{Platform, PlatformBuilder, WindowAttributes};

/// The GPUI application bridge. Holds the squiglab platform and provides
/// the entry point that GPUI's `Application::run` expects.
///
/// Windows are queued via [`SquiglabApplication::create_window`] before
/// [`SquiglabApplication::run`] starts the Winit event loop. They are created
/// when the event loop's `resumed()` handler fires.
pub struct SquiglabApplication {
    builder: PlatformBuilder,
}

impl SquiglabApplication {
    /// Create a new application bridge, initializing the Winit platform.
    pub fn new() -> Self {
        Self {
            builder: PlatformBuilder::new(),
        }
    }

    /// Queue a window for creation when the event loop starts.
    /// Returns the window's identifier.
    pub fn create_window(
        &mut self,
        title: &str,
        width: f32,
        height: f32,
    ) -> squiglab_platform::WindowId {
        let attrs = WindowAttributes {
            title: title.to_string(),
            inner_size: Some((width, height)),
            ..Default::default()
        };
        self.builder.create_window(attrs)
    }

    /// Run the application. This blocks on the Winit event loop.
    /// `on_launch` is called once the event loop has started and windows
    /// have been created.
    pub fn run(self, on_launch: Box<dyn FnOnce() + Send + Sync>) {
        let platform = self.builder.build();
        platform.run(on_launch);
    }
}

impl Default for SquiglabApplication {
    fn default() -> Self {
        Self::new()
    }
}
