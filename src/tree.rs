use ar_reshaper::{config::LigaturesFlags, ArabicReshaper, ReshaperConfig};
use chrono::{DateTime, Utc};

#[cfg(feature = "debug-ui")]
use egui::Stroke;

use egui::{
    epaint::CubicBezierShape, text::LayoutJob, Align, Color32, FontFamily, FontId, PointerButton,
    Pos2, Rect, Rounding, Sense, Shape, TextFormat, Vec2, Vec2b,
};
use serde::{Deserialize, Serialize};

use crate::{zoom::Zoom, Gender};

const NODE_RADIUS: f32 = 30.;
const NODE_PADDING: f32 = 10. * 10.;
const NODE_TEXT_PADDING: f32 = 10.;
const MAX_SCALE: f32 = 5.0;
const MIN_SCALE: f32 = 0.2;
const RESHAPER: ArabicReshaper = ArabicReshaper::new(ReshaperConfig::new(
    ar_reshaper::Language::Arabic,
    LigaturesFlags::default(),
));

pub struct TreeUi {
    offset: Vec2,
    centered: bool,
    scale: f32,
    root: Option<Node>,
}

impl TreeUi {
    pub fn new(root: Option<Node>) -> Self {
        Self {
            offset: Vec2::ZERO,
            centered: false,
            scale: 1.,
            root,
        }
    }

    pub fn set_root(&mut self, root: Option<Node>) {
        self.root = root;
    }

    pub fn draw(&mut self, ui: &mut egui::Ui) {
        let bg_rect = ui.allocate_rect(ui.max_rect(), Sense::click_and_drag());
        let viewport = bg_rect.rect;
        ui.set_clip_rect(viewport);

        ui.style_mut().zoom(self.scale);

        if bg_rect.dragged_by(PointerButton::Primary) {
            self.pan(bg_rect.drag_delta());
        }

        if !self.centered {
            self.offset = viewport.center().to_vec2();
            self.centered = true;
        }

        if let Some(hover_pos) = ui.ctx().input(|i| i.pointer.hover_pos()) {
            if bg_rect.hovered() {
                let zoom_delta = ui.ctx().input(|i| i.zoom_delta());
                let prev_scale = self.scale;
                let new_scale = (prev_scale * zoom_delta).clamp(MIN_SCALE, MAX_SCALE);

                self.scale(new_scale);
                let scale_factor = self.scale / prev_scale;
                let pos = self.offset - hover_pos.to_vec2();

                self.offset = (pos * scale_factor) + hover_pos.to_vec2();
            }
        }

        if let Some(root) = &mut self.root {
            root.draw(ui, self.offset.to_pos2(), self.scale, vec![]);
        }
    }

    fn scale(&mut self, new_scale: f32) {
        self.scale = new_scale;
    }

    fn pan(&mut self, delta: Vec2) {
        self.offset += delta;
    }

    // fn screen_pos_to_graph(&self, pos: Pos2, viewport: Rect) -> Pos2 {
    //     (pos + self.offset - viewport.center().to_vec2()) / self.scale
    // }
}

// #[derive(Serialize, Deserialize)]
// pub struct Node {
//     id: usize,
//     name: String,
//     #[serde(skip)]
//     window_is_open: bool,

//     children: Vec<Node>,
// }

#[derive(Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: i32,
    name: String,
    gender: Gender,
    birthday: Option<DateTime<Utc>>,
    last_name: String,
    pub children: Vec<Node>,

    #[serde(skip)]
    window_is_open: bool,
}

impl Node {
    // pub fn add_child(&mut self, child: Node) {
    //     self.children.push(child);
    // }

    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        offset: Pos2,
        scale: f32,
        mut lineage: Vec<SimpleNode>,
    ) {
        let stroke = ui.visuals().widgets.noninteractive.fg_stroke;
        let painter = ui.painter();

        let default_text_style = egui::style::default_text_styles()
            .get(&egui::TextStyle::Monospace)
            .map(|f| FontId::new(f.size * scale, f.family.clone()))
            .unwrap_or(FontId::new(14.0 * scale, FontFamily::Monospace));

        let mut job = LayoutJob::default();
        job.append(
            &RESHAPER
                .reshape(self.name.clone())
                .chars()
                .rev()
                .collect::<String>(),
            0.0,
            TextFormat {
                font_id: default_text_style.clone(),
                color: ui.visuals().text_color(),
                ..Default::default()
            },
        );
        let galley = painter.layout_job(job);

        let galley_c = galley.clone();

        let text_x =
            (offset.x - (NODE_RADIUS * scale + galley.size().x)) - NODE_TEXT_PADDING * scale;
        let text_y = offset.y - (galley.size().y / 2.);

        painter.galley(Pos2::new(text_x, text_y), galley, Color32::WHITE);

        let image_rect = Rect::from_center_size(
            offset,
            (Vec2::splat(NODE_RADIUS * 2.) * scale) + Vec2::splat(1.0), // add one pixel to cover the whole background circle
        );

        let response = ui.allocate_rect(image_rect, Sense::click());
        let clicked = response.clicked();
        if clicked {
            self.window_is_open = !self.window_is_open;
        }

        let window_pos = offset + Vec2::new(NODE_RADIUS * 1.2, -(NODE_RADIUS / 2.)) * scale;

        egui::Window::new(self.id.to_string())
            .id(egui::Id::new(self.id))
            .max_width(150.)
            .auto_sized()
            .resizable(false)
            .constrain(false)
            .default_pos(window_pos)
            .collapsible(false)
            .title_bar(false)
            .scroll(Vec2b::TRUE)
            .enabled(true)
            .open(&mut self.window_is_open)
            .current_pos(window_pos)
            .show(ui.ctx(), |ui| {
                ui.with_layout(egui::Layout::top_down(Align::RIGHT), |ui| {
                    let lineage = lineage
                        .iter()
                        .rev()
                        .map(|l| format!("{} ", l.name.clone()))
                        .take(2)
                        .collect::<String>();
                    ui.label(
                        RESHAPER
                            .reshape(format!("{} {}{}", self.name, lineage, self.last_name))
                            .chars()
                            .rev()
                            .collect::<String>(),
                    );
                });
            });

        if self.children.is_empty() {
            let painter = ui.painter();
            painter.circle_filled(offset, NODE_RADIUS * scale, Color32::LIGHT_BLUE);

            #[cfg(feature = "debug-ui")]
            painter.rect_stroke(image_rect, Rounding::ZERO, Stroke::new(2.0, Color32::GREEN));

            egui::Image::from_uri("https://r2.bksalman.com/ppL.webp")
                .rounding(Rounding::same(NODE_RADIUS * 2.) * scale)
                .paint_at(ui, image_rect);

            if response.hovered() {
                let painter = ui.painter();
                painter.circle_stroke(offset, NODE_RADIUS * scale, stroke);
            }

            #[cfg(feature = "debug-ui")]
            painter.rect_stroke(
                Rect {
                    min: Pos2::new(text_x, text_y),
                    max: Pos2::new(text_x + galley_c.size().x, text_y + galley_c.size().y),
                },
                Rounding::ZERO,
                Stroke::new(1., Color32::GREEN),
            );

            return;
        }

        let mut child_x = offset.x - ((self.children_shift() / 2.) * scale);
        let child_y = offset.y + ((NODE_RADIUS * 2. + NODE_PADDING) * scale);
        // draw lines
        for child in self.children.iter() {
            let painter = ui.painter();

            if child_x + NODE_RADIUS == offset.x {
                painter.line_segment(
                    [Pos2::new(child_x + (NODE_RADIUS * scale), child_y), offset],
                    stroke,
                );
            } else {
                let control_point1 = Pos2::new(offset.x, child_y - (NODE_PADDING * scale));

                #[cfg(feature = "debug-ui")]
                painter.circle_filled(control_point1, 10., Color32::WHITE);

                let control_point2 = Pos2::new(
                    child_x + (NODE_RADIUS * scale),
                    offset.y + (NODE_PADDING * scale),
                );

                #[cfg(feature = "debug-ui")]
                painter.circle_filled(control_point2, 10., Color32::YELLOW);

                painter.add(Shape::CubicBezier(CubicBezierShape::from_points_stroke(
                    [
                        Pos2::new(offset.x, offset.y + (NODE_RADIUS * scale)),
                        control_point1,
                        control_point2,
                        Pos2::new(
                            child_x + (NODE_RADIUS * scale),
                            child_y - ((NODE_RADIUS / 2.) * scale),
                        ),
                    ],
                    false,
                    Color32::TRANSPARENT,
                    stroke,
                )));
            }

            let child_children_shift = child.children_shift();
            child_x += ((child_children_shift + NODE_PADDING) * scale) + galley_c.size().x;
        }

        let mut child_x = offset.x - ((self.children_shift() / 2.) * scale);
        let node = self.clone();
        // draw nodes
        for child in self.children.iter_mut() {
            child.draw(
                ui,
                Pos2::new(child_x + (NODE_RADIUS * scale), child_y),
                scale,
                {
                    lineage.push(node.clone().into());
                    lineage.clone()
                },
            );
            let child_children_shift = child.children_shift();
            child_x += ((child_children_shift + NODE_PADDING) * scale) + galley_c.size().x;
        }

        let painter = ui.painter();
        painter.circle_filled(offset, NODE_RADIUS * scale, Color32::LIGHT_BLUE);
        let image = egui::include_image!("../assets/yoda.png");
        let image_rect = Rect::from_center_size(
            offset,
            (Vec2::splat(NODE_RADIUS * 2.) * scale) + Vec2::splat(1.0), // add one pixel to cover the whole background circle
        );

        #[cfg(feature = "debug-ui")]
        painter.rect_stroke(image_rect, Rounding::ZERO, Stroke::new(2.0, Color32::GREEN));

        egui::Image::new(image)
            .rounding(Rounding::same(NODE_RADIUS * 2.) * scale)
            .paint_at(ui, image_rect);

        #[cfg(feature = "debug-ui")]
        painter.circle_stroke(
            offset,
            NODE_RADIUS * scale,
            Stroke::new(1.0, Color32::GREEN),
        );

        if response.hovered() {
            let painter = ui.painter();
            painter.circle_stroke(offset, NODE_RADIUS * scale, stroke);
        }

        #[cfg(feature = "debug-ui")]
        painter.rect_stroke(
            Rect {
                min: Pos2::new(text_x, text_y),
                max: Pos2::new(text_x + galley_c.size().x, text_y + galley_c.size().y),
            },
            Rounding::ZERO,
            Stroke::new(1., Color32::GREEN),
        );
    }

    fn children_shift(&self) -> f32 {
        if self.children.is_empty() {
            return NODE_RADIUS * 2.;
        }

        ((NODE_RADIUS * 2.) * self.children.len() as f32)
            + (NODE_PADDING * self.children.len().saturating_sub(1) as f32)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SimpleNode {
    pub id: i32,
    name: String,
    gender: Gender,
    birthday: Option<DateTime<Utc>>,
    last_name: String,
}

impl From<Node> for SimpleNode {
    fn from(value: Node) -> Self {
        Self {
            id: value.id,
            name: value.name,
            gender: value.gender,
            birthday: value.birthday,
            last_name: value.last_name,
        }
    }
}

impl From<&Node> for SimpleNode {
    fn from(value: &Node) -> Self {
        Self {
            id: value.id,
            name: value.name.clone(),
            gender: value.gender,
            birthday: value.birthday,
            last_name: value.last_name.clone(),
        }
    }
}
