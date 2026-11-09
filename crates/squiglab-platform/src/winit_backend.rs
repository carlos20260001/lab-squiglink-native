//! winit backend for squiglab-platform.
//!
//! Translates winit `WindowEvent`s into [`PlatformRawEvent`]s, following
//! egui-winit's `State::on_window_event` pattern.
//!
//! This backend is active when the `winit` feature is enabled. It requires
//! `winit` 0.30, which is now in the offline lock file.
//!
//! # Architecture
//!
//! ```text
//!   winit WindowEvent
//!         │
//!         ▼
//!   window_event_to_platform_events()  ──►  PlatformRawEvent
//!         │
//!         ▼
//!   PlatformState::on_event()
//! ```

#![allow(dead_code)]

use crate::{
    Key, KeyEvent, MouseWheelUnit, PlatformRawEvent, PointerButton, Pos2, Theme, TouchPhase,
    Vec2,
};

use winit::event::{
    ElementState, MouseButton, MouseScrollDelta, TouchPhase as WinitTouchPhase, WindowEvent,
};
use winit::keyboard::{
    Key as WinitKey, NamedKey,
};

// ---------------------------------------------------------------------------
// Event queue: bridges winit events to PlatformState::on_event

// A thread-local queue of raw platform events collected from winit event
// handlers.
// This is documented as a regular comment because `thread_local!` does not
// support doc comments.
thread_local! {
    pub(crate) static WINDOW_EVENTS: std::cell::RefCell<Vec<PlatformRawEvent>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Drain all events collected by winit handlers since the last call.
pub fn drain_window_events() -> Vec<PlatformRawEvent> {
    WINDOW_EVENTS.with(|cell| std::mem::take(&mut *cell.borrow_mut()))
}

/// Install winit event handling. Called once at application startup.
pub fn install_event_handlers() {
    // winit events are delivered through the event loop, not through
    // global callbacks. This function is a no-op here; the application
    // is expected to call `window_event_to_platform_events` for each
    // `winit::event::WindowEvent` it receives from its event loop.
    log::debug!("winit backend: install_event_handlers called (no-op, events come from event loop)");
}

// ---------------------------------------------------------------------------
// Translate a winit WindowEvent into squiglab platform events.
//
// Call this once per winit WindowEvent received from the event loop, then
// drain the events with [`drain_window_events`] and feed them to
/// `PlatformState::on_event`.
pub fn window_event_to_platform_events(event: &WindowEvent) -> Vec<PlatformRawEvent> {
    let mut events = Vec::new();

    match event {
        WindowEvent::Resized(size) => {
            events.push(PlatformRawEvent::Resized(Vec2::new(
                size.width as f32,
                size.height as f32,
            )));
        }
        WindowEvent::Focused(focused) => {
            events.push(PlatformRawEvent::Focused(*focused));
        }
        WindowEvent::ScaleFactorChanged {
            scale_factor,
            inner_size_writer: _,
        } => {
            events.push(PlatformRawEvent::ScaleFactorChanged(*scale_factor as f32));
        }
        WindowEvent::ThemeChanged(theme) => {
            events.push(PlatformRawEvent::ThemeChanged(match theme {
                winit::window::Theme::Light => Theme::Light,
                winit::window::Theme::Dark => Theme::Dark,
            }));
        }
        WindowEvent::KeyboardInput {
            event: key_event,
            is_synthetic: _,
            ..
        } => {
            if let Some(platform_event) = translate_key_event(key_event) {
                events.push(platform_event);
            }
            // Also emit text input for printable characters on press.
            if key_event.state == ElementState::Pressed {
                if let Some(text) = &key_event.text {
                    if !text.is_empty() {
                        events.push(PlatformRawEvent::Text(text.to_string()));
                    }
                }
            }
        }
        WindowEvent::ModifiersChanged(modifiers) => {
            events.push(PlatformRawEvent::ModifiersChanged(
                modifiers_from_winit_modifiers(modifiers),
            ));
        }
        WindowEvent::CursorMoved { position, .. } => {
            events.push(PlatformRawEvent::PointerMoved(pos2_from_physical(position)));
        }
        WindowEvent::CursorEntered { .. } => {
            events.push(PlatformRawEvent::PointerEntered(pos2_from_physical(
                &winit::dpi::PhysicalPosition::new(0.0, 0.0),
            )));
        }
        WindowEvent::CursorLeft { .. } => {
            events.push(PlatformRawEvent::PointerLeft);
        }
        WindowEvent::MouseInput {
            state,
            button,
            ..
        } => {
            if let Some(winit_button) = pointer_button_from_winit(button) {
                events.push(PlatformRawEvent::PointerButton {
                    pos: Pos2::ZERO,
                    button: winit_button,
                    pressed: *state == ElementState::Pressed,
                    modifiers: crate::Modifiers::default(),
                });
            }
        }
        WindowEvent::MouseWheel {
            delta,
            phase: _,
            ..
        } => {
            let (unit, delta_vec) = match delta {
                MouseScrollDelta::LineDelta(x, y) => {
                    (MouseWheelUnit::Line, Vec2::new(*x, *y))
                }
                MouseScrollDelta::PixelDelta(pos) => {
                    (
                        MouseWheelUnit::Point,
                        Vec2::new(pos.x as f32, pos.y as f32),
                    )
                }
            };
            events.push(PlatformRawEvent::MouseWheel {
                unit,
                delta: delta_vec,
                modifiers: crate::Modifiers::default(),
            });
        }
        WindowEvent::Touch(touch) => {
            events.push(PlatformRawEvent::Touch {
                touch_id: touch.id,
                pos: pos2_from_physical(&touch.location),
                phase: touch_phase_from_winit(touch.phase),
                modifiers: crate::Modifiers::default(),
            });
        }
        WindowEvent::DroppedFile(path) => {
            events.push(PlatformRawEvent::DroppedFile(crate::DroppedFile {
                name: path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_owned(),
                path: Some(path.clone()),
                bytes: None,
            }));
        }
        WindowEvent::HoveredFile(path) => {
            events.push(PlatformRawEvent::HoveredFile(crate::DroppedFile {
                name: path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_owned(),
                path: Some(path.clone()),
                bytes: None,
            }));
        }
        WindowEvent::HoveredFileCancelled => {
            events.push(PlatformRawEvent::HoveredFileCancelled);
        }
        WindowEvent::Ime(_) => {
            // IME handling not yet implemented
        }
        _ => {
            // Other window events (ActivationTokenDone, Moved, Destroyed,
            // PinchGesture, PanGesture, DoubleTapGesture, RotationGesture,
            // TouchpadPressure, AxisMotion, Occluded) are not yet mapped.
        }
    }

    events
}

// ---------------------------------------------------------------------------
// Key translation

/// Translate a winit `KeyEvent` into a squiglab [`PlatformRawEvent::Key`].
fn translate_key_event(key_event: &winit::event::KeyEvent) -> Option<PlatformRawEvent> {
    let key = key_from_winit(&key_event.logical_key);
    let physical_key = key_physical_from_winit(&key_event.physical_key);

    Some(PlatformRawEvent::Key(KeyEvent {
        key,
        physical_key,
        pressed: key_event.state == ElementState::Pressed,
        repeat: key_event.repeat,
        modifiers: crate::Modifiers::default(),
    }))
}

/// Convert a winit logical key to squiglab [`Key`].
fn key_from_winit(key: &WinitKey) -> Key {
    match key {
        WinitKey::Named(named) => match named {
            NamedKey::Enter => Key::Enter,
            NamedKey::Tab => Key::Tab,
            NamedKey::ArrowDown => Key::ArrowDown,
            NamedKey::ArrowLeft => Key::ArrowLeft,
            NamedKey::ArrowRight => Key::ArrowRight,
            NamedKey::ArrowUp => Key::ArrowUp,
            NamedKey::End => Key::End,
            NamedKey::Home => Key::Home,
            NamedKey::PageDown => Key::PageDown,
            NamedKey::PageUp => Key::PageUp,
            NamedKey::Backspace => Key::Backspace,
            NamedKey::Delete => Key::Delete,
            NamedKey::Insert => Key::Insert,
            NamedKey::Escape => Key::Escape,
            NamedKey::Cut => Key::Cut,
            NamedKey::Copy => Key::Copy,
            NamedKey::Paste => Key::Paste,
            NamedKey::Space => Key::Space,
            NamedKey::F1 => Key::F1,
            NamedKey::F2 => Key::F2,
            NamedKey::F3 => Key::F3,
            NamedKey::F4 => Key::F4,
            NamedKey::F5 => Key::F5,
            NamedKey::F6 => Key::F6,
            NamedKey::F7 => Key::F7,
            NamedKey::F8 => Key::F8,
            NamedKey::F9 => Key::F9,
            NamedKey::F10 => Key::F10,
            NamedKey::F11 => Key::F11,
            NamedKey::F12 => Key::F12,
            NamedKey::F13 => Key::F13,
            NamedKey::F14 => Key::F14,
            NamedKey::F15 => Key::F15,
            NamedKey::F16 => Key::F16,
            NamedKey::F17 => Key::F17,
            NamedKey::F18 => Key::F18,
            NamedKey::F19 => Key::F19,
            NamedKey::F20 => Key::F20,
            NamedKey::F21 => Key::F21,
            NamedKey::F22 => Key::F22,
            NamedKey::F23 => Key::F23,
            NamedKey::F24 => Key::F24,
            NamedKey::F25 => Key::F25,
            NamedKey::F26 => Key::F26,
            NamedKey::F27 => Key::F27,
            NamedKey::F28 => Key::F28,
            NamedKey::F29 => Key::F29,
            NamedKey::F30 => Key::F30,
            NamedKey::F31 => Key::F31,
            NamedKey::F32 => Key::F32,
            NamedKey::F33 => Key::F33,
            NamedKey::F34 => Key::F34,
            NamedKey::F35 => Key::F35,
            NamedKey::BrowserBack => Key::BrowserBack,
            _ => Key::Other("unknown"),
        },
        WinitKey::Character(ch) => {
            let ch = ch.as_ref();
            match ch {
                " " => Key::Space,
                "," => Key::Comma,
                "." => Key::Period,
                ";" => Key::Semicolon,
                "/" => Key::Slash,
                "\\" => Key::Backslash,
                "[" => Key::OpenBracket,
                "]" => Key::CloseBracket,
                "`" => Key::Backtick,
                "'" => Key::Quote,
                "-" => Key::Minus,
                "=" => Key::Equals,
                "0" => Key::Num0,
                "1" => Key::Num1,
                "2" => Key::Num2,
                "3" => Key::Num3,
                "4" => Key::Num4,
                "5" => Key::Num5,
                "6" => Key::Num6,
                "7" => Key::Num7,
                "8" => Key::Num8,
                "9" => Key::Num9,
                "a" => Key::KeyA,
                "b" => Key::KeyB,
                "c" => Key::KeyC,
                "d" => Key::KeyD,
                "e" => Key::KeyE,
                "f" => Key::KeyF,
                "g" => Key::KeyG,
                "h" => Key::KeyH,
                "i" => Key::KeyI,
                "j" => Key::KeyJ,
                "k" => Key::KeyK,
                "l" => Key::KeyL,
                "m" => Key::KeyM,
                "n" => Key::KeyN,
                "o" => Key::KeyO,
                "p" => Key::KeyP,
                "q" => Key::KeyQ,
                "r" => Key::KeyR,
                "s" => Key::KeyS,
                "t" => Key::KeyT,
                "u" => Key::KeyU,
                "v" => Key::KeyV,
                "w" => Key::KeyW,
                "x" => Key::KeyX,
                "y" => Key::KeyY,
                "z" => Key::KeyZ,
                "A" => Key::KeyA,
                "B" => Key::KeyB,
                "C" => Key::KeyC,
                "D" => Key::KeyD,
                "E" => Key::KeyE,
                "F" => Key::KeyF,
                "G" => Key::KeyG,
                "H" => Key::KeyH,
                "I" => Key::KeyI,
                "J" => Key::KeyJ,
                "K" => Key::KeyK,
                "L" => Key::KeyL,
                "M" => Key::KeyM,
                "N" => Key::KeyN,
                "O" => Key::KeyO,
                "P" => Key::KeyP,
                "Q" => Key::KeyQ,
                "R" => Key::KeyR,
                "S" => Key::KeyS,
                "T" => Key::KeyT,
                "U" => Key::KeyU,
                "V" => Key::KeyV,
                "W" => Key::KeyW,
                "X" => Key::KeyX,
                "Y" => Key::KeyY,
                "Z" => Key::KeyZ,
                _ => Key::Other("unknown"),
            }
        }
        WinitKey::Dead(_) => Key::Other("dead"),
        WinitKey::Unidentified(_) => Key::Other("unidentified"),
    }
}

/// Convert a winit physical key to squiglab [`Key`].
fn key_physical_from_winit(
    physical: &winit::keyboard::PhysicalKey,
) -> Option<Key> {
    match physical {
        winit::keyboard::PhysicalKey::Code(_code) => {
            // Physical key code mapping not needed for now
            None
        }
        winit::keyboard::PhysicalKey::Unidentified(_) => None,
    }
}

/// Convert winit `Modifiers` to squiglab [`Modifiers`].
fn modifiers_from_winit_modifiers(modifiers: &winit::event::Modifiers) -> crate::Modifiers {
    let pressed = winit::keyboard::ModifiersKeyState::Pressed;
    crate::Modifiers {
        shift: modifiers.lshift_state() == pressed
            || modifiers.rshift_state() == pressed,
        ctrl: modifiers.lcontrol_state() == pressed
            || modifiers.rcontrol_state() == pressed,
        alt: modifiers.lalt_state() == pressed
            || modifiers.ralt_state() == pressed,
        command: modifiers.lsuper_state() == pressed
            || modifiers.rsuper_state() == pressed,
        mac_cmd: modifiers.lsuper_state() == pressed
            || modifiers.rsuper_state() == pressed,
    }
}

// ---------------------------------------------------------------------------
// Pointer / touch translation

/// Convert a winit mouse button to squiglab [`PointerButton`].
fn pointer_button_from_winit(button: &MouseButton) -> Option<PointerButton> {
    match button {
        MouseButton::Left => Some(PointerButton::Primary),
        MouseButton::Right => Some(PointerButton::Secondary),
        MouseButton::Middle => Some(PointerButton::Middle),
        MouseButton::Back => Some(PointerButton::Extra1),
        MouseButton::Forward => Some(PointerButton::Extra2),
        MouseButton::Other(_) => None,
    }
}

/// Convert a winit physical position to squiglab [`Pos2`] (in points).
fn pos2_from_physical(pos: &winit::dpi::PhysicalPosition<f64>) -> Pos2 {
    Pos2::new(pos.x as f32, pos.y as f32)
}

/// Convert a winit touch phase to squiglab [`TouchPhase`].
fn touch_phase_from_winit(phase: WinitTouchPhase) -> TouchPhase {
    match phase {
        WinitTouchPhase::Started => TouchPhase::Start,
        WinitTouchPhase::Moved => TouchPhase::Move,
        WinitTouchPhase::Ended => TouchPhase::End,
        WinitTouchPhase::Cancelled => TouchPhase::Cancel,
    }
}

/// Convert a winit touch position to squiglab [`Pos2`].
pub fn touch_position(x: f32, y: f32) -> Pos2 {
    Pos2::new(x, y)
}

// ---------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::{
        drain_window_events, install_event_handlers, key_from_winit,
        pointer_button_from_winit, touch_phase_from_winit,
        window_event_to_platform_events,
    };
    use super::{Key, PlatformRawEvent, PointerButton, Theme, TouchPhase};

    use winit::event::{MouseButton, TouchPhase as WinitTouchPhase, WindowEvent};
    use winit::keyboard::NamedKey;
    use winit::window::Theme as WinitTheme;

    #[test]
    fn drain_window_events_returns_queued() {
        super::WINDOW_EVENTS.with(|cell| {
            cell.borrow_mut().push(PlatformRawEvent::Focused(true));
        });
        let events = drain_window_events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], PlatformRawEvent::Focused(true)));
        let events = drain_window_events();
        assert!(events.is_empty());
    }

    #[test]
    fn window_event_resized() {
        let event = WindowEvent::Resized(winit::dpi::PhysicalSize::new(800, 600));
        let events = window_event_to_platform_events(&event);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            PlatformRawEvent::Resized(v) if v.x == 800.0 && v.y == 600.0
        ));
    }

    #[test]
    fn window_event_focused() {
        let event = WindowEvent::Focused(true);
        let events = window_event_to_platform_events(&event);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], PlatformRawEvent::Focused(true)));

        let event = WindowEvent::Focused(false);
        let events = window_event_to_platform_events(&event);
        assert!(matches!(events[0], PlatformRawEvent::Focused(false)));
    }

    #[test]
    fn window_event_theme_changed() {
        let event = WindowEvent::ThemeChanged(WinitTheme::Dark);
        let events = window_event_to_platform_events(&event);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            PlatformRawEvent::ThemeChanged(Theme::Dark)
        ));
    }

    #[test]
    fn key_from_winit_enter() {
        let key = key_from_winit(&winit::keyboard::Key::Named(NamedKey::Enter));
        assert_eq!(key, Key::Enter);
    }

    #[test]
    fn key_from_winit_letter() {
        let key = key_from_winit(&winit::keyboard::Key::Character("a".into()));
        assert_eq!(key, Key::KeyA);
    }

    #[test]
    fn pointer_button_from_winit_left() {
        let btn = pointer_button_from_winit(&MouseButton::Left);
        assert_eq!(btn, Some(PointerButton::Primary));
    }

    #[test]
    fn pointer_button_from_winit_right() {
        let btn = pointer_button_from_winit(&MouseButton::Right);
        assert_eq!(btn, Some(PointerButton::Secondary));
    }

    #[test]
    fn touch_phase_from_winit_started() {
        let phase = touch_phase_from_winit(WinitTouchPhase::Started);
        assert_eq!(phase, TouchPhase::Start);
    }

    #[test]
    fn touch_phase_from_winit_ended() {
        let phase = touch_phase_from_winit(WinitTouchPhase::Ended);
        assert_eq!(phase, TouchPhase::End);
    }

    #[test]
    fn touch_position_fn() {
        let p = super::touch_position(100.0, 200.0);
        assert_eq!(p.x, 100.0);
        assert_eq!(p.y, 200.0);
    }

    #[test]
    fn install_event_handlers_is_noop() {
        // Should not panic.
        install_event_handlers();
    }
}
