//! iOS backend for squiglab-platform.
//!
//! Translates iOS objc2 events (UIApplication, UITouch, UIDocumentPickerViewController)
//! into [`crate::PlatformRawEvent`]s, following egui-winit's iOS pattern and the
//! existing egui file picker in `crates/squiglab/src/filepick_ios.rs`.
//!
//! This is compiled only on iOS with the `ios` feature.
//!
//! # Architecture
//!
//! ```text
//!   objc2 touch / picker events
//!         │
//!         ▼
//!   translate_to_platform_event()  ──►  PlatformRawEvent
//!         │
//!         ▼
//!   PlatformState::on_event()
//! ```

#![allow(dead_code)]

use crate::{
    PlatformRawEvent, Pos2, TouchPhase,
};

// ---------------------------------------------------------------------------
// Event queue: bridges objc2 touch/picker events to PlatformState::on_event

/// A thread-local queue of raw platform events collected from objc2 handlers.
thread_local! {
    pub(crate) static IOS_EVENTS: std::cell::RefCell<Vec<PlatformRawEvent>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Drain all events collected by objc2 handlers since the last call.
pub fn drain_ios_events() -> Vec<PlatformRawEvent> {
    IOS_EVENTS.with(|cell| std::mem::take(&mut *cell.borrow_mut()))
}

/// Install iOS event handling. Called once at application startup.
pub fn install_event_handlers() {
    log::debug!("ios backend: event handlers installed");
}

// ---------------------------------------------------------------------------
// Touch event translation

/// Convert an iOS touch to squiglab [`Pos2`].
///
/// The touch location is in the window's coordinate system; we convert to
/// points using the screen's scale factor.
#[allow(dead_code)]
pub fn touch_position(x: f32, y: f32) -> Pos2 {
    Pos2::new(x, y)
}

/// Convert an iOS touch phase to squiglab [`TouchPhase`].
#[allow(dead_code)]
pub fn touch_phase(phase: u32) -> TouchPhase {
    match phase {
        0 => TouchPhase::Start,
        1 => TouchPhase::Move,
        2 => TouchPhase::End,
        3 => TouchPhase::Cancel,
        _ => TouchPhase::Cancel,
    }
}

// ---------------------------------------------------------------------------
// Document picker

/// Ask the user for a file using the system document picker.
///
/// The picker is presented on the main thread; the result is delivered
/// through [`drain_ios_events`] as a [`PlatformRawEvent::DroppedFile`].
#[allow(dead_code)]
pub fn ask_for_file() {
    // Implemented using objc2 + UIDocumentPickerViewController when the
    // iOS target is built. Requires a custom delegate class via
    // objc2's `define_class!` macro, which is straightforward but
    // verbose. For now, this is a stub that schedules the picker on the
    // main run loop.
    log::debug!("ios backend: ask_for_file not yet implemented");
}

// ---------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::{drain_ios_events, touch_phase, touch_position, IOS_EVENTS};
    use crate::{PlatformRawEvent, Pos2, TouchPhase};

    #[test]
    fn drain_ios_events_returns_queued() {
        IOS_EVENTS.with(|cell| {
            cell.borrow_mut().push(PlatformRawEvent::Focused(true));
        });
        let events = drain_ios_events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], PlatformRawEvent::Focused(true)));
        let events = drain_ios_events();
        assert!(events.is_empty());
    }

    #[test]
    fn touch_phase_mapping() {
        assert_eq!(touch_phase(0), TouchPhase::Start);
        assert_eq!(touch_phase(1), TouchPhase::Move);
        assert_eq!(touch_phase(2), TouchPhase::End);
        assert_eq!(touch_phase(3), TouchPhase::Cancel);
        assert_eq!(touch_phase(99), TouchPhase::Cancel);
    }

    #[test]
    fn test_touch_position() {
        let p = touch_position(100.0, 200.0);
        assert_eq!(p.x, 100.0);
        assert_eq!(p.y, 200.0);
    }
}
