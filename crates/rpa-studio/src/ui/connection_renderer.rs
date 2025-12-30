use egui::{Color32, Painter, Pos2, Stroke, Vec2};
use rpa_core::{
    BranchType, NanoId, Node, UiConstants,
};
use std::collections::HashMap;
use super::visibility_graph::VisibilityGraph;


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

    #[allow(dead_code)]
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
    #[allow(dead_code)]
    start: Pos2,
    #[allow(dead_code)]
    end: Pos2,
    waypoints: Vec<Pos2>,
}

impl ConnectionPath {
    pub fn new<F>(
        from_node: &Node,
        to_node: &Node,
        nodes: &[Node],
        branch_type: &BranchType,
        transform: F,
    ) -> Self
    where
        F: Fn(Pos2) -> Pos2,
    {
        let start = PinPosition::output(from_node, branch_type).screen_pos(&transform);
        let end = PinPosition::input(to_node).screen_pos(&transform);

        let visibility_graph = VisibilityGraph::new(nodes, UiConstants::ROUTING_OBSTACLE_PADDING);
        let preferred_direction = from_node.get_preferred_output_direction(branch_type);
        let waypoints = visibility_graph.find_path_with_direction(start, end, preferred_direction);

        Self {
            start,
            end,
            waypoints,
        }
    }

    pub fn get_path_points(
        &self,
        renderer: &mut ConnectionRenderer,
        connection_id: &NanoId,
    ) -> Vec<Pos2> {
        renderer.get_or_compute_path(connection_id, &self.waypoints)
    }

    pub fn hit_test(
        &self,
        point: Pos2,
        renderer: &mut ConnectionRenderer,
        connection_id: &NanoId,
        threshold: f32,
    ) -> bool {
        let path_points = self.get_path_points(renderer, connection_id);
        let dist = point_to_line_distance(point, &path_points);
        dist < threshold
    }

    pub fn intersects_line(
        &self,
        p1: Pos2,
        p2: Pos2,
        renderer: &mut ConnectionRenderer,
        connection_id: &NanoId,
    ) -> bool {
        let path_points = self.get_path_points(renderer, connection_id);

        for i in 0..path_points.len() - 1 {
            let path_start = path_points[i];
            let path_end = path_points[i + 1];

            if line_segments_intersect(p1, p2, path_start, path_end) {
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
        let points = self.get_path_points(renderer, connection_id);
        painter.add(egui::Shape::line(points, Stroke::new(2.0, color)));
    }

    #[allow(dead_code)]
    pub fn debug_draw(&self, painter: &Painter, zoom: f32) {
        let point_radius = 4.0 * zoom;
        let point_color = Color32::from_rgb(255, 150, 0);

        for (i, waypoint) in self.waypoints.iter().enumerate() {
            painter.circle_filled(*waypoint, point_radius, point_color);
            painter.text(
                *waypoint + Vec2::new(10.0 * zoom, 0.0),
                egui::Align2::LEFT_CENTER,
                format!("W{}", i),
                egui::FontId::proportional(10.0 * zoom),
                point_color,
            );
        }
    }
}

pub struct ConnectionRenderer {
    path_cache: HashMap<NanoId, Vec<Pos2>>,
}

impl ConnectionRenderer {
    pub fn new() -> Self {
        Self {
            path_cache: HashMap::new(),
        }
    }

    pub fn get_or_compute_path(&mut self, connection_id: &NanoId, waypoints: &[Pos2]) -> Vec<Pos2> {
        self.path_cache
            .entry(connection_id.clone())
            .or_insert_with(|| waypoints_to_line_segments(waypoints))
            .clone()
    }

    pub fn clear_cache(&mut self) {
        self.path_cache.clear();
    }

    #[allow(dead_code)]
    pub fn invalidate_connection(&mut self, connection_id: &NanoId) {
        self.path_cache.remove(connection_id);
    }

    #[allow(dead_code)]
    pub fn cache_size(&self) -> usize {
        self.path_cache.len()
    }

    pub fn update_pan_offset(&mut self, pan_delta: Vec2) {
        for points in self.path_cache.values_mut() {
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

fn waypoints_to_line_segments(waypoints: &[Pos2]) -> Vec<Pos2> {
    waypoints.to_vec()
}

fn point_to_line_distance(point: Pos2, line_points: &[Pos2]) -> f32 {

    let mut min_distance = f32::MAX;

    for i in 0..line_points.len() - 1 {
        let segment_start = line_points[i];
        let segment_end = line_points[i + 1];

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

#[allow(dead_code)]
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
    fn test_connection_renderer_cache() {
        let mut renderer = ConnectionRenderer::new();
        assert_eq!(renderer.cache_size(), 0);

        let id = NanoId::new("test123");
        let waypoints = vec![
            Pos2::ZERO,
            Pos2::new(1.0, 1.0),
            Pos2::new(2.0, 2.0),
            Pos2::new(3.0, 3.0),
        ];
        let points = renderer.get_or_compute_path(&id, &waypoints);
        assert!(!points.is_empty());
        assert_eq!(renderer.cache_size(), 1);

        renderer.invalidate_connection(&id);
        assert_eq!(renderer.cache_size(), 0);
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

    #[allow(dead_code)]
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
