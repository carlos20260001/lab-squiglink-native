//! Winit event loop integration for the squiglab platform.
//!
//! Runs Winit's `EventLoop`, creates windows in `resumed()`, translates
//! `WindowEvent`s to GPUI input, and triggers frame rendering.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gpui::{AnyWindowHandle, PlatformInput, WindowParams};
use winit::event::{WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId as WinitWindowId;

use super::application::SquiglabState;
use super::input::translate_event;

thread_local! {
    /// Map from Winit window IDs to GPUI window handles.
    static WINDOW_MAP: RefCell<HashMap<WinitWindowId, AnyWindowHandle>> = RefCell::new(HashMap::new());
}

/// Run the Winit event loop with the squiglab platform.
pub fn run_event_loop(state: Rc<SquiglabState>, on_finish_launching: Box<dyn FnOnce()>) {
    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(e) => {
            log::error!("Failed to create Winit event loop: {e}");
            return;
        }
    };

    let mut app = SquiglabApp {
        state,
        on_finish_launching: Some(on_finish_launching),
        windows: HashMap::new(),
    };

    if let Err(e) = event_loop.run_app(&mut app) {
        log::error!("Winit event loop error: {e}");
    }
}

struct SquiglabApp {
    state: Rc<SquiglabState>,
    on_finish_launching: Option<Box<dyn FnOnce()>>,
    windows: HashMap<AnyWindowHandle, ()>,
}

impl winit::application::ApplicationHandler<()> for SquiglabApp {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // Call the on_finish_launching callback.
        // This creates GPUI windows through platform.open_window().
        if let Some(callback) = self.on_finish_launching.take() {
            callback();
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        window_id: WinitWindowId,
        event: WindowEvent,
    ) {
        // Translate Winit event to GPUI input.
        // For now, we just drain the main thread queue.
        super::application::drain_main_thread_queue();
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Drain the main thread queue and request frames.
        super::application::drain_main_thread_queue();
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {}
}
