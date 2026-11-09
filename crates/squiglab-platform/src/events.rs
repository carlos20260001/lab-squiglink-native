//! Input event types.
//!
//! Modeled on [`egui::Event`]: one enum of all the input events the platform
//! can produce, consumed by [`PlatformState`] and turned into
//! [`PlatformInput`] for the frame.
//!
//! Uses squiglab's own types where the core already has them (e.g.
//! [`ZoomRegion`](squiglab_core::graph::ZoomRegion)).

use std::path::PathBuf;

/// One input event from the platform.
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
    pub path: Option<PathBuf>,
    /// The bytes of the file, if there is no filesystem path (e.g. web).
    pub bytes: Option<Vec<u8>>,
}

// ---------------------------------------------------------------------------
// Platform input: everything accumulated for one frame.
// Modeled on [`egui::RawInput`].

/// Everything the platform knows about this frame: time, screen rect,
/// events, modifiers, and viewports.
///
/// Consumed by [`Platform::take_input`] and passed on to
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

/// Key modifiers (alias for Modifiers, kept for API clarity).
pub type KeyModifiers = Modifiers;
