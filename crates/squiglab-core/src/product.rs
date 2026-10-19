//! The product's identity and its assets.
//!
//! The name, the identifier, the mark and the typeface belong to the product
//! rather than to any picture of it, so they live here and every interface
//! draws the same ones. The build script renders the mark from the manifest
//! and hands the result over as these bytes.

/// The product's name, from the manifest.
pub const APP_NAME: &str = env!("CRINGRAPH_APP_NAME");

/// The product's identifier, from the manifest.
pub const APP_ID: &str = env!("CRINGRAPH_APP_ID");

/// The product's mark, rendered at build time as a 128-point PNG. This is the
/// icon a window carries.
pub const WINDOW_ICON_PNG: &[u8] = include_bytes!(env!("CRINGRAPH_WINDOW_ICON_PNG"));

/// The product's mark as a Windows icon resource.
pub const WINDOW_ICON_ICO: &[u8] = include_bytes!(env!("CRINGRAPH_WINDOW_ICON_ICO"));

/// The product's typeface, regular.
pub const FONT_REGULAR: &[u8] = include_bytes!("../assets/fonts/OpenSans-Regular.ttf");

/// The product's typeface, bold.
pub const FONT_BOLD: &[u8] = include_bytes!("../assets/fonts/OpenSans-Bold.ttf");

/// Where the manifest and the assets live.
pub const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

/// The manifest itself.
pub const MANIFEST_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/packaging/app.toml");
