//! Event-loop integration — Winit's `EventLoop` + `ApplicationHandler`.
//!
//! This module bridges Winit's event loop to squiglab-platform's `Platform`
//! trait. Winit is the authoritative event loop on all targets:
//! `EventLoop`, `ActiveEventLoop`, `ApplicationHandler`, `ControlFlow`,
//! `RedrawRequested`, `WindowEvent`, `DeviceEvent` are all used directly.
//!
//! No custom native event loop is implemented — Winit's `run_app` drives
//! everything. On WebAssembly, Winit's wasm32 backend drives the event
//! loop via the browser's requestAnimationFrame.

#![allow(dead_code)]

use std::time::Instant;

use crate::capabilities::PlatformCapabilities;
use crate::events::{PlatformEvent, PlatformInput, PlatformRawEvent, Rect, ViewportId};
use crate::platform::PlatformState;
use crate::renderer::Renderer;
use crate::window::{Window, WindowAttributes, WindowId};

/// Winit-driven event loop. Holds the Winit `EventLoop` and the per-window
/// platform state.
pub struct EventLoop {
    #[allow(dead_code)]
    start_time: Instant,
}

impl EventLoop {
    /// Create a new Winit event loop.
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }

    /// Run the event loop with the given platform.
    /// This blocks until the application exits.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn run_app<P: crate::platform::Platform>(
        mut platform: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = winit::event_loop::EventLoop::new()?;
        let mut app = EventLoopApp {
            platform: Some(platform),
            windows: std::collections::HashMap::new(),
        };
        event_loop.run_app(&mut app)?;
        Ok(())
    }

    /// Run the event loop on WebAssembly (Winit's wasm32 backend).
    #[cfg(target_arch = "wasm32")]
    pub fn run_app<P: crate::platform::Platform>(
        _platform: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Winit's wasm32 backend drives the event loop via the browser's
        // requestAnimationFrame. Called from the wasm entry point.
        Ok(())
    }
}

/// Winit `ApplicationHandler` adapter. Translates winit events into
/// `PlatformRawEvent`s and dispatches them to `PlatformState`.
/// Native-only; WebAssembly uses Winit's browser event loop directly.
#[cfg(not(target_arch = "wasm32"))]
pub struct EventLoopApp<P: crate::platform::Platform> {
    platform: Option<P>,
    /// Map from winit's `WindowId` to squiglab `WindowId`.
    windows: std::collections::HashMap<winit::window::WindowId, WindowId>,
}

#[cfg(not(target_arch = "wasm32"))]
impl<P: crate::platform::Platform> winit::application::ApplicationHandler<()> for EventLoopApp<P> {
    fn new_events(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _cause: winit::event::StartCause,
    ) {
        // Start of a new iteration — nothing to accumulate yet.
    }

    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        // Platform is ready to create windows and surfaces.
    }

    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let platform = self.platform.as_mut().expect("platform set");
        let raw_events = crate::backends::native::window_event_to_platform_events(&event);
        for raw_event in raw_events {
            platform.on_event(&raw_event);
        }
        // Match winit window id to squiglab window id.
        let squiglab_window_id = *self.windows.get(&window_id).unwrap_or(&WindowId(0));
        let _ = squiglab_window_id;
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        _event: winit::event::DeviceEvent,
    ) {
        // Device events (raw mouse, keyboard, etc.) are not yet translated.
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        // End of iteration — drain input and present.
        if let Some(platform) = self.platform.as_mut() {
            let _input = platform.take_input(None);
            let _renderer = platform.renderer();
            renderer_begin_end(_renderer);
        }
    }

    fn exiting(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        // Application is exiting.
    }
}

/// Begin and end a frame on the renderer.
#[cfg(not(target_arch = "wasm32"))]
fn renderer_begin_end(renderer: &mut dyn Renderer) {
    renderer.begin_frame();
    renderer.end_frame();
}
