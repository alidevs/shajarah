use egui::Sense;

use crate::{tree::Node, Input};

lazy_static::lazy_static! {
    static ref FAMILY_DATA: Node = Node::new(1, vec![
        Node::new(2, vec![
            Node::new(6, vec![]),
            Node::new(7, vec![]),
            Node::new(9, vec![]),
        ]),
        Node::new(3, vec![]),
        Node::new(4, vec![
            Node::new(10, vec![
                // Node::new(12, vec![]),
                // Node::new(13, vec![]),
            ]),
            Node::new(11, vec![]),
        ]),
        Node::new(5, vec![])
    ]);
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct App {
    label: String,

    #[serde(skip)]
    value: f32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            label: "Hello World!".to_owned(),
            value: 2.7,
        }
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // TODO: might customize the look with `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut bg_rect = ui.allocate_rect(ui.max_rect(), Sense::click_and_drag());
            let viewport = bg_rect.rect;
            ui.set_clip_rect(viewport);

            let input = ui.ctx().input(|i| Input {
                scroll_delta: i.raw_scroll_delta.y,
                hover_pos: i.pointer.hover_pos(),
                interact_pos: i.pointer.interact_pos(),
                modifiers: i.modifiers,
                // primary_pressed: i.pointer.primary_pressed(),
                secondary_pressed: i.pointer.secondary_pressed(),
            });

            let center = viewport.center();
            FAMILY_DATA.draw(ui, center.x, center.y);
        });
    }
}
