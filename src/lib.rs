#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod tree;
pub use app::App;
use egui::{Modifiers, Pos2};

pub struct Input {
    hover_pos: Option<Pos2>,
    interact_pos: Option<Pos2>,
    scroll_delta: f32,
    // primary_pressed: bool,
    secondary_pressed: bool,
    modifiers: Modifiers,
}
