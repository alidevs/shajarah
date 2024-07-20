use crate::{
    setup_fonts,
    tree::{Node, TreeUi},
};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct App {
    #[serde(skip)]
    tree: TreeUi,
}

impl Default for App {
    fn default() -> Self {
        Self {
            tree: TreeUi::new(Node::new(
                1,
                vec![
                    Node::new(
                        2,
                        vec![
                            Node::new(6, vec![]),
                            Node::new(7, vec![]),
                            Node::new(9, vec![]),
                        ],
                    ),
                    Node::new(3, vec![]),
                    Node::new(
                        4,
                        vec![
                            Node::new(
                                10,
                                vec![
                                    // Node::new(12, vec![]),
                                    // Node::new(13, vec![]),
                                ],
                            ),
                            Node::new(11, vec![]),
                        ],
                    ),
                    Node::new(5, vec![]),
                ],
            )),
        }
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_fonts(&cc.egui_ctx);

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
            self.tree.draw(ui);
        });
    }
}
