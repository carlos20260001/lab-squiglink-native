//! The program core.
//!
//! Everything here is the program itself rather than its interface: the
//! measurements, the filter maths, the audio, and the rules that turn one into
//! another. No module in this crate draws anything, and none of them know what
//! a widget is, so the interface can be rewritten without disturbing them.

pub mod audio;
pub mod branding;
pub mod color;
pub mod config;
pub mod constraints;
pub mod data;
pub mod delta;
pub mod equalizer;
pub mod graph;
pub mod processing;
pub mod product;
pub mod session;
pub mod state;
pub mod theme;
