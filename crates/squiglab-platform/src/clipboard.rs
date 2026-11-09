//! System clipboard abstraction.
//! Modeled on egui-winit's clipboard module: one abstraction, platform
//! implementations behind feature flags.

use crate::events::DroppedFile;

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

impl DroppedFile {
    /// Create a dropped file from a path.
    pub fn from_path(path: std::path::PathBuf) -> Self {
        Self {
            name: path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_owned(),
            path: Some(path),
            bytes: None,
        }
    }
}
