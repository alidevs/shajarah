use ar_reshaper::{config::LigaturesFlags, ArabicReshaper, ReshaperConfig};
use egui::{
    epaint::CubicBezierShape, text::LayoutJob, Color32, FontFamily, FontId, PointerButton, Pos2,
    Rect, Rounding, Sense, Shape, Stroke, TextFormat, Vec2,
};

use crate::{zoom::Zoom, Input};

const NODE_RADIUS: f32 = 30.;
const NODE_PADDING: f32 = 10. * 10.;
const SCROLL_VELOCITY: f32 = 0.005;
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
    prev_scale: f32,
    root: Node,
}

impl TreeUi {
    pub fn new(root: Node) -> Self {
        Self {
            offset: Vec2::ZERO,
            centered: false,
            scale: 1.,
            prev_scale: 1.,
            root,
        }
    }

    pub fn draw(&mut self, ui: &mut egui::Ui) {
        let bg_rect = ui.allocate_rect(ui.max_rect(), Sense::click_and_drag());
        let viewport = bg_rect.rect;
        ui.set_clip_rect(viewport);

        let input = ui.ctx().input(|i| Input {
            scroll_delta: i.raw_scroll_delta.y,
            hover_pos: i.pointer.hover_pos(),
        });

        ui.style_mut().zoom(self.scale);

        if bg_rect.dragged_by(PointerButton::Primary) {
            self.pan(bg_rect.drag_delta());
        }

        if !self.centered {
            self.offset = viewport.center().to_vec2();
            self.centered = true;
        }

        self.offset = match input.hover_pos {
            Some(hover_pos) if viewport.contains(hover_pos) => {
                if input.scroll_delta != 0.0 {
                    let new_scale = (self.scale * (1.0 + input.scroll_delta * SCROLL_VELOCITY))
                        .clamp(MIN_SCALE, MAX_SCALE);

                    self.scale(new_scale);
                    let scale_factor = self.scale / self.prev_scale;

                    let pos = self.offset - hover_pos.to_vec2();
                    (pos * scale_factor) + hover_pos.to_vec2()
                } else {
                    self.offset
                }
            }
            _ => self.offset,
        };

        self.root.draw(ui, self.offset.to_pos2(), self.scale);
    }

    fn scale(&mut self, new_scale: f32) {
        self.prev_scale = self.scale;
        self.scale = new_scale;
    }

    fn pan(&mut self, delta: Vec2) {
        self.offset += delta;
    }

    // fn screen_pos_to_graph(&self, pos: Pos2, viewport: Rect) -> Pos2 {
    //     (pos + self.offset - viewport.center().to_vec2()) / self.scale
    // }
}

pub struct Node {
    // id: usize,
    children: Vec<Node>,
}

impl Node {
    pub fn new(_id: usize, children: Vec<Node>) -> Self {
        Self {
            // id,
            children,
        }
    }

    // pub fn add_child(&mut self, child: Node) {
    //     self.children.push(child);
    // }

    pub fn draw(&self, ui: &mut egui::Ui, offset: Pos2, scale: f32) {
        let painter = ui.painter();

        let mut job = LayoutJob::default();
        job.append(
            &RESHAPER.reshape("سلمان").chars().rev().collect::<String>(),
            0.0,
            TextFormat {
                font_id: FontId::new(14.0 * scale, FontFamily::Monospace),
                color: ui.visuals().text_color(),
                ..Default::default()
            },
        );
        let galley = painter.layout_job(job);
        painter.galley(
            Pos2::new(
                offset.x - (galley.size().x / 2.),
                offset.y - ((NODE_RADIUS * 2.) * scale),
            ),
            galley,
            Color32::WHITE,
        );

        if self.children.is_empty() {
            painter.circle_filled(offset, NODE_RADIUS * scale, Color32::LIGHT_BLUE);

            let image_rect = Rect::from_center_size(
                offset,
                (Vec2::splat(NODE_RADIUS * 2.) * scale) + Vec2::splat(1.0), // add one pixel to cover the whole background circle
            );

            #[cfg(feature = "debug-ui")]
            painter.rect_stroke(image_rect, Rounding::ZERO, Stroke::new(2.0, Color32::GREEN));

            egui::Image::from_uri("https://r2.bksalman.com/ppL.webp")
                .rounding(Rounding::same(NODE_RADIUS * 2.) * scale)
                .paint_at(ui, image_rect);
            return;
        }

        let stroke = ui.visuals().widgets.noninteractive.fg_stroke;
        let stroke = Stroke::new(stroke.width * scale, stroke.color);

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
            child_x += (child_children_shift + NODE_PADDING) * scale;
        }

        let mut child_x = offset.x - ((self.children_shift() / 2.) * scale);
        // draw nodes
        for child in self.children.iter() {
            child.draw(
                ui,
                Pos2::new(child_x + (NODE_RADIUS * scale), child_y),
                scale,
            );
            let child_children_shift = child.children_shift();
            child_x += (child_children_shift + NODE_PADDING) * scale;
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
    }

    fn children_shift(&self) -> f32 {
        if self.children.is_empty() {
            return NODE_RADIUS * 2.;
        }

        ((NODE_RADIUS * 2.) * self.children.len() as f32)
            + (NODE_PADDING * self.children.len().saturating_sub(1) as f32)
    }
}
