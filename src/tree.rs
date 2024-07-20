use egui::{
    epaint::{CubicBezierShape, PathStroke},
    text::LayoutJob,
    Align2, Color32, FontFamily, FontId, Pos2, Shape, TextFormat,
};

const NODE_RADIUS: f32 = 30.;
const NODE_PADDING: f32 = 10. * 10.;

pub struct Node {
    id: usize,
    children: Vec<Node>,
}

impl Node {
    pub fn new(id: usize, children: Vec<Node>) -> Self {
        Self { id, children }
    }

    pub fn travers_tree<'a>(&'a self) -> Vec<NodeRef<'a>> {
        let mut ret = vec![NodeRef(&self)];

        for child in self.children.iter() {
            ret.extend(child.travers_tree());
        }

        return ret;
    }

    pub fn add_child(&mut self, child: Node) {
        self.children.push(child);
    }

    pub fn draw(&self, ui: &mut egui::Ui, x_offset: f32, y_offset: f32) {
        let painter = ui.painter();
        let parent_pos = Pos2::new(x_offset, y_offset);

        let mut job = LayoutJob::default();
        job.append(
            &format!("{}", self.id),
            0.0,
            TextFormat {
                font_id: FontId::new(14.0, FontFamily::Monospace),
                color: Color32::WHITE,
                ..Default::default()
            },
        );
        let galley = painter.layout_job(job);
        painter.galley(
            Pos2::new(
                parent_pos.x - (galley.size().x / 2.),
                y_offset - (NODE_RADIUS * 2.),
            ),
            galley,
            Color32::WHITE,
        );

        if self.children.len() == 0 {
            painter.circle_filled(parent_pos, NODE_RADIUS, Color32::RED);
            return;
        }

        let mut child_x = parent_pos.x - (self.children_shift() / 2.);
        let child_y = parent_pos.y + NODE_RADIUS * 2. + NODE_PADDING;
        // draw lines
        for child in self.children.iter() {
            let painter = ui.painter();

            if child_x + NODE_RADIUS == parent_pos.x {
                painter.line_segment(
                    [Pos2::new(child_x + NODE_RADIUS, child_y), parent_pos],
                    PathStroke::new(3., Color32::GREEN),
                );
            } else {
                let control_point1 = Pos2::new(parent_pos.x, child_y - NODE_PADDING);
                // painter.circle_filled(control_point1, 10., Color32::WHITE);
                let control_point2 = Pos2::new(child_x + NODE_RADIUS, parent_pos.y + NODE_PADDING);
                // painter.circle_filled(control_point2, 10., Color32::YELLOW);
                painter.add(Shape::CubicBezier(CubicBezierShape::from_points_stroke(
                    [
                        Pos2::new(parent_pos.x, y_offset + NODE_RADIUS),
                        control_point1,
                        control_point2,
                        Pos2::new(child_x + NODE_RADIUS, child_y - NODE_RADIUS / 2.),
                    ],
                    false,
                    Color32::TRANSPARENT,
                    PathStroke::new(3., Color32::GREEN),
                )));
            }

            let child_children_shift = child.children_shift();
            child_x += child_children_shift + NODE_PADDING;
        }

        let mut child_x = x_offset - (self.children_shift() / 2.);
        // draw nodes
        for child in self.children.iter() {
            child.draw(ui, child_x + NODE_RADIUS, child_y);
            let child_children_shift = child.children_shift();
            child_x += child_children_shift + NODE_PADDING;
        }

        let painter = ui.painter();
        painter.circle_filled(parent_pos, NODE_RADIUS, Color32::RED);
    }

    fn children_shift(&self) -> f32 {
        if self.children.len() == 0 {
            return NODE_RADIUS * 2.;
        }

        ((NODE_RADIUS * 2.) * self.children.len() as f32)
            + (NODE_PADDING * self.children.len().saturating_sub(1) as f32)
    }
}

pub struct NodeRef<'a>(&'a Node);

impl<'a> NodeRef<'a> {
    pub fn id(&self) -> usize {
        self.0.id
    }
    pub fn children(&self) -> Vec<NodeRef<'a>> {
        self.0.children.iter().map(|c| NodeRef(c)).collect()
    }

    pub fn draw(&self, ui: &mut egui::Ui, x_offset: f32, y_offset: f32) {
        self.0.draw(ui, x_offset, y_offset);
    }
}
