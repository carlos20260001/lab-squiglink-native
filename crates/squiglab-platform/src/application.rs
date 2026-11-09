//! Application handler — bridges Winit's ApplicationHandler to squiglab's
//! platform abstraction.
//!
//! Implements Winit's `ApplicationHandler` trait to receive window events
//! and dispatch them to PlatformState.

#![allow(dead_code)]

use crate::events::PlatformRawEvent;
use crate::platform::PlatformState;

/// Handles winit events and dispatches them to the platform state.
pub struct ApplicationHandler {
    platform_state: PlatformState,
}

impl ApplicationHandler {
    pub fn new(viewport_id: crate::events::ViewportId) -> Self {
        Self {
            platform_state: PlatformState::new(viewport_id, None),
        }
    }

    /// Handle a winit WindowEvent, translating it to PlatformRawEvents and
    /// dispatching them to PlatformState.
    pub fn on_window_event(&mut self, event: &winit::event::WindowEvent) {
        let raw_events = crate::backends::native::window_event_to_platform_events(event);
        for raw_event in raw_events {
            self.platform_state.on_event(&raw_event);
        }
    }

    /// Get a mutable reference to the platform state.
    pub fn platform_state_mut(&mut self) -> &mut PlatformState {
        &mut self.platform_state
    }

    /// Get a reference to the platform state.
    pub fn platform_state(&self) -> &PlatformState {
        &self.platform_state
    }
}
