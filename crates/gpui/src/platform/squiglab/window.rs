//! GPUI Window implementation for squiglab.
//!
//! Implements GPUI's internal `PlatformWindow` trait, rendering GPUI scenes
//! to a wgpu surface.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    AnyWindowHandle, Bounds, CursorStyle, DispatchEventResult, PlatformDisplay, PlatformInput,
    PlatformInputHandler, PlatformWindow, Point, Pixels, PromptButton, PromptLevel,
    RequestFrameOptions, Scene, Size, WindowAppearance, WindowBounds, WindowParams, px, size,
};

use super::application::SquiglabState;

/// GPUI Window backed by a wgpu surface.
pub struct SquiglabWindow {
    handle: AnyWindowHandle,
    state: Rc<SquiglabState>,
    bounds: Bounds<Pixels>,
    content_size: Size<Pixels>,
    scale_factor: f32,
    appearance: WindowAppearance,
    input_handler: Option<PlatformInputHandler>,
    request_frame_callbacks: Vec<Box<dyn FnMut(RequestFrameOptions)>>,
    input_callbacks: Vec<Box<dyn FnMut(PlatformInput) -> DispatchEventResult>>,
    active_status_callbacks: Vec<Box<dyn FnMut(bool)>>,
    resize_callbacks: Vec<Box<dyn FnMut(Size<Pixels>, f32)>>,
    close_callbacks: Vec<Box<dyn FnOnce()>>,
    should_close_callbacks: Vec<Box<dyn FnMut() -> bool>>,
    on_close: Option<Box<dyn FnOnce()>>,
}

impl SquiglabWindow {
    pub fn new(
        handle: AnyWindowHandle,
        params: WindowParams,
        state: Rc<SquiglabState>,
    ) -> anyhow::Result<Self> {
        let bounds = params.bounds;

        Ok(Self {
            handle,
            state,
            bounds,
            content_size: bounds.size,
            scale_factor: 1.0,
            appearance: WindowAppearance::Light,
            input_handler: None,
            request_frame_callbacks: Vec::new(),
            input_callbacks: Vec::new(),
            active_status_callbacks: Vec::new(),
            resize_callbacks: Vec::new(),
            close_callbacks: Vec::new(),
            should_close_callbacks: Vec::new(),
            on_close: None,
        })
    }

    pub fn handle(&self) -> AnyWindowHandle {
        self.handle
    }
}

impl raw_window_handle::HasWindowHandle for SquiglabWindow {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        Err(raw_window_handle::HandleError::Unavailable)
    }
}

impl raw_window_handle::HasDisplayHandle for SquiglabWindow {
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        Err(raw_window_handle::HandleError::Unavailable)
    }
}

impl PlatformWindow for SquiglabWindow {
    fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }

    fn is_maximized(&self) -> bool {
        false
    }

    fn window_bounds(&self) -> WindowBounds {
        WindowBounds::Windowed(self.bounds)
    }

    fn content_size(&self) -> Size<Pixels> {
        self.content_size
    }

    fn resize(&mut self, size: Size<Pixels>) {
        self.content_size = size;
        self.bounds.size = size;
        for callback in &mut self.resize_callbacks {
            callback(size, self.scale_factor);
        }
    }

    fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    fn appearance(&self) -> WindowAppearance {
        self.appearance
    }

    fn display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        Some(self.state.active_display.clone())
    }

    fn mouse_position(&self) -> Point<Pixels> {
        Point { x: px(0.0), y: px(0.0) }
    }

    fn modifiers(&self) -> gpui::Modifiers {
        gpui::Modifiers::default()
    }

    fn capslock(&self) -> gpui::Capslock {
        gpui::Capslock { on: false }
    }

    fn set_input_handler(&mut self, input_handler: PlatformInputHandler) {
        self.input_handler = Some(input_handler);
    }

    fn take_input_handler(&mut self) -> Option<PlatformInputHandler> {
        self.input_handler.take()
    }

    fn prompt(
        &self,
        _level: PromptLevel,
        _msg: &str,
        _detail: Option<&str>,
        _answers: &[PromptButton],
    ) -> Option<futures::channel::oneshot::Receiver<usize>> {
        None
    }

    fn activate(&self) {
        for callback in &self.active_status_callbacks {
            // Can't call FnMut from &self
        }
    }

    fn is_active(&self) -> bool {
        true
    }

    fn is_hovered(&self) -> bool {
        false
    }

    fn set_title(&mut self, _title: &str) {}

    fn set_background_appearance(&self, _background_appearance: gpui::WindowBackgroundAppearance) {}

    fn minimize(&self) {}

    fn zoom(&self) {}

    fn toggle_fullscreen(&self) {}

    fn is_fullscreen(&self) -> bool {
        false
    }

    fn on_request_frame(&self, callback: Box<dyn FnMut(RequestFrameOptions)>) {
        // Can't store FnMut in &self
        let _ = callback;
    }

    fn on_input(&self, callback: Box<dyn FnMut(PlatformInput) -> DispatchEventResult>) {
        let _ = callback;
    }

    fn on_active_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        let _ = callback;
    }

    fn on_hover_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        let _ = callback;
    }

    fn on_resize(&self, callback: Box<dyn FnMut(Size<Pixels>, f32)>) {
        let _ = callback;
    }

    fn on_moved(&self, callback: Box<dyn FnMut()>) {
        let _ = callback;
    }

    fn on_should_close(&self, callback: Box<dyn FnMut() -> bool>) {
        let _ = callback;
    }

    fn on_hit_test_window_control(&self, callback: Box<dyn FnMut() -> Option<gpui::WindowControlArea>>) {
        let _ = callback;
    }

    fn on_close(&self, callback: Box<dyn FnOnce()>) {
        let _ = callback;
    }

    fn on_appearance_changed(&self, callback: Box<dyn FnMut()>) {
        let _ = callback;
    }

    fn draw(&self, scene: &Scene) {
        // GPUI scene rendering would go here.
        // For now, this is a no-op.
        let _ = scene;
    }

    fn completed_frame(&self) {}

    fn sprite_atlas(&self) -> Arc<dyn gpui::PlatformAtlas> {
        unimplemented!()
    }

    fn update_ime_position(&self, _bounds: Bounds<Pixels>) {}

    fn gpu_specs(&self) -> Option<gpui::GpuSpecs> {
        None
    }
}
