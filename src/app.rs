use std::sync::mpsc::{self, Receiver, Sender};

use crate::{load_family_data, setup_fonts, tree::TreeUi, Message};

pub struct App {
    tree: TreeUi,
    message_receiver: Receiver<Message>,
    message_sender: Sender<Message>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_fonts(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);

        // if let Some(storage) = cc.storage {
        //     return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        // }

        let (sender, receiver) = mpsc::channel();

        load_family_data(sender.clone());

        Self {
            tree: TreeUi::new(None),
            message_receiver: receiver,
            message_sender: sender,
        }
    }
}

impl eframe::App for App {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        // eframe::set_value(storage, eframe::APP_KEY, self);
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

        if let Ok(message) = self.message_receiver.try_recv() {
            match message {
                Message::LoadedFamilyData(root_node) => self.tree.set_root(Some(root_node)),
            }
        }
    }
}
