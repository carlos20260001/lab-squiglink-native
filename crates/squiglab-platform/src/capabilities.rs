//! Platform capabilities — common capability interface across targets.
//!
//! Platform-specific limitations are represented explicitly. Do not report
//! unsupported functionality as successful.

/// Capabilities supported by the current platform.
#[derive(Clone, Debug, Default)]
pub struct PlatformCapabilities {
    /// Whether GPU rendering (wgpu) is available.
    pub gpu_rendering: bool,
    /// Whether the system clipboard is available.
    pub clipboard: bool,
    /// Whether IME input is available.
    pub ime: bool,
    /// Whether fullscreen mode is available.
    pub fullscreen: bool,
    /// Whether file dialogs are available.
    pub file_dialogs: bool,
    /// Whether drag and drop is available.
    pub drag_and_drop: bool,
    /// Whether multiple windows are supported.
    pub multiple_windows: bool,
}

impl PlatformCapabilities {
    /// Capabilities for the native desktop target.
    pub fn native() -> Self {
        Self {
            gpu_rendering: true,
            clipboard: true,
            ime: true,
            fullscreen: true,
            file_dialogs: true,
            drag_and_drop: true,
            multiple_windows: true,
        }
    }

    /// Capabilities for the WebAssembly target.
    pub fn web() -> Self {
        Self {
            gpu_rendering: true, // WebGPU / WebGL2 via wgpu
            clipboard: true,     // Clipboard API (with user gesture)
            ime: true,           // Composition events
            fullscreen: false,   // Requires user gesture; not guaranteed
            file_dialogs: false, // <input type="file"> via wasm-bindgen
            drag_and_drop: true,
            multiple_windows: false, // Browser tabs, not true multiple windows
        }
    }
}
