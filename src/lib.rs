#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod tree;
mod zoom;
pub use app::App;
use egui::Pos2;

pub struct Input {
    hover_pos: Option<Pos2>,
    scroll_delta: f32,
}

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "arial".to_owned(),
        egui::FontData::from_static(include_bytes!("../fonts/arial.ttf")),
    );

    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "arial".to_owned());

    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("arial".to_owned());

    ctx.set_fonts(fonts);
}
