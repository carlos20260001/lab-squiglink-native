//! Multiplatform windowing, input and clipboard plugin for squiglab.
//!
//! This crate follows the same architecture as [`egui-winit`]: the core
//! (`squiglab-core`) is platform-agnostic, and this crate translates
//! platform events (winit WindowEvents, web DOM events, iOS objc2 events,
//! Android JNI events) into squiglab input events that the core and the
//! GPUI interface can consume without knowing what platform they run on.
//!
//! # Architecture
//!
//! ```text
//!  platform event (winit WindowEvent / web DOM event)
//!         │
//!         ▼
//!   PlatformState::on_event()  ──►  PlatformEvent
//!         │
//!         ▼
//!   PlatformInput (accumulated per frame)
//!         │
//!         ▼
//!   PlatformState::take_input() ──► squiglab-core / squiglab-gpui
//! ```
//!
//! # Backends
//!
//! Feature flags select the backend:
//! * `default`  — desktop (window handle via `raw-window-handle`)
//! * `web`      — wasm32 via `web_sys` / `wasm-bindgen` (like eframe's web)
//! * `ios`      — objc2 (like egui-winit on iOS)
//! * `android`  — jni + ndk (like egui-winit on Android)
//! * `winit`    — winit backend (stub; winit not in the offline lock yet)

//! ---------------------------------------------------------------------------
//! Input event types
//!
//! Modeled on [`egui::Event`]: one enum of all the input events the platform
//! can produce, consumed by [`PlatformState`] and turned into
//! [`PlatformInput`] for the frame.

use std::time::Instant;

/// One input event from the platform.
///
/// Mirrors [`egui::Event`] but uses squiglab's own types where the core
// already has them (e.g. [`ZoomRegion`], [`EqBank`]).
#[derive(Clone, Debug, PartialEq)]
pub enum PlatformEvent {
    /// A key was pressed or released.
    Key {
        key: Key,
        physical_key: Option<Key>,
        pressed: bool,
        repeat: bool,
        modifiers: Modifiers,
    },
    /// A character was typed (text input).
    Text(String),
    /// The pointer (mouse / touch / pen) moved.
    PointerMoved(Pos2),
    /// A pointer button was pressed or released.
    PointerButton {
        pos: Pos2,
        button: PointerButton,
        pressed: bool,
        modifiers: Modifiers,
    },
    /// The pointer entered the window.
    PointerEntered(Pos2),
    /// The pointer left the window.
    PointerLeft,
    /// The pointer wheel was scrolled.
    MouseWheel {
        unit: MouseWheelUnit,
        delta: Vec2,
        modifiers: Modifiers,
    },
    /// A touch event began / moved / ended / was cancelled.
    Touch {
        touch_id: u64,
        pos: Pos2,
        phase: TouchPhase,
        modifiers: Modifiers,
    },
    /// The modifiers (ctrl / shift / alt / command) changed.
    ModifiersChanged(Modifiers),
    /// The window was resized (physical pixels).
    Resized(Vec2),
    /// The window was focused or unfocused.
    Focused(bool),
    /// The window's scale factor (DPR) changed.
    ScaleFactorChanged(f32),
    /// The system theme (light / dark) changed.
    ThemeChanged(Theme),
    /// A file was dropped onto the window.
    DroppedFile(DroppedFile),
    /// A file is being hovered over the window.
    HoveredFile(DroppedFile),
    /// A file hover was cancelled.
    HoveredFileCancelled,
    /// Text was pasted.
    Paste(String),
    /// Text was cut.
    Cut,
    /// Text was copied.
    Copy,
    /// The user wants to open a URL in the system browser.
    OpenUrl(String),
}

/// A keyboard key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    Escape,
    Tab,
    Backspace,
    Enter,
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Space,
    Comma,
    Period,
    Semicolon,
    Slash,
    Backslash,
    OpenBracket,
    CloseBracket,
    Backtick,
    Quote,
    Minus,
    Equals,
    Plus,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    F25,
    F26,
    F27,
    F28,
    F29,
    F30,
    F31,
    F32,
    F33,
    F34,
    F35,
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,
    SuperLeft,
    SuperRight,
    IntlBackslash,
    Cut,
    Copy,
    Paste,
    BrowserBack,
    /// Any other key, identified by its name string.
    Other(&'static str),
}

/// Pointer (mouse / touch) button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
    Extra1,
    Extra2,
}

/// Modifier key state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub command: bool,
    pub mac_cmd: bool,
}

/// The unit of a mouse wheel delta.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseWheelUnit {
    Point,
    Line,
    Page,
}

/// The phase of a touch event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TouchPhase {
    Start,
    Move,
    End,
    Cancel,
}

/// A 2D point in points (not pixels).
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Default)]
pub struct Pos2 {
    pub x: f32,
    pub y: f32,
}

impl Pos2 {
    pub const ZERO: Pos2 = Pos2 { x: 0.0, y: 0.0 };

    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A 2D vector in points.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// The system theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Theme {
    Light,
    Dark,
}

/// A file that was dropped or hovered over the window.
#[derive(Clone, Debug, PartialEq)]
pub struct DroppedFile {
    pub name: String,
    /// Where the file is, if there is a filesystem path for it.
    pub path: Option<std::path::PathBuf>,
    /// The bytes of the file, if there is no filesystem path (e.g. web).
    pub bytes: Option<Vec<u8>>,
}

// ---------------------------------------------------------------------------
// Platform input: everything accumulated for one frame.
// Modeled on [`egui::RawInput`].

/// Everything the platform knows about this frame: time, screen rect,
/// events, modifiers, and viewports.
///
/// Consumed by [`PlatformState::take_input`] and passed on to
/// squiglab-core / squiglab-gpui.
#[derive(Clone, Debug, Default)]
pub struct PlatformInput {
    /// The time since the application started, in seconds.
    pub time: Option<f64>,
    /// The screen rectangle in points.
    pub screen_rect: Option<Rect>,
    /// The viewport this input is for.
    pub viewport_id: ViewportId,
    /// All input events this frame.
    pub events: Vec<PlatformEvent>,
    /// The modifiers at the end of the frame.
    pub modifiers: Modifiers,
    /// The system theme.
    pub system_theme: Option<Theme>,
    /// The native pixels per point (DPR * zoom).
    pub native_pixels_per_point: Option<f32>,
    /// Files currently being hovered over the window.
    pub hovered_files: Vec<DroppedFile>,
    /// Files dropped this frame.
    pub dropped_files: Vec<DroppedFile>,
}

/// A 2D rectangle in points.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Rect {
    pub min: Pos2,
    pub max: Pos2,
}

impl Rect {
    pub fn from_min_size(min: Pos2, size: Vec2) -> Self {
        Self {
            max: Pos2::new(min.x + size.x, min.y + size.y),
            min,
        }
    }

    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }
}

/// An identifier for a viewport (window).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct ViewportId(pub u64);

impl ViewportId {
    /// The root viewport.
    pub const ROOT: Self = Self(0);
}

// ---------------------------------------------------------------------------
// Platform output: things squiglab-core / squiglab-gpui ask the platform to do.
// Modeled on [`egui::PlatformOutput`].

/// Commands the application can issue to the platform.
#[derive(Clone, Debug)]
pub enum PlatformOutputCommand {
    /// Copy text to the clipboard.
    CopyText(String),
    /// Open a URL in the system browser.
    OpenUrl(String),
    /// Close the window / viewport.
    Close,
    /// Set the window title.
    Title(String),
    /// Set whether the window is visible.
    Visible(bool),
    /// Set whether the window is maximized.
    Maximized(bool),
    /// Set whether the window is minimized.
    Minimized(bool),
    /// Set the window's inner size in points.
    InnerSize(Vec2),
    /// Set the window theme.
    SetTheme(Theme),
}

/// Everything the platform output can contain.
#[derive(Clone, Debug, Default)]
pub struct PlatformOutput {
    pub commands: Vec<PlatformOutputCommand>,
    /// The cursor icon to show.
    pub cursor_icon: Option<CursorIcon>,
    /// The IME purpose and rect, if IME input is active.
    pub ime: Option<Ime>,
}

/// The cursor icon to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CursorIcon {
    Default,
    Crosshair,
    Hand,
    Text,
    ResizeHorizontal,
    ResizeVertical,
    Grab,
    Grabbing,
    None,
}

/// IME input state.
#[derive(Clone, Debug)]
pub struct Ime {
    pub rect: Rect,
    pub purpose: ImePurpose,
    pub should_interrupt_composition: bool,
}

/// The purpose of IME input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImePurpose {
    Normal,
    Password,
    Terminal,
}

// ---------------------------------------------------------------------------
// PlatformState: the main integration object.
// Modeled on [`egui_winit::State`]: holds the egui-like context, accumulates
// input events, and translates platform events into squiglab input events.

/// Handles the integration between the platform and squiglab.
///
/// Instantiate one per window / viewport. Call [`Self::on_event`] for each
/// platform event, then [`Self::take_input`] each frame to get the
/// accumulated [`PlatformInput`].
pub struct PlatformState {
    /// The viewport this state belongs to.
    viewport_id: ViewportId,
    /// When the application started.
    start_time: Instant,
    /// The accumulated input for the current frame.
    egui_input: PlatformInput,
    /// The current modifier state.
    modifiers: Modifiers,
    /// The current pointer position in points, if inside the window.
    pointer_pos_in_points: Option<Pos2>,
    /// Whether any pointer button is currently down.
    any_pointer_button_down: bool,
    /// The current cursor icon.
    current_cursor_icon: Option<CursorIcon>,
    /// The clipboard.
    clipboard: Clipboard,
    /// Whether IME input is allowed.
    allow_ime: bool,
}

impl PlatformState {
    /// Construct a new instance.
    pub fn new(viewport_id: ViewportId, native_pixels_per_point: Option<f32>) -> Self {
        Self {
            viewport_id,
            start_time: Instant::now(),
            egui_input: PlatformInput {
                viewport_id,
                native_pixels_per_point,
                ..Default::default()
            },
            modifiers: Modifiers::default(),
            pointer_pos_in_points: None,
            any_pointer_button_down: false,
            current_cursor_icon: None,
            clipboard: Clipboard::new(),
            allow_ime: false,
        }
    }

    /// Returns a reference to the accumulated input.
    pub fn platform_input(&self) -> &PlatformInput {
        &self.egui_input
    }

    /// Returns a mutable reference to the accumulated input.
    pub fn platform_input_mut(&mut self) -> &mut PlatformInput {
        &mut self.egui_input
    }

    /// The current modifiers.
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    /// Set the last value that [`Window::set_ime_allowed`] was called with (for debouncing).
    pub fn set_allow_ime(&mut self, allow: bool) {
        self.allow_ime = allow;
    }

    /// Call this when there is a new platform event.
    ///
    /// Translates the platform event into one or more [`PlatformEvent`]s and
    /// accumulates them in the frame input.
    pub fn on_event(&mut self, event: &PlatformRawEvent) -> EventResponse {
        let mut consumed = false;
        let repaint = true;

        match event {
            PlatformRawEvent::Key(key_event) => {
                self.modifiers = key_event.modifiers;
                self.egui_input.modifiers = key_event.modifiers;
                self.egui_input.events.push(PlatformEvent::Key {
                    key: key_event.key,
                    physical_key: key_event.physical_key,
                    pressed: key_event.pressed,
                    repeat: key_event.repeat,
                    modifiers: key_event.modifiers,
                });
                consumed = true;

            }
            PlatformRawEvent::Text(text) => {
                self.egui_input.events.push(PlatformEvent::Text(text.clone()));
                consumed = true;

            }
            PlatformRawEvent::PointerMoved(pos) => {
                self.pointer_pos_in_points = Some(*pos);
                self.egui_input.events.push(PlatformEvent::PointerMoved(*pos));

            }
            PlatformRawEvent::PointerButton { pos, button, pressed, modifiers } => {
                self.modifiers = *modifiers;
                self.egui_input.modifiers = *modifiers;
                self.any_pointer_button_down = *pressed;
                self.pointer_pos_in_points = Some(*pos);
                self.egui_input.events.push(PlatformEvent::PointerButton {
                    pos: *pos,
                    button: *button,
                    pressed: *pressed,
                    modifiers: *modifiers,
                });
                consumed = true;

            }
            PlatformRawEvent::PointerEntered(pos) => {
                self.pointer_pos_in_points = Some(*pos);
                self.egui_input.events.push(PlatformEvent::PointerEntered(*pos));

            }
            PlatformRawEvent::PointerLeft => {
                self.pointer_pos_in_points = None;
                self.egui_input.events.push(PlatformEvent::PointerLeft);

            }
            PlatformRawEvent::MouseWheel { unit, delta, modifiers } => {
                self.modifiers = *modifiers;
                self.egui_input.modifiers = *modifiers;
                self.egui_input.events.push(PlatformEvent::MouseWheel {
                    unit: *unit,
                    delta: *delta,
                    modifiers: *modifiers,
                });

            }
            PlatformRawEvent::Touch { touch_id, pos, phase, modifiers } => {
                self.modifiers = *modifiers;
                self.egui_input.modifiers = *modifiers;
                self.egui_input.events.push(PlatformEvent::Touch {
                    touch_id: *touch_id,
                    pos: *pos,
                    phase: *phase,
                    modifiers: *modifiers,
                });
                consumed = true;

            }
            PlatformRawEvent::ModifiersChanged(modifiers) => {
                self.modifiers = *modifiers;
                self.egui_input.modifiers = *modifiers;
                self.egui_input
                    .events
                    .push(PlatformEvent::ModifiersChanged(*modifiers));

            }
            PlatformRawEvent::Resized(size) => {
                self.egui_input.events.push(PlatformEvent::Resized(*size));

            }
            PlatformRawEvent::Focused(focused) => {
                self.egui_input.events.push(PlatformEvent::Focused(*focused));

            }
            PlatformRawEvent::ScaleFactorChanged(scale) => {
                self.egui_input.native_pixels_per_point = Some(*scale);
                self.egui_input.events.push(PlatformEvent::ScaleFactorChanged(*scale));

            }
            PlatformRawEvent::ThemeChanged(theme) => {
                self.egui_input.system_theme = Some(*theme);
                self.egui_input
                    .events
                    .push(PlatformEvent::ThemeChanged(*theme));

            }
            PlatformRawEvent::DroppedFile(file) => {
                self.egui_input.dropped_files.push(file.clone());

            }
            PlatformRawEvent::HoveredFile(file) => {
                self.egui_input.hovered_files.push(file.clone());

            }
            PlatformRawEvent::HoveredFileCancelled => {
                self.egui_input.hovered_files.clear();

            }
            PlatformRawEvent::Paste(text) => {
                self.egui_input.events.push(PlatformEvent::Paste(text.clone()));

            }
            PlatformRawEvent::Cut => {
                self.egui_input.events.push(PlatformEvent::Cut);

            }
            PlatformRawEvent::Copy => {
                self.egui_input.events.push(PlatformEvent::Copy);

            }
            PlatformRawEvent::OpenUrl(url) => {
                self.egui_input.events.push(PlatformEvent::OpenUrl(url.clone()));

            }
        }

        EventResponse { consumed, repaint }
    }

    /// Prepare for a new frame by extracting the accumulated input.
    ///
    /// Sets the time and screen rect, then takes the input so the next frame
    /// starts fresh.
    pub fn take_input(&mut self, screen_rect: Option<Rect>) -> PlatformInput {
        let mut input = std::mem::take(&mut self.egui_input);
        input.time = Some(self.start_time.elapsed().as_secs_f64());
        input.screen_rect = screen_rect;
        input.viewport_id = self.viewport_id;
        input.modifiers = self.modifiers;
        input
    }

    /// Handle platform output from squiglab-core / squiglab-gpui.
    ///
    /// Executes commands such as copying to the clipboard or opening a URL.
    pub fn handle_platform_output(&mut self, output: &PlatformOutput) {
        for command in &output.commands {
            match command {
                PlatformOutputCommand::CopyText(text) => {
                    self.clipboard.set_text(text);
                }
                PlatformOutputCommand::OpenUrl(url) => {
                    open_url_in_browser(url);
                }
                PlatformOutputCommand::Close => {
                    // Handled by the application crate.
                }
                PlatformOutputCommand::Title(title) => {
                    // Handled by the application crate.
                    let _ = title;
                }
                PlatformOutputCommand::Visible(_) | PlatformOutputCommand::Maximized(_) | PlatformOutputCommand::Minimized(_) | PlatformOutputCommand::InnerSize(_) | PlatformOutputCommand::SetTheme(_) => {
                    // Handled by the application crate.
                }
            }
        }
        self.current_cursor_icon = output.cursor_icon;
    }

    /// The clipboard.
    pub fn clipboard(&self) -> &Clipboard {
        &self.clipboard
    }

    /// The clipboard, mutably.
    pub fn clipboard_mut(&mut self) -> &mut Clipboard {
        &mut self.clipboard
    }
}

/// The response to processing a platform event.
#[derive(Clone, Copy, Debug, Default)]
pub struct EventResponse {
    /// Whether egui consumed this event (wants exclusive use).
    pub consumed: bool,
    /// Whether an egui repaint is needed.
    pub repaint: bool,
}

/// A raw platform event, before translation into [`PlatformEvent`].
///
/// Each backend converts its own event type into this.
#[derive(Clone, Debug)]
pub enum PlatformRawEvent {
    Key(KeyEvent),
    Text(String),
    PointerMoved(Pos2),
    PointerButton {
        pos: Pos2,
        button: PointerButton,
        pressed: bool,
        modifiers: Modifiers,
    },
    PointerEntered(Pos2),
    PointerLeft,
    MouseWheel {
        unit: MouseWheelUnit,
        delta: Vec2,
        modifiers: Modifiers,
    },
    Touch {
        touch_id: u64,
        pos: Pos2,
        phase: TouchPhase,
        modifiers: Modifiers,
    },
    ModifiersChanged(Modifiers),
    Resized(Vec2),
    Focused(bool),
    ScaleFactorChanged(f32),
    ThemeChanged(Theme),
    DroppedFile(DroppedFile),
    HoveredFile(DroppedFile),
    HoveredFileCancelled,
    Paste(String),
    Cut,
    Copy,
    OpenUrl(String),
}

/// A keyboard event.
#[derive(Clone, Debug)]
pub struct KeyEvent {
    pub key: Key,
    pub physical_key: Option<Key>,
    pub pressed: bool,
    pub repeat: bool,
    pub modifiers: Modifiers,
}

// ---------------------------------------------------------------------------
// Clipboard
// Modeled on egui-winit's clipboard module: one abstraction, platform
// implementations behind feature flags.

/// The system clipboard.
#[derive(Clone, Debug)]
pub struct Clipboard {
    text: Option<String>,
}

impl Clipboard {
    pub fn new() -> Self {
        Self { text: None }
    }

    /// Get the current clipboard text, if any.
    pub fn get(&self) -> Option<String> {
        self.text.clone()
    }

    /// Set the clipboard text.
    pub fn set_text(&mut self, text: &str) {
        self.text = Some(text.to_owned());
    }
}

impl Default for Clipboard {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Platform output helpers

fn open_url_in_browser(url: &str) {
    #[cfg(feature = "web")]
    {
        if let Some(win) = web_sys::window() {
            let _ = win.open_with_url_and_target(url, "_blank");
        }
    }

    #[cfg(not(feature = "web"))]
    {
        let _ = url;
        log::warn!("Cannot open url - platform does not support it.");
    }
}

// ---------------------------------------------------------------------------
// Backends

#[cfg(feature = "web")]
mod web_backend;
#[cfg(feature = "ios")]
mod ios_backend;
#[cfg(feature = "android")]
mod android_backend;
#[cfg(feature = "winit")]
mod winit_backend;

// ---------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_input_accumulates_events() {
        let mut state = PlatformState::new(ViewportId::ROOT, None);

        let response = state.on_event(&PlatformRawEvent::Key(KeyEvent {
            key: Key::KeyA,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::default(),
        }));
        assert!(response.repaint);

        let input = state.take_input(None);
        assert_eq!(input.events.len(), 1);
        assert!(matches!(input.events[0], PlatformEvent::Key { key: Key::KeyA, .. }));
        assert_eq!(input.modifiers, Modifiers::default());
    }

    #[test]
    fn modifiers_changed_propagate() {
        let mut state = PlatformState::new(ViewportId::ROOT, None);

        state.on_event(&PlatformRawEvent::ModifiersChanged(Modifiers {
            ctrl: true,
            ..Default::default()
        }));

        let input = state.take_input(None);
        assert_eq!(
            input.modifiers,
            Modifiers { ctrl: true, ..Default::default() }
        );
    }
}
