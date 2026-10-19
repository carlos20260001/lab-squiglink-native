//! The interface: the window, the panels, and the widgets they are drawn with.
//!
//! It draws the program and knows nothing about how the program works. The
//! rules, the state and the data are in `squiglab-core`, which does not depend
//! on egui and could be drawn by anything.

pub mod app;
pub mod curves;
pub mod equalizer;
pub mod graph;
pub mod header;
pub mod selector;
pub mod sound;
pub mod theme;
pub mod tools;
