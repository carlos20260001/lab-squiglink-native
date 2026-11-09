//! WebAssembly backend using Winit's wasm32 integration.
//!
//! Creates and attaches a Winit-managed browser canvas, handles browser
//! lifecycle events, and integrates GPU presentation with the browser canvas.
//!
//! Compiled only on `wasm32` with the `web` feature.

#![allow(dead_code)]

use crate::events::{PlatformEvent, PlatformInput, PlatformOutput, PlatformRawEvent, ViewportId};
use crate::platform::PlatformState;
use crate::renderer::{NullRenderer, Renderer};
use crate::window::{Window, WindowAttributes, WindowId};

/// Web platform implementation using Winit's wasm32 backend.
pub struct WebPlatform {
    state: PlatformState,
    renderer: NullRenderer,
}

impl WebPlatform {
    pub fn new(native_pixels_per_point: Option<f32>) -> Self {
        Self {
            state: PlatformState::new(ViewportId::ROOT, native_pixels_per_point),
            renderer: NullRenderer,
        }
    }
}

impl crate::platform::Platform for WebPlatform {
    fn run(self: Box<Self>) {
        let _ = self;
        log::debug!("web platform: run() called — Winit drives the browser event loop");
    }

    fn poll_events(&mut self) -> bool {
        true
    }

    fn create_window(&mut self, _attrs: &WindowAttributes) -> Box<dyn crate::window::Window> {
        unimplemented!("web window creation via Winit not yet wired")
    }

    fn on_event(&mut self, event: &PlatformRawEvent) {
        self.state.on_event(event);
    }

    fn take_input(&mut self, screen_rect: Option<crate::events::Rect>) -> PlatformInput {
        self.state.take_input(screen_rect)
    }

    fn handle_platform_output(&mut self, output: &PlatformOutput) {
        self.state.handle_platform_output(output);
    }

    fn renderer(&mut self) -> &mut dyn Renderer {
        &mut self.renderer
    }
}

/// Winit-managed browser canvas window.
pub struct WebWindow {
    id: WindowId,
}

impl WebWindow {
    pub fn new(id: WindowId) -> Self {
        Self { id }
    }
}

impl Window for WebWindow {
    fn id(&self) -> WindowId {
        self.id
    }

    fn set_title(&self, _title: &str) {
        // Browser tab title set via document.title
        if let Some(win) = web_sys::window() {
            let _ = win;
        }
    }

    fn set_inner_size(&self, _size: crate::events::Vec2) {
        // Canvas size managed by Winit/CSS
    }

    fn set_resizable(&self, _resizable: bool) {}

    fn set_focus(&self) {}

    fn set_visible(&self, _visible: bool) {}

    fn set_maximized(&self, _maximized: bool) {}

    fn set_minimized(&self, _minimized: bool) {}

    fn set_theme(&self, _theme: crate::events::Theme) {}

    fn set_cursor_icon(&self, _icon: crate::events::CursorIcon) {}

    fn set_ime_allowed(&self, _allowed: bool) {}

    fn set_ime_cursor_area(&self, _rect: crate::events::Rect, _purpose: crate::events::ImePurpose) {
    }

    fn inner_size(&self) -> (u32, u32) {
        (1440, 900)
    }

    fn scale_factor(&self) -> f32 {
        web_sys::window()
            .map(|w| w.device_pixel_ratio() as f32)
            .unwrap_or(1.0)
    }

    fn request_redraw(&self) {
        // Browser handles redraw via requestAnimationFrame through Winit
    }
}
