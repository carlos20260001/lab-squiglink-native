//! Opening a link in the user's browser.
//!
//! The reference's row of other graphs' tools is a row of links to other
//! people's sites. A window has no pages to go to, so they are opened in the
//! browser the user already has.

use std::process::Command;

/// Open a link in the user's browser.
///
/// Nothing is said if the browser cannot be opened: the link is still there
/// to copy, and a window with no browser is not a broken program.
pub fn open(url: &str) {
    let result = if cfg!(target_os = "macos") {
        Command::new("open").arg(url).spawn()
    } else if cfg!(target_os = "windows") {
        Command::new("rundll32")
            .arg("url.dll,FileProtocolHandler")
            .arg(url)
            .spawn()
    } else {
        Command::new("xdg-open").arg(url).spawn()
    };
    let _ = result;
}
