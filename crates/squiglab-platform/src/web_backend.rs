//! Web backend for squiglab-platform.
//!
//! Translates web DOM events (via `web_sys`) into [`crate::PlatformRawEvent`]s,
//! following eframe's web backend pattern (`eframe/src/web/events.rs`).
//!
//! This is compiled only on `wasm32` with the `web` feature.
//!
//! # Architecture
//!
//! ```text
//!   DOM event (web_sys)
//!         │
//!         ▼
//!   translate_to_platform_event()  ──►  PlatformRawEvent
//!         │
//!         ▼
//!   PlatformState::on_event()
//! ```

#![allow(dead_code)]

use crate::{
    Key, MouseWheelUnit, PlatformRawEvent, PointerButton, Pos2, TouchPhase, Vec2,
};

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{window, Document, EventTarget, HtmlElement, Window};

// ---------------------------------------------------------------------------
// Event queue: bridges DOM listeners to PlatformState::on_event

// A thread-local queue of raw platform events collected from DOM listeners.
// DOM listeners run on the browser's main thread (the same thread Rust runs
// on in wasm), so a `thread_local!` `RefCell` is sufficient.
thread_local! {
    pub(crate) static WEB_EVENTS: std::cell::RefCell<Vec<PlatformRawEvent>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Drain all events collected by DOM listeners since the last call.
///
/// The application should call this once per frame, then feed each event to
/// [`PlatformState::on_event`](crate::PlatformState::on_event).
pub fn drain_web_events() -> Vec<PlatformRawEvent> {
    WEB_EVENTS.with(|cell| std::mem::take(&mut *cell.borrow_mut()))
}

/// Register a DOM event listener on `target` for `event_name`.
///
/// The closure receives the raw `web_sys::Event` and returns an optional
/// [`PlatformRawEvent`] to push into the platform state.
fn listen<F>(target: &EventTarget, event_name: &str, mut on_event: F)
where
    F: FnMut(web_sys::Event) -> Option<PlatformRawEvent> + 'static,
{
    let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        if let Some(platform_event) = on_event(event) {
            WEB_EVENTS.with(|cell| {
                cell.borrow_mut().push(platform_event);
            });
        }
    }) as Box<dyn FnMut(web_sys::Event)>);

    let _ = target.add_event_listener_with_callback(
        event_name,
        closure.as_ref().unchecked_ref(),
    );
    // Leak the closure so it lives for the page's lifetime.
    ::std::mem::forget(closure);
}

/// Install all DOM event listeners. Called once at application startup.
pub fn install_event_handlers() -> Result<(), JsValue> {
    let win = window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = win.document().ok_or_else(|| JsValue::from_str("no document"))?;
    let canvas = get_canvas(&document)?;

    // ---- Focus / blur ----------------------------------------------------
    install_blur_focus(&win, &document);
    install_blur_focus(&win, &canvas);

    // ---- Keyboard --------------------------------------------------------
    install_keydown(&canvas);
    install_keyup(&canvas);

    // ---- Clipboard (paste / cut / copy on the document) ------------------
    install_copy_cut_paste(&document);

    // ---- Pointer ---------------------------------------------------------
    // Use the document for move/up so we don't lose a drag that leaves the canvas.
    install_pointerdown(&canvas, &canvas);
    install_pointerup(&document, &canvas);
    install_pointermove(&document, &canvas);
    install_pointerleave(&canvas);

    // ---- Touch -----------------------------------------------------------
    install_touchstart(&canvas);
    install_touchmove(&document);
    install_touchend(&document);
    install_touchcancel(&canvas);

    // ---- Wheel -----------------------------------------------------------
    install_wheel(&canvas);

    // ---- Resize ----------------------------------------------------------
    install_resize(&win);

    // ---- Drag and drop ---------------------------------------------------
    install_drag_and_drop(&canvas);

    log::debug!("web backend: event handlers installed");
    Ok(())
}

/// Return the page's canvas element, or fall back to the document body.
fn get_canvas(document: &Document) -> Result<HtmlElement, JsValue> {
    // Prefer the first canvas if one exists.
    let canvas_option = document.query_selector("canvas");
    match canvas_option {
        Ok(Some(canvas)) => canvas
            .dyn_into::<HtmlElement>()
            .map_err(|_| JsValue::from_str("canvas is not an HtmlElement")),
        Ok(None) => document
            .body()
            .ok_or_else(|| JsValue::from_str("no body"))
            .map(|body| body.dyn_into::<HtmlElement>().unwrap()),
        Err(err) => Err(err),
    }
}

// ---------------------------------------------------------------------------
// Focus / blur

fn install_blur_focus(_window: &Window, target: &EventTarget) {
    let target = target.clone();
    let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        let focused = match event.type_().as_str() {
            "blur" | "visibilitychange" => false,
            "focus" => true,
            _ => false,
        };
        WEB_EVENTS.with(|cell| {
            cell.borrow_mut().push(PlatformRawEvent::Focused(focused));
        });
    }) as Box<dyn FnMut(web_sys::Event)>);

    for event_name in ["blur", "focus", "visibilitychange"] {
        let _ = target.add_event_listener_with_callback(
            event_name,
            closure.as_ref().unchecked_ref(),
        );
    }
    ::std::mem::forget(closure);
}

// ---------------------------------------------------------------------------
// Keyboard

fn install_keydown(target: &EventTarget) {
    listen(
        target,
        "keydown",
        |event: web_sys::Event| {
            let ke: web_sys::KeyboardEvent = match event.dyn_into() {
                Ok(k) => k,
                Err(_) => return None,
            };
            let modifiers = modifiers_from_kb_event(&ke);
            if let Some(text) = text_from_keyboard_event(&ke) {
                if !modifiers.ctrl && !modifiers.command && !modifiers.alt {
                    WEB_EVENTS.with(|cell| {
                        cell.borrow_mut().push(PlatformRawEvent::Text(text));
                    });
                }
            }
            let key = translate_key(&ke.key());
            let physical_key = translate_key(&ke.code());
            let active_key = match key.or(physical_key) {
                Some(k) => k,
                None => return None,
            };
            Some(crate::PlatformRawEvent::Key(crate::KeyEvent {
                key: active_key,
                physical_key,
                pressed: true,
                repeat: false,
                modifiers,
            }))
        },
    );
}

fn install_keyup(target: &EventTarget) {
    listen(
        target,
        "keyup",
        |event: web_sys::Event| {
            let ke: web_sys::KeyboardEvent = match event.dyn_into() {
                Ok(k) => k,
                Err(_) => return None,
            };
            let modifiers = modifiers_from_kb_event(&ke);
            let key = translate_key(&ke.key());
            let physical_key = translate_key(&ke.code());
            let active_key = match key.or(physical_key) {
                Some(k) => k,
                None => return None,
            };
            Some(crate::PlatformRawEvent::Key(crate::KeyEvent {
                key: active_key,
                physical_key,
                pressed: false,
                repeat: false,
                modifiers,
            }))
        },
    );
}

fn text_from_keyboard_event(event: &web_sys::KeyboardEvent) -> Option<String> {
    // Only produce text for printable characters when no modifier is held.
    if event.ctrl_key() || event.meta_key() || event.alt_key() {
        return None;
    }
    let key = event.key();
    if key.len() == 1 {
        let ch = key.chars().next()?;
        if ch.is_ascii_control() {
            return None;
        }
        return Some(key);
    }
    None
}

fn modifiers_from_kb_event(event: &web_sys::KeyboardEvent) -> crate::Modifiers {
    crate::Modifiers {
        shift: event.shift_key(),
        ctrl: event.ctrl_key(),
        alt: event.alt_key(),
        command: event.meta_key(),
        mac_cmd: event.meta_key(),
    }
}

fn translate_key(key: &str) -> Option<Key> {
    match key {
        "Enter" => Some(Key::Enter),
        "Tab" => Some(Key::Tab),
        "ArrowDown" => Some(Key::ArrowDown),
        "ArrowLeft" => Some(Key::ArrowLeft),
        "ArrowRight" => Some(Key::ArrowRight),
        "ArrowUp" => Some(Key::ArrowUp),
        "End" => Some(Key::End),
        "Home" => Some(Key::Home),
        "PageDown" => Some(Key::PageDown),
        "PageUp" => Some(Key::PageUp),
        "Backspace" => Some(Key::Backspace),
        "Delete" => Some(Key::Delete),
        "Insert" => Some(Key::Insert),
        "Escape" => Some(Key::Escape),
        " " => Some(Key::Space),
        "," => Some(Key::Comma),
        "." => Some(Key::Period),
        ";" => Some(Key::Semicolon),
        "/" => Some(Key::Slash),
        "\\" => Some(Key::Backslash),
        "[" => Some(Key::OpenBracket),
        "]" => Some(Key::CloseBracket),
        "`" => Some(Key::Backtick),
        "'" => Some(Key::Quote),
        "-" => Some(Key::Minus),
        "=" => Some(Key::Equals),
        "0" => Some(Key::Num0),
        "1" => Some(Key::Num1),
        "2" => Some(Key::Num2),
        "3" => Some(Key::Num3),
        "4" => Some(Key::Num4),
        "5" => Some(Key::Num5),
        "6" => Some(Key::Num6),
        "7" => Some(Key::Num7),
        "8" => Some(Key::Num8),
        "9" => Some(Key::Num9),
        "a" => Some(Key::KeyA),
        "b" => Some(Key::KeyB),
        "c" => Some(Key::KeyC),
        "d" => Some(Key::KeyD),
        "e" => Some(Key::KeyE),
        "f" => Some(Key::KeyF),
        "g" => Some(Key::KeyG),
        "h" => Some(Key::KeyH),
        "i" => Some(Key::KeyI),
        "j" => Some(Key::KeyJ),
        "k" => Some(Key::KeyK),
        "l" => Some(Key::KeyL),
        "m" => Some(Key::KeyM),
        "n" => Some(Key::KeyN),
        "o" => Some(Key::KeyO),
        "p" => Some(Key::KeyP),
        "q" => Some(Key::KeyQ),
        "r" => Some(Key::KeyR),
        "s" => Some(Key::KeyS),
        "t" => Some(Key::KeyT),
        "u" => Some(Key::KeyU),
        "v" => Some(Key::KeyV),
        "w" => Some(Key::KeyW),
        "x" => Some(Key::KeyX),
        "y" => Some(Key::KeyY),
        "z" => Some(Key::KeyZ),
        "F1" => Some(Key::F1),
        "F2" => Some(Key::F2),
        "F3" => Some(Key::F3),
        "F4" => Some(Key::F4),
        "F5" => Some(Key::F5),
        "F6" => Some(Key::F6),
        "F7" => Some(Key::F7),
        "F8" => Some(Key::F8),
        "F9" => Some(Key::F9),
        "F10" => Some(Key::F10),
        "F11" => Some(Key::F11),
        "F12" => Some(Key::F12),
        "Shift" => Some(Key::ShiftLeft),
        "Control" => Some(Key::ControlLeft),
        "Alt" => Some(Key::AltLeft),
        "Meta" => Some(Key::SuperLeft),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Clipboard

fn install_copy_cut_paste(document: &Document) {
    for event_name in ["paste", "cut", "copy"] {
        let document = document.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let _ = event.prevent_default();
            let clipboard_event: web_sys::ClipboardEvent = match event.dyn_into() {
                Ok(k) => k,
                Err(_) => return,
            };
            match event_name {
                "paste" => {
                    if let Some(data) = clipboard_event.clipboard_data() {
                        if let Ok(text) = data.get_data("text") {
                            if !text.is_empty() {
                                WEB_EVENTS.with(|cell| {
                                    cell.borrow_mut().push(PlatformRawEvent::Paste(text));
                                });
                            }
                        }
                    }
                }
                "cut" => {
                    WEB_EVENTS.with(|cell| {
                        cell.borrow_mut().push(PlatformRawEvent::Cut);
                    });
                }
                "copy" => {
                    WEB_EVENTS.with(|cell| {
                        cell.borrow_mut().push(PlatformRawEvent::Copy);
                    });
                }
                _ => {}
            }
        }) as Box<dyn FnMut(web_sys::Event)>);

        let _ = document
            .add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref());
        ::std::mem::forget(closure);
    }
}

// ---------------------------------------------------------------------------
// Pointer (mouse)

fn pointer_button_from_button(button: i16) -> Option<PointerButton> {
    match button {
        0 => Some(PointerButton::Primary),
        2 => Some(PointerButton::Secondary),
        1 => Some(PointerButton::Middle),
        3 => Some(PointerButton::Extra1),
        4 => Some(PointerButton::Extra2),
        _ => None,
    }
}

fn pos_from_mouse_event(
    canvas: &HtmlElement,
    event: &web_sys::MouseEvent,
    scale_factor: f32,
) -> Pos2 {
    let rect = canvas.get_bounding_client_rect();
    let x = (event.client_x() as f32 - rect.left() as f32) / scale_factor;
    let y = (event.client_y() as f32 - rect.top() as f32) / scale_factor;
    Pos2::new(x, y)
}

fn install_pointer(target: &EventTarget, canvas: &HtmlElement, event_name: &str, pressed: bool) {
    let canvas = canvas.clone();
    let scale_factor = window().map(|w| w.device_pixel_ratio() as f32).unwrap_or(1.0);
    listen(
        target,
        event_name,
        move |event: web_sys::Event| {
            let me: web_sys::PointerEvent = match event.dyn_into() {
                Ok(k) => k,
                Err(_) => return None,
            };
            let modifiers = modifiers_from_mouse_event(&me);
            let button = match pointer_button_from_button(me.button()) {
                Some(b) => b,
                None => return None,
            };
            let pos = pos_from_mouse_event(&canvas, &me, scale_factor);
            Some(crate::PlatformRawEvent::PointerButton {
                pos,
                button,
                pressed,
                modifiers,
            })
        },
    );
}

fn install_pointerdown(target: &EventTarget, canvas: &HtmlElement) {
    install_pointer(target, canvas, "pointerdown", true);
}

fn install_pointerup(target: &EventTarget, canvas: &HtmlElement) {
    install_pointer(target, canvas, "pointerup", false);
}

fn install_pointermove(target: &EventTarget, canvas: &HtmlElement) {
    let canvas = canvas.clone();
    let scale_factor = window().map(|w| w.device_pixel_ratio() as f32).unwrap_or(1.0);
    listen(
        target,
        "pointermove",
        move |event: web_sys::Event| {
            let me: web_sys::PointerEvent = match event.dyn_into() {
                Ok(k) => k,
                Err(_) => return None,
            };
            let _modifiers = modifiers_from_mouse_event(&me);
            let pos = pos_from_mouse_event(&canvas, &me, scale_factor);
            Some(crate::PlatformRawEvent::PointerMoved(pos))
        },
    );
}

fn install_pointerleave(target: &EventTarget) {
    listen(
        target,
        "pointerleave",
        |_event: web_sys::Event| {
            Some(crate::PlatformRawEvent::PointerLeft)
        },
    );
}

fn modifiers_from_mouse_event(event: &web_sys::MouseEvent) -> crate::Modifiers {
    crate::Modifiers {
        shift: event.shift_key(),
        ctrl: event.ctrl_key(),
        alt: event.alt_key(),
        command: event.meta_key(),
        mac_cmd: event.meta_key(),
    }
}

// ---------------------------------------------------------------------------
// Touch

fn install_touch(target: &EventTarget, event_name: &str, phase: TouchPhase) {
    listen(
        target,
        event_name,
        move |event: web_sys::Event| {
            let te: web_sys::TouchEvent = match event.dyn_into() {
                Ok(k) => k,
                Err(_) => return None,
            };
            let modifiers = modifiers_from_touch_event(&te);
            let mut first_event = None;
            for i in 0..te.touches().length() {
                if let Some(touch) = te.touches().item(i) {
                    let scale = window().map(|w| w.device_pixel_ratio() as f32).unwrap_or(1.0);
                    let rect_pos = if let Some(target) = touch.target() {
                        if let Some(el) = target.dyn_ref::<web_sys::Element>() {
                            let r = el.get_bounding_client_rect();
                            Pos2::new(r.left() as f32, r.top() as f32)
                        } else {
                            Pos2::ZERO
                        }
                    } else {
                        Pos2::ZERO
                    };
                    let x = (touch.client_x() as f32 - rect_pos.x) / scale;
                    let y = (touch.client_y() as f32 - rect_pos.y) / scale;
                    let ev = crate::PlatformRawEvent::Touch {
                        touch_id: touch.identifier() as u64,
                        pos: Pos2::new(x, y),
                        phase,
                        modifiers,
                    };
                    if i == 0 {
                        first_event = Some(ev);
                    } else {
                        WEB_EVENTS.with(|cell| {
                            cell.borrow_mut().push(ev);
                        });
                    }
                }
            }
            first_event
        },
    );
}

fn install_touchstart(target: &EventTarget) {
    install_touch(target, "touchstart", TouchPhase::Start);
}

fn install_touchmove(target: &EventTarget) {
    install_touch(target, "touchmove", TouchPhase::Move);
}

fn install_touchend(target: &EventTarget) {
    install_touch(target, "touchend", TouchPhase::End);
}

fn install_touchcancel(target: &EventTarget) {
    install_touch(target, "touchcancel", TouchPhase::Cancel);
}

fn modifiers_from_touch_event(event: &web_sys::TouchEvent) -> crate::Modifiers {
    crate::Modifiers {
        shift: event.shift_key(),
        ctrl: event.ctrl_key(),
        alt: event.alt_key(),
        command: event.meta_key(),
        mac_cmd: event.meta_key(),
    }
}

// ---------------------------------------------------------------------------
// Wheel

fn install_wheel(target: &EventTarget) {
    listen(
        target,
        "wheel",
        move |event: web_sys::Event| {
            let we: web_sys::WheelEvent = match event.dyn_into() {
                Ok(k) => k,
                Err(_) => return None,
            };
            let unit = match we.delta_mode() {
                web_sys::WheelEvent::DOM_DELTA_PIXEL => MouseWheelUnit::Point,
                web_sys::WheelEvent::DOM_DELTA_LINE => MouseWheelUnit::Line,
                web_sys::WheelEvent::DOM_DELTA_PAGE => MouseWheelUnit::Page,
                _ => return None,
            };
            let delta = Vec2::new(-we.delta_x() as f32, -we.delta_y() as f32);
            let modifiers = modifiers_from_wheel_event(&we);
            Some(crate::PlatformRawEvent::MouseWheel {
                unit,
                delta,
                modifiers,
            })
        },
    );
}

fn modifiers_from_wheel_event(event: &web_sys::WheelEvent) -> crate::Modifiers {
    crate::Modifiers {
        shift: event.shift_key(),
        ctrl: event.ctrl_key(),
        alt: event.alt_key(),
        command: event.meta_key(),
        mac_cmd: event.meta_key(),
    }
}

// ---------------------------------------------------------------------------
// Resize

fn install_resize(window: &Window) {
    let win = window.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        let width = win
            .inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .map(|v| v as f32);
        let height = win
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .map(|v| v as f32);
        if let Some(w) = width {
            if let Some(h) = height {
                WEB_EVENTS.with(|cell| {
                    cell.borrow_mut().push(crate::PlatformRawEvent::Resized(Vec2::new(w, h)));
                });
            }
        }
    }) as Box<dyn FnMut(web_sys::Event)>);

    let _ = window
        .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref());
    ::std::mem::forget(closure);
}

// ---------------------------------------------------------------------------
// Drag and drop

fn install_drag_and_drop(target: &EventTarget) {
    for event_name in ["dragover", "dragleave", "drop"] {
        let target = target.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let _ = event.prevent_default();
            let _ = event.stop_propagation();
            let _de: web_sys::DragEvent = match event.dyn_into() {
                Ok(k) => k,
                Err(_) => return,
            };
            match event_name {
                "dragover" => {}
                "dragleave" => {
                    WEB_EVENTS.with(|cell| {
                        cell.borrow_mut().push(PlatformRawEvent::HoveredFileCancelled);
                    });
                }
                "drop" => {
                    WEB_EVENTS.with(|cell| {
                        cell.borrow_mut().push(PlatformRawEvent::DroppedFile(
                            crate::DroppedFile {
                                name: String::new(),
                                path: None,
                                bytes: None,
                            },
                        ));
                    });
                }
                _ => {}
            }
        }) as Box<dyn FnMut(web_sys::Event)>);

        let _ = target
            .add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref());
        ::std::mem::forget(closure);
    }
}

// ---------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translate_key_maps_common_keys() {
        assert_eq!(translate_key("Enter"), Some(Key::Enter));
        assert_eq!(translate_key(" "), Some(Key::Space));
        assert_eq!(translate_key("a"), Some(Key::KeyA));
        assert_eq!(translate_key("F1"), Some(Key::F1));
        assert_eq!(translate_key("Shift"), Some(Key::ShiftLeft));
        assert_eq!(translate_key("UnknownKey"), None);
    }

    #[test]
    fn pointer_button_mapping() {
        assert_eq!(pointer_button_from_button(0), Some(PointerButton::Primary));
        assert_eq!(pointer_button_from_button(2), Some(PointerButton::Secondary));
        assert_eq!(pointer_button_from_button(1), Some(PointerButton::Middle));
        assert_eq!(pointer_button_from_button(3), Some(PointerButton::Extra1));
        assert_eq!(pointer_button_from_button(4), Some(PointerButton::Extra2));
        assert_eq!(pointer_button_from_button(99), None);
    }

    #[test]
    fn drain_web_events_returns_queued() {
        WEB_EVENTS.with(|cell| {
            cell.borrow_mut().push(PlatformRawEvent::Focused(true));
        });
        let events = drain_web_events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], PlatformRawEvent::Focused(true)));
        // Queue is now empty.
        let events = drain_web_events();
        assert!(events.is_empty());
    }
}
