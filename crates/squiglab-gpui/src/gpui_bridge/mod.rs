//! GPUI compatibility bridge.
//!
//! This module provides the integration layer between GPUI 0.2.2 and
//! squiglab-platform. GPUI owns the UI (layout, scene generation, text,
//! application architecture); squiglab-platform owns windowing, input, and
//! the Winit event loop. The bridge connects them without exposing egui
//! types or duplicating GPUI's rendering architecture.
//!
//! ```text
//!   GPUI 0.2.2
//!       |
//!       v
//!   squiglab-platform
//!       |
//!       v
//!     Winit
//!       |
//!   +---+---+---+
//!   |   |   | |
//! Lin Win Mac Wasm
//!
//! GPUI's internal `Platform` trait is `pub(crate)`, so a full implementation
//! requires either vendoring GPUI or a feature-gated compatibility boundary.
//! This module defines the bridge structure; the actual trait implementation
//! lives in `gpui/src/platform/squiglab/` when GPUI is vendored.

#![allow(dead_code)]

pub mod platform;
