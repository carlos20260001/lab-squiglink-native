//! Platform backends.
//!
//! `native` — Winit-based desktop platform (Linux, Windows, macOS).
//! `web` — Winit-based WebAssembly platform (browser canvas).

#![allow(dead_code)]

#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub mod web;

// On wasm32 without the `web` feature, no backend is available — the
// application must enable `--features web` to build for the browser.
#[cfg(all(target_arch = "wasm32", not(feature = "web")))]
mod _no_web_backend;
