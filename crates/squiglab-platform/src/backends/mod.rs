//! Platform backends.
//!
//! `native` — Winit-based desktop platform (Linux, Windows, macOS).
//! `web` — Winit-based WebAssembly platform (browser canvas).

#![allow(dead_code)]

#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[cfg(target_arch = "wasm32")]
pub mod web;
