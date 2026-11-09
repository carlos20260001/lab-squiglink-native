//! Android backend for squiglab-platform.
//!
//! Translates Android JNI / NDK input events (AInputQueue key and motion
//! events) into [`crate::PlatformRawEvent`]s, following egui-winit's Android
//! pattern and the existing egui file picker in
//! `crates/squiglab/src/filepick_android.rs`.
//!
//! This is compiled only on Android with the `android` feature.
//!
//! # Architecture
//!
//! ```text
//!   AInputQueue key / motion events
//!         │
//!         ▼
//!   translate_to_platform_event()  ──►  PlatformRawEvent
//!         │
//!         ▼
//!   PlatformState::on_event()
//! ```

#![allow(dead_code)]

use crate::{
    Key, KeyEvent, MouseWheelUnit, PlatformRawEvent, PointerButton, Pos2, TouchPhase, Vec2,
};

// ---------------------------------------------------------------------------
// Event queue: bridges JNI/NDK input events to PlatformState::on_event

/// A thread-local queue of raw platform events collected from Android input
/// handlers.
thread_local! {
    pub(crate) static ANDROID_EVENTS: std::cell::RefCell<Vec<PlatformRawEvent>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Drain all events collected by Android input handlers since the last call.
pub fn drain_android_events() -> Vec<PlatformRawEvent> {
    ANDROID_EVENTS.with(|cell| std::mem::take(&mut *cell.borrow_mut()))
}

/// Install Android event handling. Called once at application startup from
/// the native activity's `onCreate`.
pub fn install_event_handlers() {
    log::debug!("android backend: event handlers installed");
}

// ---------------------------------------------------------------------------
// Key event translation

/// Convert an Android key event to squiglab [`KeyEvent`].
///
/// `key_code` is an `AKeyEvent` key code (e.g. `AKEYCODE_ENTER`).
#[allow(dead_code)]
pub fn key_event(key_code: i32, scan_code: i32, meta_state: i32, down: bool) -> Option<KeyEvent> {
    let key = key_code_to_key(key_code)?;
    let modifiers = modifiers_from_meta_state(meta_state);
    Some(KeyEvent {
        key,
        physical_key: None,
        pressed: down,
        repeat: false,
        modifiers,
    })
}

/// Map Android key codes to squiglab [`Key`].
fn key_code_to_key(key_code: i32) -> Option<Key> {
    match key_code {
        // Android key codes from NDK (AKEYCODE_*)
        0 => None, // AKEYCODE_UNKNOWN
        62 => Some(Key::ArrowDown),
        61 => Some(Key::ArrowLeft),
        60 => Some(Key::ArrowRight),
        63 => Some(Key::ArrowUp),
        111 => Some(Key::Escape),
        61 => Some(Key::Tab),
        67 => Some(Key::Backspace),
        66 => Some(Key::Enter),
        124 => Some(Key::Delete),
        122 => Some(Key::Home),
        123 => Some(Key::End),
        93 => Some(Key::PageUp),
        92 => Some(Key::PageDown),
        68 => Some(Key::Space),
        56 => Some(Key::Period),
        75 => Some(Key::Semicolon),
        76 => Some(Key::Slash),
        73 => Some(Key::Backslash),
        72 => Some(Key::OpenBracket),
        71 => Some(Key::CloseBracket),
        70 => Some(Key::Backtick),
        69 => Some(Key::Quote),
        74 => Some(Key::Minus),
        78 => Some(Key::Equals),
        7 => Some(Key::Num0),
        8 => Some(Key::Num1),
        9 => Some(Key::Num2),
        10 => Some(Key::Num3),
        11 => Some(Key::Num4),
        12 => Some(Key::Num5),
        13 => Some(Key::Num6),
        14 => Some(Key::Num7),
        15 => Some(Key::Num8),
        16 => Some(Key::Num9),
        30 => Some(Key::KeyA),
        31 => Some(Key::KeyB),
        32 => Some(Key::KeyC),
        33 => Some(Key::KeyD),
        34 => Some(Key::KeyE),
        35 => Some(Key::KeyF),
        36 => Some(Key::KeyG),
        37 => Some(Key::KeyH),
        38 => Some(Key::KeyI),
        39 => Some(Key::KeyJ),
        40 => Some(Key::KeyK),
        41 => Some(Key::KeyL),
        42 => Some(Key::KeyM),
        43 => Some(Key::KeyN),
        44 => Some(Key::KeyO),
        45 => Some(Key::KeyP),
        46 => Some(Key::KeyQ),
        47 => Some(Key::KeyR),
        48 => Some(Key::KeyS),
        49 => Some(Key::KeyT),
        50 => Some(Key::KeyU),
        51 => Some(Key::KeyV),
        52 => Some(Key::KeyW),
        53 => Some(Key::KeyX),
        54 => Some(Key::KeyY),
        55 => Some(Key::KeyZ),
        131 => Some(Key::F1),
        132 => Some(Key::F2),
        133 => Some(Key::F3),
        134 => Some(Key::F4),
        135 => Some(Key::F5),
        136 => Some(Key::F6),
        137 => Some(Key::F7),
        138 => Some(Key::F8),
        139 => Some(Key::F9),
        140 => Some(Key::F10),
        141 => Some(Key::F11),
        142 => Some(Key::F12),
        59 => Some(Key::ShiftLeft),
        113 => Some(Key::ControlLeft),
        57 => Some(Key::AltLeft),
        114 => Some(Key::SuperLeft),
        _ => None,
    }
}

/// Convert Android meta state to squiglab [`Modifiers`].
fn modifiers_from_meta_state(meta_state: i32) -> crate::Modifiers {
    crate::Modifiers {
        shift: meta_state & 0x01 != 0, // AMETA_SHIFT_ON
        ctrl: meta_state & 0x1000 != 0, // AMETA_CTRL_ON
        alt: meta_state & 0x02 != 0, // AMETA_ALT_ON
        command: meta_state & 0x2000 != 0, // AMETA_META_ON
        mac_cmd: meta_state & 0x2000 != 0,
    }
}

// ---------------------------------------------------------------------------
// Motion (touch) event translation

/// Convert an Android motion event to squiglab [`PlatformRawEvent`].
///
/// `action` is an `AMotionEvent` action (e.g. `AMOTION_EVENT_ACTION_DOWN`).
/// `pointer_id` is the touch pointer ID. `x` and `y` are in screen pixels.
#[allow(dead_code)]
pub fn motion_event(action: i32, pointer_id: i32, x: f32, y: f32, meta_state: i32) -> Vec<PlatformRawEvent> {
    let phase = match action {
        0 => TouchPhase::Start, // AMOTION_EVENT_ACTION_DOWN
        2 => TouchPhase::Move, // AMOTION_EVENT_ACTION_MOVE
        1 => TouchPhase::End, // AMOTION_EVENT_ACTION_UP
        3 => TouchPhase::Cancel, // AMOTION_EVENT_ACTION_CANCEL
        6 => TouchPhase::End, // AMOTION_EVENT_ACTION_POINTER_UP
        5 => TouchPhase::Start, // AMOTION_EVENT_ACTION_POINTER_DOWN
        _ => return Vec::new(),
    };
    let modifiers = modifiers_from_meta_state(meta_state);
    // Convert from pixels to points using the display scale factor.
    let scale = display_scale();
    let pos = Pos2::new(x / scale, y / scale);
    vec![PlatformRawEvent::Touch {
        touch_id: pointer_id as u64,
        pos,
        phase,
        modifiers,
    }]
}

/// Get the display scale factor (pixels per point).
#[allow(dead_code)]
fn display_scale() -> f32 {
    // On Android, this is obtained from the DisplayMetrics density.
    // For now, return 1.0; the application can override via
    // PlatformInput::native_pixels_per_point.
    1.0
}

// ---------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use crate::{PlatformRawEvent, PointerButton, TouchPhase};

    use super::*;

    #[test]
    fn drain_android_events_returns_queued() {
        ANDROID_EVENTS.with(|cell| {
            cell.borrow_mut().push(PlatformRawEvent::Focused(true));
        });
        let events = drain_android_events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], PlatformRawEvent::Focused(true)));
        let events = drain_android_events();
        assert!(events.is_empty());
    }

    #[test]
    fn key_code_mapping() {
        assert_eq!(key_code_to_key(66), Some(Key::Enter));
        assert_eq!(key_code_to_key(67), Some(Key::Backspace));
        assert_eq!(key_code_to_key(62), Some(Key::ArrowDown));
        assert_eq!(key_code_to_key(30), Some(Key::KeyA));
        assert_eq!(key_code_to_key(999), None);
    }

    #[test]
    fn motion_event_mapping() {
        let events = motion_event(0, 1, 100.0, 200.0, 0);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            PlatformRawEvent::Touch {
                touch_id: 1,
                phase: TouchPhase::Start,
                ..
            }
        ));

        let events = motion_event(2, 1, 150.0, 250.0, 0);
        assert!(matches!(
            events[0],
            PlatformRawEvent::Touch {
                phase: TouchPhase::Move,
                ..
            }
        ));

        let events = motion_event(1, 1, 150.0, 250.0, 0);
        assert!(matches!(
            events[0],
            PlatformRawEvent::Touch {
                phase: TouchPhase::End,
                ..
            }
        ));
    }

    #[test]
    fn modifiers_from_meta_state() {
        let modifiers = super::modifiers_from_meta_state(0x01); // AMETA_SHIFT_ON
        assert!(modifiers.shift);
        assert!(!modifiers.ctrl);
        assert!(!modifiers.alt);
        assert!(!modifiers.command);

        let modifiers = super::modifiers_from_meta_state(0x1000); // AMETA_CTRL_ON
        assert!(!modifiers.shift);
        assert!(modifiers.ctrl);
    }

    #[test]
    fn pointer_button_mapping() {
        assert_eq!(
            pointer_button_from_action(0),
            Some(PointerButton::Primary)
        );
        assert_eq!(
            pointer_button_from_action(1),
            Some(PointerButton::Secondary)
        );
        assert_eq!(pointer_button_from_action(99), None);
    }

    fn pointer_button_from_action(button: i32) -> Option<PointerButton> {
        match button {
            0 => Some(PointerButton::Primary),
            1 => Some(PointerButton::Secondary),
            2 => Some(PointerButton::Middle),
            _ => None,
        }
    }
}
