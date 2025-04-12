use ar_reshaper::letters::letters_db::LETTERS_ARABIC;
use ar_reshaper::{config::LigaturesFlags, ArabicReshaper, ReshaperConfig};
use egui::epaint::PathStroke;
use egui::Stroke;
use egui::{
    epaint::CubicBezierShape, text::LayoutJob, Align, Color32, CornerRadius, FontFamily, FontId,
    PointerButton, Pos2, Rect, Sense, Shape, TextFormat, Vec2, Vec2b, Widget,
};

use crate::zoom::Zoom;

#[cfg(feature = "debug-ui")]
use egui::Stroke;

#[cfg(feature = "debug-ui")]
use egui::StrokeKind;

use super::{layout::LayoutTree, Node, SimpleNode, TreeUi, DEFAULT_IMAGE, NODE_RADIUS};

const MAX_SCALE: f32 = 5.0;
const MIN_SCALE: f32 = 0.2;
const RESHAPER: ArabicReshaper = ArabicReshaper::new(ReshaperConfig::new(
    ar_reshaper::Language::Arabic,
    LigaturesFlags::default(),
));

const EXPAND_INDICATOR_SIZE: f32 = 20.;

impl TreeUi {
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        ui.style_mut().zoom(self.scale);

        let bg_rect = ui.allocate_rect(ui.max_rect(), Sense::click_and_drag());
        let viewport = bg_rect.rect;
        ui.set_clip_rect(viewport);

        if bg_rect.dragged_by(PointerButton::Primary) {
            self.pan(bg_rect.drag_delta());

            #[cfg(feature = "debug-ui")]
            log::debug!("new offset: {:?}", self.offset);
        }

        let background_clicked = bg_rect.clicked_by(PointerButton::Primary);

        if let Some(hover_pos) = ui.ctx().input(|i| i.pointer.hover_pos()) {
            if bg_rect.hovered() {
                let zoom_delta = ui.ctx().input(|i| i.zoom_delta());

                // there is change
                if zoom_delta != 1. {
                    let prev_scale = self.scale;
                    let new_scale = (prev_scale * zoom_delta).clamp(MIN_SCALE, MAX_SCALE);

                    self.scale(new_scale);
                    let scale_factor = self.scale / prev_scale;
                    let pos = self.offset - hover_pos.to_vec2();

                    self.offset = (pos * scale_factor) + hover_pos.to_vec2();

                    #[cfg(feature = "debug-ui")]
                    log::debug!("new offset: {:?}", self.offset);
                }
            }
        }

        if let Some(root) = &mut self.root {
            if !self.centered {
                if let Some(layout_root) = self.layout_tree.root() {
                    let root_coords = &self.layout_tree[layout_root];
                    let center = viewport.center().to_vec2();
                    // log::debug!("{center}");

                    #[cfg(feature = "debug-ui")]
                    log::debug!("root_coords: {root_coords:?}");

                    self.offset = Vec2::new(-root_coords.x + center.x, center.y);

                    #[cfg(feature = "debug-ui")]
                    log::debug!("offset: {:?}", self.offset);
                }

                self.centered = true;
            }

            root.draw(
                ui,
                &mut self.offset,
                self.scale,
                &mut self.layout_tree,
                vec![],
                background_clicked,
            );
        }
    }
}

impl Node {
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        offset: &mut Vec2,
        scale: f32,
        layout_tree: &mut LayoutTree,
        mut lineage: Vec<SimpleNode>,
        background_clicked: bool,
    ) {
        let stroke = ui.visuals().widgets.noninteractive.fg_stroke;
        let layout_node = layout_tree
            .get(self.id)
            .expect("probably didn't update the layout tree");
        let coords = Pos2::new(
            offset.x + layout_node.x * scale,
            offset.y + layout_node.y * scale,
        );

        let text_style = FontId::new(24.0 * scale, FontFamily::Monospace);

        let painter = ui.painter();

        let mut job = LayoutJob::default();
        job.append(
            &RESHAPER
                .reshape(self.name.clone())
                .chars()
                .rev()
                .collect::<String>(),
            0.0,
            TextFormat {
                font_id: text_style.clone(),
                color: ui.visuals().text_color(),
                ..Default::default()
            },
        );
        let galley = painter.layout_job(job);

        #[cfg(feature = "debug-ui")]
        let galley_c = galley.clone();

        let text_coords = Pos2::new(
            coords.x - galley.size().x / 2.,
            coords.y + NODE_RADIUS as f32 * scale,
        );
        let text_size = galley.size();

        painter.galley(text_coords, galley, Color32::WHITE);

        // #[cfg(feature = "debug-ui")]
        // {
        //     log::debug!("coords: {coords:?}");
        // }

        if !self.collapsed {
            for child in self.children.iter() {
                let child_coords = layout_tree
                    .get(child.id)
                    .expect("probably didn't update the layout tree");
                let child_coords = Pos2::new(
                    offset.x + child_coords.x * scale,
                    offset.y + child_coords.y * scale,
                );

                if child_coords.x == coords.x {
                    painter.line_segment(
                        [
                            child_coords,
                            text_coords + Vec2::new(text_size.x / 2., text_size.y),
                        ],
                        Stroke::new(stroke.width * 2., stroke.color),
                    );
                } else {
                    let control_point1 =
                        Pos2::new(text_coords.x + text_size.x / 2., child_coords.y);

                    #[cfg(feature = "debug-ui")]
                    painter.circle_filled(control_point1, 10., Color32::WHITE);

                    let control_point2 = Pos2::new(child_coords.x, text_coords.y + text_size.y);

                    #[cfg(feature = "debug-ui")]
                    painter.circle_filled(control_point2, 10., Color32::YELLOW);

                    painter.add(Shape::CubicBezier(CubicBezierShape::from_points_stroke(
                        [
                            Pos2::new(
                                text_coords.x + text_size.x / 2.,
                                text_coords.y + text_size.y,
                            ),
                            control_point1,
                            control_point2,
                            Pos2::new(child_coords.x, child_coords.y),
                        ],
                        false,
                        Color32::TRANSPARENT,
                        Stroke::new(stroke.width * 2., stroke.color),
                    )));
                }
            }
        }

        let window_pos =
            coords + Vec2::new(NODE_RADIUS as f32 * 1.2, -(NODE_RADIUS as f32 / 2.)) * scale;

        if background_clicked && self.window_is_open {
            self.window_is_open = false;
        }

        egui::Window::new(self.id.to_string())
            .id(egui::Id::new(self.id))
            .max_width(180.)
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
                    let image = self
                        .image
                        .as_ref()
                        .map(|i| egui::ImageSource::Bytes {
                            uri: format!("{}-{}", self.id, self.name).into(),
                            bytes: egui::load::Bytes::from(i.clone()),
                        })
                        .unwrap_or(DEFAULT_IMAGE);

                    egui::Image::new(image)
                        .maintain_aspect_ratio(true)
                        .show_loading_spinner(true)
                        .ui(ui);

                    #[cfg(feature = "debug-ui")]
                    ui.label(format!("{{ x: {}, y: {} }}", layout_node.x, layout_node.y));

                    ui.label(self.id.to_string());

                    let lineage = lineage
                        .iter()
                        .rev()
                        .map(|l| format!("{} ", l.name.clone()))
                        .take(2)
                        .collect::<String>();
                    ui.heading(
                        RESHAPER
                            .reshape(format!("{} {}{}", self.name, lineage, self.last_name))
                            .chars()
                            .rev()
                            .collect::<String>(),
                    );

                    if let Some(personal_info) = self.personal_info.as_ref() {
                        if !personal_info.is_empty() {
                            ui.add_space(10.);
                            ui.label(
                                RESHAPER
                                    .reshape("المعلومات الشخصية:")
                                    .chars()
                                    .rev()
                                    .collect::<String>(),
                            );

                            for (key, value) in personal_info {
                                let key = fix_arabic(&format!("{key}: "));
                                let value = fix_arabic(&value);
                                ui.horizontal(|ui| {
                                    ui.label(key);
                                    ui.label(value);
                                });
                            }
                        }
                    }
                });
            });

        lineage.push(self.clone().into());

        if !self.collapsed {
            for child in self.children.iter_mut() {
                child.draw(
                    ui,
                    offset,
                    scale,
                    layout_tree,
                    lineage.clone(),
                    background_clicked,
                );
            }
        }

        let image_rect = Rect::from_center_size(
            coords,
            (Vec2::splat(NODE_RADIUS as f32 * 2.) * scale) + Vec2::splat(1.0), // add one pixel to cover the whole background circle
        );

        let response = ui.allocate_rect(image_rect, Sense::click());

        if response.double_clicked() {
            self.collapsed = !self.collapsed;
            layout_tree
                .get_mut(self.id)
                .expect("node should exist in layout tree")
                .collapsed = self.collapsed;

            let prev_x = layout_tree
                .get(self.id)
                .expect("node should exist in layout tree")
                .x;

            layout_tree.layout();

            let new_x = layout_tree
                .get(self.id)
                .expect("node should exist in layout tree")
                .x;

            offset.x -= (new_x - prev_x) * scale;
        }

        if response.clicked() {
            self.window_is_open = !self.window_is_open;
        }

        let painter = ui.painter();
        painter.circle_filled(coords, NODE_RADIUS as f32 * scale, Color32::LIGHT_BLUE);

        if !self.children.is_empty() {
            if self.collapsed {
                let mut shape = Shape::convex_polygon(
                    vec![
                        coords + Vec2::new(-(NODE_RADIUS as f32), NODE_RADIUS as f32) * scale,
                        coords
                            + Vec2::new(-(NODE_RADIUS as f32), NODE_RADIUS as f32) * scale
                            + Vec2::new(-EXPAND_INDICATOR_SIZE / 2., -EXPAND_INDICATOR_SIZE / 2.)
                                * scale,
                        coords
                            + Vec2::new(-(NODE_RADIUS as f32), NODE_RADIUS as f32) * scale
                            + Vec2::new(EXPAND_INDICATOR_SIZE / 2., -EXPAND_INDICATOR_SIZE / 2.)
                                * scale,
                    ],
                    stroke.color,
                    PathStroke::NONE,
                );

                shape.translate(Vec2::new(-1., 5.));

                painter.add(shape);
            } else {
                let mut shape = Shape::convex_polygon(
                    vec![
                        coords
                            + Vec2::new(-(NODE_RADIUS as f32), NODE_RADIUS as f32) * scale
                            + Vec2::new(0., -EXPAND_INDICATOR_SIZE / 1.8) * scale,
                        coords
                            + Vec2::new(-(NODE_RADIUS as f32), NODE_RADIUS as f32) * scale
                            + Vec2::new(-EXPAND_INDICATOR_SIZE / 1.8, 0.) * scale,
                        coords
                            + Vec2::new(-(NODE_RADIUS as f32), NODE_RADIUS as f32) * scale
                            + Vec2::new(EXPAND_INDICATOR_SIZE / 1.8, 0.) * scale,
                    ],
                    stroke.color,
                    PathStroke::NONE,
                );

                shape.translate(Vec2::new(-1., 5.));

                painter.add(shape);
            }
        }

        #[cfg(feature = "debug-ui")]
        painter.rect_stroke(
            image_rect,
            CornerRadius::ZERO,
            Stroke::new(2.0, Color32::GREEN),
            StrokeKind::Middle,
        );

        let image = self
            .image
            .as_ref()
            .map(|i| egui::ImageSource::Bytes {
                uri: format!("{}-{}", self.id, self.name).into(),
                bytes: egui::load::Bytes::from(i.clone()),
            })
            .unwrap_or(DEFAULT_IMAGE);

        egui::Image::new(image)
            .corner_radius(CornerRadius::same(NODE_RADIUS) * scale)
            .maintain_aspect_ratio(true)
            .show_loading_spinner(true)
            .paint_at(ui, image_rect);

        if response.hovered() {
            let painter = ui.painter();
            painter.circle_stroke(coords, NODE_RADIUS as f32 * scale, stroke);
        }

        #[cfg(feature = "debug-ui")]
        painter.rect_stroke(
            Rect {
                min: Pos2::new(text_coords.x, text_coords.y),
                max: Pos2::new(
                    text_coords.x + galley_c.size().x,
                    text_coords.y + galley_c.size().y,
                ),
            },
            CornerRadius::ZERO,
            Stroke::new(1., Color32::GREEN),
            StrokeKind::Middle,
        );
    }
}

/// workaround to fix arabic words until egui has better RTL rendering
fn fix_arabic(input: &str) -> String {
    let input = RESHAPER.reshape(input);
    let mut output = String::with_capacity(input.len());
    let mut ar_buffer = String::new();
    let mut in_arabic_block = false;

    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        let is_arabic_char = LETTERS_ARABIC.iter().any(|l| {
            l.0 == c || l.1.isolated == c || l.1.initial == c || l.1.medial == c || l.1.end == c
        });

        if is_arabic_char {
            ar_buffer.push(c);
            in_arabic_block = true;
        } else if in_arabic_block && (c.is_whitespace() || is_symbol(c)) {
            ar_buffer.push(c);
        } else {
            if in_arabic_block {
                output.extend(ar_buffer.chars().rev());
                ar_buffer.clear();
                in_arabic_block = false;
            }
            output.push(c);
        }
    }

    if in_arabic_block {
        output.extend(ar_buffer.chars().rev());
    }

    output
}

/// a non-exhaustive check of symbols
fn is_symbol(c: char) -> bool {
    matches!(
        c,
        ':' | '?'
            | '؟'
            | '،'
            | '.'
            | ','
            | ';'
            | '!'
            | 'ـ'
            | '"'
            | '\''
            | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rev_arabic_only_arabic() {
        let input = String::from("السلام عليكم جميعا");

        let shaped = RESHAPER.reshape(&input);

        let output = fix_arabic(&shaped);

        let expected = String::from("ﺎﻌﻴﻤﺟ ﻢﻜﻴﻠﻋ ﻡﺎﻠﺴﻟﺍ");

        assert_eq!(output, expected);
    }
}
