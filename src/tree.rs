use std::collections::HashMap;

use ar_reshaper::{config::LigaturesFlags, ArabicReshaper, ReshaperConfig};
use chrono::{DateTime, Utc};

#[cfg(feature = "debug-ui")]
use egui::Stroke;

use egui::{
    epaint::CubicBezierShape, include_image, text::LayoutJob, Align, Color32, CornerRadius,
    FontFamily, FontId, PointerButton, Pos2, Rect, Sense, Shape, TextFormat, Vec2, Vec2b, Widget,
};

#[cfg(feature = "debug-ui")]
use egui::StrokeKind;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{zoom::Zoom, Gender};

const DEFAULT_IMAGE: egui::ImageSource<'static> = include_image!("../assets/avatar.png");
const NODE_RADIUS: u8 = 30;
const NODE_PADDING: f32 = 30.;
const NODE_TEXT_PADDING: f32 = 10.;
const MAX_SCALE: f32 = 5.0;
const MIN_SCALE: f32 = 0.2;
const RESHAPER: ArabicReshaper = ArabicReshaper::new(ReshaperConfig::new(
    ar_reshaper::Language::Arabic,
    LigaturesFlags::default(),
));

/// this holds all the nodes, and acts as an arena allocator.
/// this is done to be able to mutate nodes while iterating them
/// in different orders
// the layout code is based on the Reingold Tilford algorithm explained in this blog post:
// https://rachel53461.wordpress.com/2014/04/20/algorithm-for-drawing-trees/
// and shamelessly stolen from https://gitlab.com/seamsay/reingold-tilford
pub struct LayoutTree(Vec<LayoutNode>);

impl LayoutTree {
    pub fn update_tree(&mut self, root: Node) {
        let mut tree = Vec::new();
        tree.push(LayoutNode {
            id: root.id,
            order: 0,
            depth: 0,

            gender: root.gender,
            father_idx: None,
            mother_idx: None,
            children: Vec::new(),

            x: 0.,
            y: 0.,
            mod_: 0.,
            collapsed: root.collapsed,
        });

        let mut queue = std::collections::VecDeque::new();
        queue.push_back((0, root));

        while let Some((node_idx, node)) = queue.pop_front() {
            let index = tree.len();

            for (i, child) in node.children.into_iter().enumerate() {
                let index = index + i;
                let depth = tree[node_idx].depth + 1;

                tree[node_idx].children.push(index);

                let mother_idx = match node.gender {
                    Gender::Male => None,
                    Gender::Female => Some(node_idx),
                };

                let father_idx = match node.gender {
                    Gender::Male => Some(node_idx),
                    Gender::Female => None,
                };

                tree.push(LayoutNode {
                    id: child.id,
                    mother_idx,
                    father_idx,
                    gender: child.gender,

                    order: i,
                    depth,

                    children: Vec::new(),
                    x: 0.,
                    y: 0.,
                    mod_: 0.,
                    collapsed: child.collapsed,
                });

                queue.push_back((index, child));
            }
        }

        self.0 = tree;
    }

    pub fn set_root(&mut self, root: Option<Node>) {
        if let Some(root) = root {
            self.update_tree(root)
        } else {
            self.0 = vec![];
        }
    }

    pub fn reset_positions(&mut self) {
        for node in &mut self.0 {
            node.x = 0.;
            node.y = 0.;
            node.mod_ = 0.;
        }
    }

    pub fn root(&self) -> Option<usize> {
        if self.0.is_empty() {
            None
        } else {
            Some(0)
        }
    }

    fn post_order(&self, node: usize) -> Vec<usize> {
        let mut breadth_first = vec![node];
        let mut post_order = Vec::new();

        while let Some(node) = breadth_first.pop() {
            if !self[node].collapsed {
                breadth_first.extend_from_slice(&self[node].children);
            }
            post_order.push(node);
        }

        post_order.reverse();
        post_order
    }

    fn previous_sibling(&self, node: usize) -> Option<usize> {
        let order = self[node].order;
        if order == 0 {
            return None;
        }

        let father = self[node]
            .father_idx
            .expect("Nodes where `order != 0` always have parents.");

        Some(self[father].children[order - 1])
    }

    fn left_siblings(&self, node: usize) -> Vec<usize> {
        let order = self[node].order;

        if let Some(parent) = self[node].father_idx {
            self[parent].children[0..order].into()
        } else {
            Vec::new()
        }
    }

    fn siblings_between(&self, left: usize, right: usize) -> Vec<usize> {
        let left_order = self[left].order;
        let right_order = self[right].order;

        if self[left].is_root() || self[right].is_root() {
            assert!(
                self[left].is_root(),
                "If one node is the root then both nodes must be."
            );
            assert!(
                self[right].is_root(),
                "If one node is the root then both nodes must be."
            );

            return Vec::new();
        }

        let left_parent = self[left]
            .father_idx
            .expect("`is_none` has already been checked.");

        let right_parent = self[right]
            .father_idx
            .expect("`is_none` has already been checked.");

        assert!(
            left_parent == right_parent,
            "Nodes must actually be siblings."
        );

        let parent = left_parent;

        self[parent].children[left_order + 1..right_order].into()
    }

    fn breadth_first(&self, node: usize) -> Vec<usize> {
        let mut breadth_first = vec![node];
        let mut index = 0;

        while index < breadth_first.len() {
            let node = breadth_first[index];
            if !self[node].collapsed {
                breadth_first.extend_from_slice(&self[node].children);
            }
            index += 1;
        }

        breadth_first
    }

    fn initialize_y(&mut self, root: usize) {
        let mut next_row = vec![root];

        while !next_row.is_empty() {
            let row = next_row;
            next_row = Vec::new();

            let mut max = -f32::INFINITY;
            for node in &row {
                let node = *node;

                // TODO: handle mother_idx as well
                self[node].y = if let Some(parent) = self[node].father_idx {
                    self[parent].y + NODE_RADIUS as f32 * 2. + NODE_PADDING * 2.
                } else {
                    0.0
                }; //  + self[node].data.top_space();

                if self[node].y > max {
                    max = self[node].y;
                }

                if !self[node].collapsed {
                    next_row.extend_from_slice(&self[node].children);
                }
            }

            for node in &row {
                self[*node].y = max;
            }
        }
    }

    fn center_nodes_between(&mut self, left: usize, right: usize) {
        let num_gaps = self[right].order - self[left].order;

        let space_per_gap = ((self[right].x - NODE_RADIUS as f32)
            - (self[left].x + NODE_RADIUS as f32))
            / (num_gaps as f32);

        for (i, sibling) in self.siblings_between(left, right).into_iter().enumerate() {
            let i = i + 1;

            let old_x = self[sibling].x;
            // HINT: We traverse the tree in post-order so we should never be moving anything to the
            //       left.
            // TODO: Have some kind of `move_node` method that checks things like this?
            let new_x = f32::max(
                old_x,
                (self[left].x + NODE_RADIUS as f32) + space_per_gap * (i as f32),
            );
            let diff = new_x - old_x;

            self[sibling].x = new_x;
            self[sibling].mod_ += diff;
        }
    }

    fn fix_overlaps(&mut self, right: usize) {
        fn max_depth(l: &HashMap<usize, f32>, r: &HashMap<usize, f32>) -> usize {
            if let Some(l) = l.keys().max() {
                if let Some(r) = r.keys().max() {
                    return std::cmp::min(*l, *r);
                }
            }

            0
        }

        let right_node_contour = left_contour(self, right);

        for left in self.left_siblings(right) {
            let left_node_contour = right_contour(self, left);
            let mut shift = 0.0;

            log::debug!(
                "left contour: {right_node_contour:#?}, right contour: {left_node_contour:#?}"
            );

            for depth in self[right].depth..=max_depth(&right_node_contour, &left_node_contour) {
                let gap = right_node_contour[&depth] - left_node_contour[&depth];
                if gap + shift < 0.0 {
                    shift = -gap;
                }
            }

            log::debug!("left: {left}, right: {right}, shift: {shift}");

            self[right].x += shift;
            self[right].mod_ += shift;

            self.center_nodes_between(left, right);
        }
    }

    fn initialize_x(&mut self, root: usize) {
        for node in self.post_order(root) {
            if self[node].is_leaf() {
                self[node].x = if let Some(sibling) = self.previous_sibling(node) {
                    self[sibling].x + NODE_RADIUS as f32 * 2. + NODE_PADDING * 2.
                } else {
                    0.0
                };
            } else {
                let mid = {
                    let first = self[*self[node]
                        .children
                        .first()
                        .expect("Only leaf nodes have no children.")]
                    .x;
                    let last = self[*self[node]
                        .children
                        .last()
                        .expect("Only leaf nodes have no children.")]
                    .x;

                    (first + last) / 2.0
                };

                if let Some(sibling) = self.previous_sibling(node) {
                    self[node].x = self[sibling].x + NODE_RADIUS as f32 * 2. + NODE_PADDING * 2.;
                    self[node].mod_ = self[node].x - mid;
                } else {
                    self[node].x = mid;
                }

                self.fix_overlaps(node);
            }
        }
    }

    fn ensure_positive_x(&mut self, root: usize) {
        let contour = left_contour(self, root);
        let shift = -contour
            .values()
            .fold(None, |acc, curr| {
                let acc = acc.unwrap_or(f32::INFINITY);
                let curr = *curr;
                // log::debug!("curr: {curr}, acc: {acc}");
                Some(if curr < acc { curr } else { acc })
            })
            .unwrap_or(0.0);

        // log::debug!("shift: {shift}");

        self[root].x += shift;
        self[root].mod_ += shift;
    }

    fn finalize_x(&mut self, root: usize) {
        for node in self.breadth_first(root) {
            let shift = if let Some(parent) = self[node].father_idx {
                self[parent].mod_
            } else {
                0.0
            };

            self[node].x += shift;
            self[node].mod_ += shift;
        }
    }

    pub fn layout(&mut self) {
        if let Some(root) = self.root() {
            self.reset_positions();
            // log::debug!("laying out the tree");

            self.initialize_y(root);
            self.initialize_x(root);

            self.ensure_positive_x(root);
            self.finalize_x(root);
        }
    }

    pub fn get(&self, id: i32) -> Option<&LayoutNode> {
        self.0.iter().find(|n| n.id == id)
    }

    pub fn get_mut(&mut self, id: i32) -> Option<&mut LayoutNode> {
        self.0.iter_mut().find(|n| n.id == id)
    }
}

impl std::ops::Index<usize> for LayoutTree {
    type Output = LayoutNode;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for LayoutTree {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

fn left_contour(tree: &LayoutTree, node: usize) -> HashMap<usize, f32> {
    contour(tree, node, f32::min, |n| n.x)
}

fn right_contour(tree: &LayoutTree, node: usize) -> HashMap<usize, f32> {
    contour(tree, node, f32::max, |n| n.x)
}

fn contour<C, E>(tree: &LayoutTree, node: usize, cmp: C, edge: E) -> HashMap<usize, f32>
where
    C: Fn(f32, f32) -> f32,
    E: Fn(&LayoutNode) -> f32,
{
    let mut stack = vec![(0.0, node)];
    let mut contour = HashMap::new();

    while let Some((mod_, node)) = stack.pop() {
        let depth = tree[node].depth;
        let shifted = edge(&tree[node]) + mod_;
        let new = if let Some(current) = contour.get(&depth) {
            cmp(*current, shifted)
        } else {
            shifted
        };
        let mod_ = mod_ + tree[node].mod_;

        contour.insert(depth, new);

        if !tree[node].collapsed {
            stack.extend(tree[node].children.iter().map(|c| (mod_, *c)));
        }
    }

    contour
}

pub struct TreeUi {
    pub offset: Vec2,
    centered: bool,
    scale: f32,
    pub root: Option<Node>,
    pub layout_tree: LayoutTree,
}

impl TreeUi {
    pub fn new(root: Option<Node>) -> Self {
        let mut tree = LayoutTree(vec![]);

        tree.set_root(root.clone());

        Self {
            offset: Vec2::ZERO,
            centered: false,
            scale: 1.,
            layout_tree: tree,
            root,
        }
    }

    pub fn set_root(&mut self, root: Option<Node>) {
        self.root = root;
        self.layout_tree.set_root(self.root.clone());
    }

    pub fn layout(&mut self) {
        self.layout_tree.layout();
    }

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

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct LayoutNode {
    id: i32,
    gender: Gender,

    pub children: Vec<usize>,

    mother_idx: Option<usize>,
    father_idx: Option<usize>,

    /// The position of this node among it's siblings.
    ///
    /// Can also be thought of as the number of left-siblings this node has.
    pub order: usize,
    /// The depth of this node.
    ///
    /// Can be thought of as the number of edges between this node and the root node.
    pub depth: usize,

    pub x: f32,
    pub y: f32,

    /// How much to shift this node's children by
    mod_: f32,

    collapsed: bool,
}

impl LayoutNode {
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty() | self.collapsed
    }

    pub fn is_root(&self) -> bool {
        self.mother_idx.is_none() && self.father_idx.is_none()
    }
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: i32,
    name: String,
    gender: Gender,
    birthday: Option<DateTime<Utc>>,
    last_name: String,
    father_id: Option<i32>,
    mother_id: Option<i32>,
    pub personal_info: Option<IndexMap<String, String>>,
    pub children: Vec<Node>,
    image: Option<Vec<u8>>,

    /// used for displaying or hiding the member info window
    #[serde(skip)]
    window_is_open: bool,

    #[serde(default = "yes")]
    collapsed: bool,
}

impl Node {
    // pub fn add_child(&mut self, child: Node) {
    //     self.children.push(child);
    // }

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

        let text_style = FontId::new(20.0 * scale, FontFamily::Monospace);

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
                        stroke,
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
                        stroke,
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
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RESHAPER
                                            .reshape(format!("{key}: "))
                                            .chars()
                                            .rev()
                                            .collect::<String>(),
                                    );
                                    ui.label(
                                        RESHAPER.reshape(value).chars().rev().collect::<String>(),
                                    );
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
