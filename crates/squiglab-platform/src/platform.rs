//! The Platform trait and its Winit-based implementation.
//!
//! Winit is the authoritative windowing system. All targets (native and
//! WebAssembly) go through Winit's event loop and window creation.

#![allow(dead_code)]

use std::time::Instant;

use crate::events::{PlatformEvent, PlatformInput, PlatformOutput, PlatformRawEvent, ViewportId};
use crate::window::Window;

/// The platform abstraction. Provides windowing, event-loop integration,
/// input translation, and renderer access to GPUI.
///
/// One `Platform` per application. Created via [`PlatformBuilder`].
pub trait Platform: Send + Sync {
    /// Start the event loop. Blocks until the application exits.
    /// Takes ownership via `Box<Self>` to be dyn-compatible.
    fn run(self: Box<Self>);

    /// Process one iteration of the event loop (non-blocking).
    /// Returns `false` when the application should exit.
    fn poll_events(&mut self) -> bool;

    /// Create a new window. Returns a trait object so the concrete window
    /// type can vary by backend.
    fn create_window(
        &mut self,
        attrs: &crate::window::WindowAttributes,
    ) -> Box<dyn crate::window::Window>;

    /// Deliver a raw platform event to the input state.
    fn on_event(&mut self, event: &PlatformRawEvent);

    /// Take the accumulated input for the current frame.
    fn take_input(&mut self, screen_rect: Option<crate::events::Rect>) -> PlatformInput;

    /// Handle platform output (clipboard, URLs, cursor, IME, etc.).
    fn handle_platform_output(&mut self, output: &PlatformOutput);

    /// Get the renderer for drawing.
    fn renderer(&mut self) -> &mut dyn crate::renderer::Renderer;
}

/// Builder for creating a [`Platform`].
pub struct PlatformBuilder {
    #[allow(dead_code)]
    native_pixels_per_point: Option<f32>,
}

impl PlatformBuilder {
    pub fn new() -> Self {
        Self {
            native_pixels_per_point: None,
        }
    }

    pub fn native_pixels_per_point(mut self, v: f32) -> Self {
        self.native_pixels_per_point = Some(v);
        self
    }

    /// Build the platform for the current target.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn build(self) -> Box<dyn Platform> {
        Box::new(crate::backends::native::NativePlatform::new(
            self.native_pixels_per_point,
        ))
    }

    /// Build the platform for WebAssembly.
    #[cfg(target_arch = "wasm32")]
    pub fn build(self) -> Box<dyn Platform> {
        Box::new(crate::backends::web::WebPlatform::new(
            self.native_pixels_per_point,
        ))
    }
}

impl Default for PlatformBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Winit-backed platform state (shared by native and web backends).
//

/// Holds the per-viewport input state. Modeled on egui-winit's `State`.
pub struct PlatformState {
    viewport_id: ViewportId,
    start_time: Instant,
    egui_input: PlatformInput,
    modifiers: Modifiers,
    pointer_pos_in_points: Option<Pos2>,
    any_pointer_button_down: bool,
    current_cursor_icon: Option<CursorIcon>,
    clipboard: Clipboard,
    allow_ime: bool,
}

use crate::clipboard::Clipboard;
use crate::events::PlatformOutputCommand;
use crate::events::{CursorIcon, Modifiers, Pos2};

impl PlatformState {
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

    pub fn platform_input(&self) -> &PlatformInput {
        &self.egui_input
    }

    pub fn platform_input_mut(&mut self) -> &mut PlatformInput {
        &mut self.egui_input
    }

    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    pub fn set_allow_ime(&mut self, allow: bool) {
        self.allow_ime = allow;
    }

    /// Translate a raw platform event into [`PlatformEvent`]s and accumulate
    /// them in the frame input.
    pub fn on_event(&mut self, event: &PlatformRawEvent) {
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
            }
            PlatformRawEvent::Text(text) => {
                self.egui_input
                    .events
                    .push(PlatformEvent::Text(text.clone()));
            }
            PlatformRawEvent::PointerMoved(pos) => {
                self.pointer_pos_in_points = Some(*pos);
                self.egui_input
                    .events
                    .push(PlatformEvent::PointerMoved(*pos));
            }
            PlatformRawEvent::PointerButton {
                pos,
                button,
                pressed,
                modifiers,
            } => {
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
            }
            PlatformRawEvent::PointerEntered(pos) => {
                self.pointer_pos_in_points = Some(*pos);
                self.egui_input
                    .events
                    .push(PlatformEvent::PointerEntered(*pos));
            }
            PlatformRawEvent::PointerLeft => {
                self.pointer_pos_in_points = None;
                self.egui_input.events.push(PlatformEvent::PointerLeft);
            }
            PlatformRawEvent::MouseWheel {
                unit,
                delta,
                modifiers,
            } => {
                self.modifiers = *modifiers;
                self.egui_input.modifiers = *modifiers;
                self.egui_input.events.push(PlatformEvent::MouseWheel {
                    unit: *unit,
                    delta: *delta,
                    modifiers: *modifiers,
                });
            }
            PlatformRawEvent::Touch {
                touch_id,
                pos,
                phase,
                modifiers,
            } => {
                self.modifiers = *modifiers;
                self.egui_input.modifiers = *modifiers;
                self.egui_input.events.push(PlatformEvent::Touch {
                    touch_id: *touch_id,
                    pos: *pos,
                    phase: *phase,
                    modifiers: *modifiers,
                });
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
                self.egui_input
                    .events
                    .push(PlatformEvent::Focused(*focused));
            }
            PlatformRawEvent::ScaleFactorChanged(scale) => {
                self.egui_input.native_pixels_per_point = Some(*scale);
                self.egui_input
                    .events
                    .push(PlatformEvent::ScaleFactorChanged(*scale));
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
                self.egui_input
                    .events
                    .push(PlatformEvent::Paste(text.clone()));
            }
            PlatformRawEvent::Cut => {
                self.egui_input.events.push(PlatformEvent::Cut);
            }
            PlatformRawEvent::Copy => {
                self.egui_input.events.push(PlatformEvent::Copy);
            }
            PlatformRawEvent::OpenUrl(url) => {
                self.egui_input
                    .events
                    .push(PlatformEvent::OpenUrl(url.clone()));
            }
        }
    }

    /// Prepare for a new frame by extracting the accumulated input.
    pub fn take_input(&mut self, screen_rect: Option<crate::events::Rect>) -> PlatformInput {
        let mut input = std::mem::take(&mut self.egui_input);
        input.time = Some(self.start_time.elapsed().as_secs_f64());
        input.screen_rect = screen_rect;
        input.viewport_id = self.viewport_id;
        input.modifiers = self.modifiers;
        input
    }

    /// Handle platform output from squiglab-core / squiglab-gpui.
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
                    let _ = title;
                }
                PlatformOutputCommand::Visible(_)
                | PlatformOutputCommand::Maximized(_)
                | PlatformOutputCommand::Minimized(_)
                | PlatformOutputCommand::InnerSize(_)
                | PlatformOutputCommand::SetTheme(_) => {
                    // Handled by the application crate.
                }
            }
        }
        self.current_cursor_icon = output.cursor_icon;
    }

    pub fn clipboard(&self) -> &Clipboard {
        &self.clipboard
    }

    pub fn clipboard_mut(&mut self) -> &mut Clipboard {
        &mut self.clipboard
    }
}

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
