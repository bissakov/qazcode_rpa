use egui::{Color32, Painter, Pos2, Stroke, Vec2};
use rpa_core::{BranchType, NanoId, Node, UiConstants, constants::FlowDirection};
use std::collections::HashMap;

pub struct PinPosition {
    world_pos: Pos2,
}

impl PinPosition {
    pub fn output(node: &Node, branch_type: &BranchType) -> Self {
        let world_pos = match branch_type {
            BranchType::TrueBranch => node.get_output_pin_pos_by_index(0),
            BranchType::FalseBranch => node.get_output_pin_pos_by_index(1),
            BranchType::LoopBody => node.get_output_pin_pos_by_index(1),
            BranchType::ErrorBranch => node.get_output_pin_pos_by_index(1),
            BranchType::TryBranch => node.get_output_pin_pos_by_index(0),
            BranchType::CatchBranch => node.get_output_pin_pos_by_index(1),
            BranchType::Default => {
                if node.get_output_pin_count() > 1 {
                    node.get_output_pin_pos_by_index(0)
                } else {
                    node.get_output_pin_pos()
                }
            }
        };
        Self { world_pos }
    }

    pub fn input(node: &Node) -> Self {
        let world_pos = node.get_input_pin_pos();
        Self { world_pos }
    }

    pub fn world_pos(&self) -> Pos2 {
        self.world_pos
    }

    pub fn screen_pos<F>(&self, transform: F) -> Pos2
    where
        F: Fn(Pos2) -> Pos2,
    {
        transform(self.world_pos)
    }
}

pub struct ConnectionPath {
    start: Pos2,
    end: Pos2,
    control1: Pos2,
    control2: Pos2,
}

impl ConnectionPath {
    pub fn new<F>(from_node: &Node, to_node: &Node, branch_type: &BranchType, transform: F) -> Self
    where
        F: Fn(Pos2) -> Pos2,
    {
        let start = PinPosition::output(from_node, branch_type).screen_pos(&transform);
        let end = PinPosition::input(to_node).screen_pos(&transform);

        let is_error_branch = match *branch_type {
            BranchType::ErrorBranch
            | BranchType::LoopBody
            | BranchType::FalseBranch
            | BranchType::CatchBranch => true,
            _ => false,
        };
        let (offset1, offset2) = calculate_bezier_control_points(start, end, is_error_branch);

        let control1 = start + offset1;
        let control2 = end - offset2;

        Self {
            start,
            end,
            control1,
            control2,
        }
    }

    pub fn get_bezier_points(
        &self,
        renderer: &mut ConnectionRenderer,
        connection_id: &NanoId,
    ) -> Vec<Pos2> {
        renderer.get_or_compute_bezier(
            connection_id,
            self.start,
            self.control1,
            self.control2,
            self.end,
        )
    }

    pub fn hit_test(
        &self,
        point: Pos2,
        renderer: &mut ConnectionRenderer,
        connection_id: &NanoId,
        threshold: f32,
    ) -> bool {
        let bezier_points = self.get_bezier_points(renderer, connection_id);
        let dist = point_to_bezier_distance(point, &bezier_points);
        dist < threshold
    }

    pub fn intersects_line(
        &self,
        p1: Pos2,
        p2: Pos2,
        renderer: &mut ConnectionRenderer,
        connection_id: &NanoId,
    ) -> bool {
        let bezier_points = self.get_bezier_points(renderer, connection_id);

        for i in 0..bezier_points.len() - 1 {
            let bezier_start = bezier_points[i];
            let bezier_end = bezier_points[i + 1];

            if line_segments_intersect(p1, p2, bezier_start, bezier_end) {
                return true;
            }
        }

        false
    }

    pub fn draw(
        &self,
        painter: &Painter,
        color: Color32,
        renderer: &mut ConnectionRenderer,
        connection_id: &NanoId,
    ) {
        let points = self.get_bezier_points(renderer, connection_id);
        painter.add(egui::Shape::line(points, Stroke::new(2.0, color)));
    }

    pub fn debug_draw(&self, painter: &Painter, zoom: f32) {
        let control_radius = 4.0 * zoom;
        let line_stroke = Stroke::new(1.0 * zoom, Color32::from_rgb(255, 200, 100));
        let point_color = Color32::from_rgb(255, 150, 0);

        painter.line_segment([self.start, self.control1], line_stroke);
        painter.circle_filled(self.control1, control_radius, point_color);

        painter.line_segment([self.control2, self.end], line_stroke);
        painter.circle_filled(self.control2, control_radius, point_color);

        painter.text(
            self.control1 + Vec2::new(10.0 * zoom, 0.0),
            egui::Align2::LEFT_CENTER,
            "C1",
            egui::FontId::proportional(10.0 * zoom),
            point_color,
        );

        painter.text(
            self.control2 + Vec2::new(10.0 * zoom, 0.0),
            egui::Align2::LEFT_CENTER,
            "C2",
            egui::FontId::proportional(10.0 * zoom),
            point_color,
        );
    }
}

pub struct ConnectionRenderer {
    bezier_cache: HashMap<NanoId, Vec<Pos2>>,
}

impl ConnectionRenderer {
    pub fn new() -> Self {
        Self {
            bezier_cache: HashMap::new(),
        }
    }

    pub fn get_or_compute_bezier(
        &mut self,
        connection_id: &NanoId,
        p0: Pos2,
        p1: Pos2,
        p2: Pos2,
        p3: Pos2,
    ) -> Vec<Pos2> {
        self.bezier_cache
            .entry(connection_id.clone())
            .or_insert_with(|| bezier_to_line_segments(p0, p1, p2, p3))
            .clone()
    }

    pub fn clear_cache(&mut self) {
        self.bezier_cache.clear();
    }

    pub fn invalidate_connection(&mut self, connection_id: &NanoId) {
        self.bezier_cache.remove(connection_id);
    }

    pub fn cache_size(&self) -> usize {
        self.bezier_cache.len()
    }

    pub fn update_pan_offset(&mut self, pan_delta: Vec2) {
        for points in self.bezier_cache.values_mut() {
            for point in points.iter_mut() {
                *point += pan_delta;
            }
        }
    }
}

impl Default for ConnectionRenderer {
    fn default() -> Self {
        Self::new()
    }
}

pub fn calculate_bezier_control_points(
    start: Pos2,
    end: Pos2,
    is_error_branch: bool,
) -> (Vec2, Vec2) {
    match UiConstants::FLOW_DIRECTION {
        FlowDirection::Horizontal => {
            let distance = (end.x - start.x).abs();
            let control_offset = (distance * 0.5).max(UiConstants::BEZIER_CONTROL_OFFSET);
            (
                Vec2::new(control_offset, 0.0),
                Vec2::new(control_offset, 0.0),
            )
        }
        FlowDirection::Vertical => {
            if is_error_branch {
                let dx = end.x - start.x;
                let dy = end.y - start.y;

                let h = dx.abs().clamp(80.0, 200.0);
                let v = dy.abs().clamp(80.0, 200.0);

                (
                    Vec2::new(dx.signum() * h, 0.0),
                    Vec2::new(0.0, dy.signum() * v),
                )
            } else {
                let distance = (end.y - start.y).abs();
                let control_offset = (distance * 0.5).max(UiConstants::BEZIER_CONTROL_OFFSET);
                (
                    Vec2::new(0.0, control_offset),
                    Vec2::new(0.0, control_offset),
                )
            }
        }
    }
}

pub fn bezier_to_line_segments(p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2) -> Vec<Pos2> {
    let steps = UiConstants::BEZIER_STEPS;
    let mut points = Vec::with_capacity(steps + 1);

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;

        let x = mt3 * p0.x + 3.0 * mt2 * t * p1.x + 3.0 * mt * t2 * p2.x + t3 * p3.x;
        let y = mt3 * p0.y + 3.0 * mt2 * t * p1.y + 3.0 * mt * t2 * p2.y + t3 * p3.y;

        points.push(Pos2::new(x, y));
    }

    points
}

fn point_to_bezier_distance(point: Pos2, bezier_points: &[Pos2]) -> f32 {
    let mut min_distance = f32::MAX;

    for i in 0..bezier_points.len() - 1 {
        let segment_start = bezier_points[i];
        let segment_end = bezier_points[i + 1];

        let segment = segment_end - segment_start;
        let point_vec = point - segment_start;

        let segment_length_sq = segment.length_sq();
        if segment_length_sq < 0.0001 {
            min_distance = min_distance.min(point_vec.length());
            continue;
        }

        let t = (point_vec.dot(segment) / segment_length_sq).clamp(0.0, 1.0);
        let projection = segment_start + segment * t;
        let distance = (point - projection).length();

        min_distance = min_distance.min(distance);
    }

    min_distance
}

fn line_segments_intersect(p1: Pos2, p2: Pos2, p3: Pos2, p4: Pos2) -> bool {
    let d = (p2.x - p1.x) * (p4.y - p3.y) - (p2.y - p1.y) * (p4.x - p3.x);
    if d.abs() < 0.0001 {
        return false;
    }

    let t = ((p3.x - p1.x) * (p4.y - p3.y) - (p3.y - p1.y) * (p4.x - p3.x)) / d;
    let u = ((p3.x - p1.x) * (p2.y - p1.y) - (p3.y - p1.y) * (p2.x - p1.x)) / d;

    (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pin_position_output() {
        let node = create_test_node();
        let pos = PinPosition::output(&node, &BranchType::Default);
        assert_eq!(pos.world_pos(), node.get_output_pin_pos());
    }

    #[test]
    fn test_pin_position_input() {
        let node = create_test_node();
        let pos = PinPosition::input(&node);
        assert_eq!(pos.world_pos(), node.get_input_pin_pos());
    }

    #[test]
    fn test_connection_renderer_cache() {
        let mut renderer = ConnectionRenderer::new();
        assert_eq!(renderer.cache_size(), 0);

        let id = NanoId::new("test123");
        let points = renderer.get_or_compute_bezier(
            &id,
            Pos2::ZERO,
            Pos2::new(1.0, 1.0),
            Pos2::new(2.0, 2.0),
            Pos2::new(3.0, 3.0),
        );
        assert!(!points.is_empty());
        assert_eq!(renderer.cache_size(), 1);

        renderer.invalidate_connection(&id);
        assert_eq!(renderer.cache_size(), 0);
    }

    #[test]
    fn test_bezier_steps() {
        let points = bezier_to_line_segments(
            Pos2::ZERO,
            Pos2::new(10.0, 0.0),
            Pos2::new(20.0, 0.0),
            Pos2::new(30.0, 0.0),
        );
        assert_eq!(points.len(), UiConstants::BEZIER_STEPS + 1);
    }

    #[test]
    fn test_line_intersection() {
        assert!(line_segments_intersect(
            Pos2::new(0.0, 0.0),
            Pos2::new(2.0, 2.0),
            Pos2::new(0.0, 2.0),
            Pos2::new(2.0, 0.0),
        ));

        assert!(!line_segments_intersect(
            Pos2::new(0.0, 0.0),
            Pos2::new(1.0, 0.0),
            Pos2::new(0.0, 1.0),
            Pos2::new(1.0, 1.0),
        ));
    }

    fn create_test_node() -> Node {
        Node {
            id: NanoId::new("test"),
            activity: rpa_core::Activity::Start {
                scenario_id: NanoId::new("scenario"),
            },
            position: Pos2::new(100.0, 100.0),
            width: 180.0,
            height: 60.0,
        }
    }
}
