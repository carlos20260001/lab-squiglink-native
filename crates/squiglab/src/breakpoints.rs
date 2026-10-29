//! Desktop layout breakpoints taken from `web/style.css`.
//!
//! These mirror the reference's media queries so the native layout can follow
//! the same responsive rules without a WebView. Values are in logical pixels,
//! assuming the reference's 16 px root font size (1 em = 16 px).

/// `max-width: 70em`
pub const MAX_WIDTH_70EM: f32 = 70.0 * 16.0;
/// `max-width: 60em`
pub const MAX_WIDTH_60EM: f32 = 60.0 * 16.0;
/// `max-width: 40em`
pub const MAX_WIDTH_40EM: f32 = 40.0 * 16.0;
/// `max-width: 800px`
pub const MAX_WIDTH_800PX: f32 = 800.0;

/// Aspect ratio thresholds from the reference.
pub const ASPECT_WIDE: f32 = 9.0 / 4.0;
pub const ASPECT_TALL: f32 = 5.0 / 3.0;

/// `max-height: 60vw`
pub const MAX_HEIGHT_60VW: f32 = 0.6; // fraction of viewport width
