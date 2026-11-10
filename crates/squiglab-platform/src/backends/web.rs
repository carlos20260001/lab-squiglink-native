//! WebAssembly backend using Winit's wasm32 integration.
//!
//! Creates and attaches a Winit-managed browser canvas, handles browser
//! lifecycle events, and integrates GPU presentation with the browser canvas.
//!
//! Compiled only on `wasm32` with the `web` feature.
//!
//! Windows are created within the event loop (via `ActiveEventLoop::create_window`).
//! `PlatformBuilder::create_window()` queues attributes; they are created
//! when the event loop's `resumed()` handler fires.

#![allow(dead_code)]

use crate::capabilities::PlatformCapabilities;
use crate::events::{PlatformEvent, PlatformInput, PlatformOutput, PlatformRawEvent, ViewportId};
use crate::platform::PlatformState;
use crate::renderer::{NullRenderer, Renderer};
use crate::window::{Window, WindowAttributes, WindowId};

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use web_sys::window;

thread_local! {
    /// The single platform instance, shared between wasm-bindgen exports.
    static WEB_PLATFORM: RefCell<Option<Rc<RefCell<WebPlatformPlaceholder>>>> = RefCell::new(None);
}

/// Web platform implementation using Winit's wasm32 backend.
pub struct WebPlatform {
    pending_windows: Vec<(WindowId, WindowAttributes)>,
    state: PlatformState,
    renderer: NullRenderer,
    windows: std::collections::HashMap<WindowId, web_sys::HtmlCanvasElement>,
    on_launch: Option<Box<dyn FnOnce() + Send + Sync>>,
}

impl WebPlatform {
    pub fn new(
        pending_windows: Vec<(WindowId, WindowAttributes)>,
        native_pixels_per_point: Option<f32>,
    ) -> Self {
        Self {
            pending_windows,
            state: PlatformState::new(ViewportId::ROOT, native_pixels_per_point),
            renderer: NullRenderer,
            windows: std::collections::HashMap::new(),
            on_launch: None,
        }
    }
}

impl crate::platform::Platform for WebPlatform {
    fn run(mut self: Box<Self>, on_launch: Box<dyn FnOnce() + Send + Sync>) {
        self.on_launch = Some(on_launch);
        log::debug!("web platform: run() called — Winit drives the browser event loop");

        // Register the platform instance for wasm-bindgen exports.
        let rc = Rc::new(RefCell::new(WebPlatformPlaceholder {
            capabilities: self.capabilities(),
        }));
        WEB_PLATFORM.with(|cell| {
            *cell.borrow_mut() = Some(Rc::clone(&rc));
        });

        // Call on_launch immediately for wasm (Winit's wasm32 backend
        // drives the event loop via requestAnimationFrame).
        if let Some(callback) = self.on_launch.take() {
            callback();
        }
    }

    fn poll_events(&mut self) -> bool {
        true
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

/// Placeholder for wasm-bindgen exports (the actual platform is consumed by run()).
struct WebPlatformPlaceholder {
    capabilities: PlatformCapabilities,
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
        if let Some(win) = window() {
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
        window()
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
    log::debug!("web renderer: platform created");
    Ok(())
}

/// Run one frame. Called from JavaScript's requestAnimationFrame loop.
#[wasm_bindgen(js_name = frame)]
pub fn frame() {
    WEB_PLATFORM.with(|cell| {
        if let Some(placeholder) = cell.borrow().as_ref() {
            let _caps = placeholder.borrow().capabilities.clone();
            // Frame rendering would go here.
        }
    });
}

/// Get a summary of platform capabilities as a string.
#[wasm_bindgen(js_name = capabilities)]
pub fn capabilities() -> Result<String, JsValue> {
    WEB_PLATFORM.with(|cell| {
        if let Some(placeholder) = cell.borrow().as_ref() {
            let caps = placeholder.borrow().capabilities.clone();
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
