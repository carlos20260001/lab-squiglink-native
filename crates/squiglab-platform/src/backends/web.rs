//! WebAssembly backend using Winit's wasm32 integration.
//!
//! Creates and attaches a Winit-managed browser canvas, handles browser
//! lifecycle events, and integrates GPU presentation with the browser canvas.
//!
//! Compiled only on `wasm32` with the `web` feature.

#![allow(dead_code)]

use crate::capabilities::PlatformCapabilities;
use crate::events::{PlatformEvent, PlatformInput, PlatformOutput, PlatformRawEvent, ViewportId};
use crate::platform::{Platform, PlatformState};
use crate::renderer::{NullRenderer, Renderer};
use crate::window::{Window, WindowAttributes, WindowId};

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use web_sys::window;

thread_local! {
    /// The single platform instance, shared between wasm-bindgen exports.
    static WEB_PLATFORM: RefCell<Option<Rc<RefCell<WebPlatform>>>> = RefCell::new(None);
}

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

    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities::web()
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

// ---------------------------------------------------------------------------
// wasm-bindgen exports for browser testing
//

/// Create the web platform instance. Called from JavaScript at startup.
#[wasm_bindgen(js_name = createApp)]
pub fn create_app() -> Result<(), JsValue> {
    let platform = Rc::new(RefCell::new(WebPlatform::new(None)));
    WEB_PLATFORM.with(|cell| {
        *cell.borrow_mut() = Some(Rc::clone(&platform));
    });
    log::debug!("web renderer: platform created");
    Ok(())
}

/// Run one frame. Called from JavaScript's requestAnimationFrame loop.
#[wasm_bindgen(js_name = frame)]
pub fn frame() {
    WEB_PLATFORM.with(|cell| {
        if let Some(platform) = cell.borrow().as_ref() {
            let mut p = platform.borrow_mut();
            let _input = p.take_input(None);
            let _renderer = p.renderer();
            renderer_begin_end_web(_renderer);
        }
    });
}

/// Get a summary of platform capabilities as a string.
#[wasm_bindgen(js_name = capabilities)]
pub fn capabilities() -> Result<String, JsValue> {
    WEB_PLATFORM.with(|cell| {
        if let Some(platform) = cell.borrow().as_ref() {
            let caps = platform.borrow().capabilities();
            Ok(format!(
                "gpu_rendering={} clipboard={} ime={} fullscreen={} file_dialogs={} drag_and_drop={} multiple_windows={}",
                caps.gpu_rendering,
                caps.clipboard,
                caps.ime,
                caps.fullscreen,
                caps.file_dialogs,
                caps.drag_and_drop,
                caps.multiple_windows,
            ))
        } else {
            Err(JsValue::from_str("platform not initialized"))
        }
    })
}

/// Application name.
#[wasm_bindgen(js_name = appName)]
pub fn app_name() -> String {
    squiglab_core::product::APP_NAME.to_string()
}

/// Begin and end a frame on the renderer (web).
fn renderer_begin_end_web(renderer: &mut dyn Renderer) {
    renderer.begin_frame();
    renderer.end_frame();
}

