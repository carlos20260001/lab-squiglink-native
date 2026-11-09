//! GPUI Application bridge.
//!
//! Adapts GPUI's application lifecycle (run, activate, quit, open_window) to
//! squiglab-platform's Winit-based event loop.
//!
//! GPUI's `Platform` trait is `pub(crate)`, so this module defines the bridge
//! interface. When GPUI is vendored, the implementation lives in
//! `gpui/src/platform/squiglab/application.rs`.

#![allow(dead_code)]

use squiglab_platform::{Platform, PlatformBuilder};

/// The GPUI application bridge. Holds the squiglab platform and provides
/// the entry point that GPUI's `Application::run` expects.
pub struct SquiglabApplication {
    platform: Box<dyn Platform>,
}

impl SquiglabApplication {
    /// Create a new application bridge, initializing the Winit platform.
    pub fn new() -> Self {
        let platform = PlatformBuilder::new().build();
        Self { platform }
    }

    /// Run the application. This blocks on the Winit event loop.
    pub fn run(mut self) {
        self.platform.run();
    }

    /// Access the underlying platform for window creation and input.
    pub fn platform_mut(&mut self) -> &mut dyn Platform {
        self.platform.as_mut()
    }
}

impl Default for SquiglabApplication {
    fn default() -> Self {
        Self::new()
    }
}
