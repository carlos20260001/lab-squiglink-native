//! Native desktop backend using Winit.
//!
//! Translates winit `WindowEvent`s into [`PlatformRawEvent`]s, following
//! egui-winit's `State::on_window_event` pattern.
//!
//! This backend is active on all non-wasm32 targets.

#![allow(dead_code)]

use crate::capabilities::PlatformCapabilities;
use crate::events::PlatformOutput;
use crate::events::{
    Key, KeyEvent, MouseWheelUnit, PlatformInput, PlatformRawEvent, PointerButton, Pos2, Rect,
    Theme, TouchPhase, Vec2,
};
use crate::platform::PlatformState;
use crate::renderer::{NullRenderer, Renderer};
use crate::window::{Window, WindowAttributes, WindowId};

use winit::event::{
    ElementState, MouseButton, MouseScrollDelta, TouchPhase as WinitTouchPhase, WindowEvent,
};
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::{Theme as WinitTheme, WindowId as WinitWindowId};

// ---------------------------------------------------------------------------
// Winit window wrapper
//

/// Winit-backed window implementation.
pub struct WinitWindow {
    inner: winit::window::Window,
    id: WindowId,
}

impl WinitWindow {
    pub fn new(inner: winit::window::Window, id: WindowId) -> Self {
        Self { inner, id }
    }
}

impl Window for WinitWindow {
    fn id(&self) -> WindowId {
        self.id
    }

    fn set_title(&self, title: &str) {
        self.inner.set_title(title);
    }

    fn set_inner_size(&self, _size: crate::events::Vec2) {
        // winit 0.30 does not have set_inner_size; size is set at creation.
    }

    fn set_resizable(&self, resizable: bool) {
        self.inner.set_resizable(resizable);
    }

    fn set_focus(&self) {
        self.inner.focus_window();
    }

    fn set_visible(&self, visible: bool) {
        self.inner.set_visible(visible);
    }

    fn set_maximized(&self, _maximized: bool) {
        // Maximize/restore handled via winit's window.set_maximized
    }

    fn set_minimized(&self, _minimized: bool) {}

    fn set_theme(&self, theme: Theme) {
        self.inner.set_theme(match theme {
            Theme::Light => Some(winit::window::Theme::Light),
            Theme::Dark => Some(winit::window::Theme::Dark),
        });
    }

    fn set_cursor_icon(&self, _icon: crate::events::CursorIcon) {
        // Cursor icon set via winit's window.set_cursor_icon
    }

    fn set_ime_allowed(&self, _allowed: bool) {}

    fn set_ime_cursor_area(&self, _rect: crate::events::Rect, _purpose: crate::events::ImePurpose) {
    }

    fn inner_size(&self) -> (u32, u32) {
        let s = self.inner.inner_size();
        (s.width, s.height)
    }

    fn scale_factor(&self) -> f32 {
        self.inner.scale_factor() as f32
    }

    fn request_redraw(&self) {
        self.inner.request_redraw();
    }
}

// ---------------------------------------------------------------------------
// Event translation (from existing winit_backend.rs)
//

/// Translate a winit WindowEvent into squiglab platform events.
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
                WinitTheme::Light => Theme::Light,
                WinitTheme::Dark => Theme::Dark,
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
        WindowEvent::MouseInput { state, button, .. } => {
            if let Some(winit_button) = pointer_button_from_winit(button) {
                events.push(PlatformRawEvent::PointerButton {
                    pos: Pos2::ZERO,
                    button: winit_button,
                    pressed: *state == ElementState::Pressed,
                    modifiers: crate::events::Modifiers::default(),
                });
            }
        }
        WindowEvent::MouseWheel {
            delta, phase: _, ..
        } => {
            let (unit, delta_vec) = match delta {
                MouseScrollDelta::LineDelta(x, y) => (MouseWheelUnit::Line, Vec2::new(*x, *y)),
                MouseScrollDelta::PixelDelta(pos) => {
                    (MouseWheelUnit::Point, Vec2::new(pos.x as f32, pos.y as f32))
                }
            };
            events.push(PlatformRawEvent::MouseWheel {
                unit,
                delta: delta_vec,
                modifiers: crate::events::Modifiers::default(),
            });
        }
        WindowEvent::Touch(touch) => {
            events.push(PlatformRawEvent::Touch {
                touch_id: touch.id,
                pos: pos2_from_physical(&touch.location),
                phase: touch_phase_from_winit(touch.phase),
                modifiers: crate::events::Modifiers::default(),
            });
        }
        WindowEvent::DroppedFile(path) => {
            events.push(PlatformRawEvent::DroppedFile(crate::events::DroppedFile {
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
            events.push(PlatformRawEvent::HoveredFile(crate::events::DroppedFile {
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
        _ => {}
    }

    events
}

/// Translate a winit `KeyEvent` into a squiglab [`PlatformRawEvent::Key`].
fn translate_key_event(key_event: &winit::event::KeyEvent) -> Option<PlatformRawEvent> {
    let key = key_from_winit(&key_event.logical_key);
    let physical_key = key_physical_from_winit(&key_event.physical_key);

    Some(PlatformRawEvent::Key(KeyEvent {
        key,
        physical_key,
        pressed: key_event.state == ElementState::Pressed,
        repeat: key_event.repeat,
        modifiers: crate::events::Modifiers::default(),
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
fn key_physical_from_winit(physical: &winit::keyboard::PhysicalKey) -> Option<Key> {
    match physical {
        winit::keyboard::PhysicalKey::Code(_code) => None,
        winit::keyboard::PhysicalKey::Unidentified(_) => None,
    }
}

/// Convert winit `Modifiers` to squiglab [`Modifiers`].
fn modifiers_from_winit_modifiers(modifiers: &winit::event::Modifiers) -> crate::events::Modifiers {
    let pressed = winit::keyboard::ModifiersKeyState::Pressed;
    crate::events::Modifiers {
        shift: modifiers.lshift_state() == pressed || modifiers.rshift_state() == pressed,
        ctrl: modifiers.lcontrol_state() == pressed || modifiers.rcontrol_state() == pressed,
        alt: modifiers.lalt_state() == pressed || modifiers.ralt_state() == pressed,
        command: modifiers.lsuper_state() == pressed || modifiers.rsuper_state() == pressed,
        mac_cmd: modifiers.lsuper_state() == pressed || modifiers.rsuper_state() == pressed,
    }
}

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

// ---------------------------------------------------------------------------
// Native platform implementation
//

/// Native platform implementation using Winit's EventLoop.
pub struct NativePlatform {
    state: PlatformState,
    renderer: NullRenderer,
}

impl NativePlatform {
    pub fn new(native_pixels_per_point: Option<f32>) -> Self {
        Self {
            state: PlatformState::new(crate::events::ViewportId::ROOT, native_pixels_per_point),
            renderer: NullRenderer,
        }
    }
}

impl crate::platform::Platform for NativePlatform {
    fn run(self: Box<Self>) {
        let _ = self;
        log::debug!("native platform: run() called — event loop integration via Winit");
    }

    fn poll_events(&mut self) -> bool {
        true
    }

    fn create_window(&mut self, _attrs: &WindowAttributes) -> Box<dyn crate::window::Window> {
        let _ = &self.state;
        // Winit window creation is handled by the application via EventLoop.
        // This is a placeholder — the application crate creates the window.
        unimplemented!("window creation via EventLoop not yet wired")
    }

    fn on_event(&mut self, event: &PlatformRawEvent) {
        self.state.on_event(event);
    }

    fn take_input(&mut self, screen_rect: Option<Rect>) -> PlatformInput {
        self.state.take_input(screen_rect)
    }

    fn handle_platform_output(&mut self, output: &PlatformOutput) {
        self.state.handle_platform_output(output);
    }

    fn renderer(&mut self) -> &mut dyn Renderer {
        &mut self.renderer
    }

    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities::native()
    }
}
