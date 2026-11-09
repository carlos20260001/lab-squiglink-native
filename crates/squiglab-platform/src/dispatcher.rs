//! Event dispatcher — bridges platform events (winit WindowEvents, web DOM
//! events) to PlatformState::on_event.

use crate::events::PlatformRawEvent;

/// Dispatches raw platform events to the platform state.
/// Each backend calls this for every platform event it receives.
pub struct EventDispatcher;

impl EventDispatcher {
    /// Dispatch a raw event to all registered platform states.
    /// In the common single-window case there is one state.
    pub fn dispatch(state: &mut crate::platform::PlatformState, event: &PlatformRawEvent) {
        state.on_event(event);
    }
}
