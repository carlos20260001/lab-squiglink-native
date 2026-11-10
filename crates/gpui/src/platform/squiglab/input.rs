//! GPUI input bridge for squiglab.
//!
//! Adapts squiglab-platform's Winit-translated input events to GPUI's input
//! model: keyboard, pointer, touch, scroll, IME, and focus.

use gpui::{PlatformInput, KeyDownEvent, KeyUpEvent, Keystroke};

use squiglab_platform::{Key, Modifiers, PlatformEvent};

/// Translates a squiglab-platform event into GPUI `PlatformInput`.
pub fn translate_event(event: &PlatformEvent) -> Option<PlatformInput> {
    match event {
        PlatformEvent::Key { key, pressed, modifiers, .. } => {
            let keystroke = translate_keystroke(key, modifiers);
            if *pressed {
                Some(PlatformInput::KeyDown(KeyDownEvent {
                    keystroke,
                    is_held: false,
                }))
            } else {
                Some(PlatformInput::KeyUp(KeyUpEvent { keystroke }))
            }
        }
        PlatformEvent::PointerMoved(pos) => {
            Some(PlatformInput::MouseMove(gpui::MouseMoveEvent {
                position: gpui::Point { x: gpui::px(pos.x), y: gpui::px(pos.y) },
                modifiers: translate_modifiers(&Modifiers::default()),
                ..Default::default()
            }))
        }
        _ => None,
    }
}

fn translate_keystroke(key: &Key, modifiers: &Modifiers) -> Keystroke {
    Keystroke {
        key: match key {
            Key::KeyA => "a".to_string(),
            Key::KeyB => "b".to_string(),
            Key::KeyC => "c".to_string(),
            Key::KeyD => "d".to_string(),
            Key::KeyE => "e".to_string(),
            Key::KeyF => "f".to_string(),
            Key::KeyG => "g".to_string(),
            Key::KeyH => "h".to_string(),
            Key::KeyI => "i".to_string(),
            Key::KeyJ => "j".to_string(),
            Key::KeyK => "k".to_string(),
            Key::KeyL => "l".to_string(),
            Key::KeyM => "m".to_string(),
            Key::KeyN => "n".to_string(),
            Key::KeyO => "o".to_string(),
            Key::KeyP => "p".to_string(),
            Key::KeyQ => "q".to_string(),
            Key::KeyR => "r".to_string(),
            Key::KeyS => "s".to_string(),
            Key::KeyT => "t".to_string(),
            Key::KeyU => "u".to_string(),
            Key::KeyV => "v".to_string(),
            Key::KeyW => "w".to_string(),
            Key::KeyX => "x".to_string(),
            Key::KeyY => "y".to_string(),
            Key::KeyZ => "z".to_string(),
            Key::Enter => "enter".to_string(),
            Key::Space => "space".to_string(),
            Key::Escape => "escape".to_string(),
            Key::Tab => "tab".to_string(),
            Key::Backspace => "backspace".to_string(),
            Key::Delete => "delete".to_string(),
            Key::ArrowUp => "up".to_string(),
            Key::ArrowDown => "down".to_string(),
            Key::ArrowLeft => "left".to_string(),
            Key::ArrowRight => "right".to_string(),
            _ => "unknown".to_string(),
        },
        key_char: None,
        modifiers: translate_modifiers(modifiers),
    }
}

fn translate_modifiers(m: &Modifiers) -> gpui::Modifiers {
    gpui::Modifiers {
        control: m.ctrl,
        alt: m.alt,
        shift: m.shift,
        platform: m.command,
        function: false,
    }
}
