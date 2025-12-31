use super::visibility_graph::VisibilityGraph;
use egui::{Color32, Painter, Pos2, Stroke, Vec2};
use rpa_core::{BranchType, NanoId, Node, UiConstants};
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

    #[allow(dead_code)]
    pub fn world_pos(&self) -> Pos2 {
        self.world_pos
    }

    #[allow(dead_code)]
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
    ghost_pin: Pos2,
    ghost_input: Pos2,
    source_rect: egui::Rect,
}

impl ConnectionPath {
    pub fn new(
        from_node: &Node,
        to_node: &Node,
        nodes: &[Node],
        branch_type: &BranchType,
    ) -> Self {
        let start = PinPosition::output(from_node, branch_type).world_pos();
        let end = PinPosition::input(to_node).world_pos();

        let preferred_direction = from_node.get_preferred_output_direction(branch_type);
        
        // Create visibility graph with all nodes as obstacles
        let visibility_graph = VisibilityGraph::new(
            nodes,
            UiConstants::ROUTING_OBSTACLE_PADDING,
        );

        let source_rect = egui::Rect {
            min: Pos2::new(
                from_node.position.x - UiConstants::ROUTING_EXPANDED_PADDING,
                from_node.position.y - UiConstants::ROUTING_EXPANDED_PADDING,
            ),
            max: Pos2::new(
                from_node.position.x + from_node.width + UiConstants::ROUTING_EXPANDED_PADDING,
                from_node.position.y + from_node.height + UiConstants::ROUTING_EXPANDED_PADDING,
            ),
        };

        let routing_path = visibility_graph.find_path_with_ghost_pins(
            start,
            end,
            preferred_direction,
            UiConstants::ROUTING_GHOST_PIN_DISTANCE,
            source_rect,
        );

        Self {
            start,
            end,
            waypoints: routing_path.waypoints,
            ghost_pin: routing_path.ghost_pin,
            ghost_input: routing_path.ghost_input,
            source_rect,
        }
    }

    pub fn get_path_points(
        &self,
        renderer: &mut ConnectionRenderer,
        connection_id: &NanoId,
    ) -> Vec<Pos2> {
        renderer.get_or_compute_path(connection_id, &self.waypoints, self.source_rect)
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
        transform: impl Fn(Pos2) -> Pos2,
    ) {
        let world_points = self.get_path_points(renderer, connection_id);
        let screen_points: Vec<Pos2> = world_points.iter().map(|p| transform(*p)).collect();
        painter.add(egui::Shape::line(screen_points, Stroke::new(2.0, color)));
    }

    pub fn draw_debug_info(&self, painter: &Painter) {
        if !UiConstants::DEBUG_ROUTING_VISUALIZATION {
            return;
        }

        painter.circle_filled(self.ghost_pin, 8.0, Color32::from_rgb(0, 255, 255));
        painter.circle_filled(self.ghost_input, 8.0, Color32::from_rgb(255, 0, 255));

        for waypoint in &self.waypoints {
            painter.circle_filled(*waypoint, 4.0, Color32::from_rgb(255, 255, 0));
        }

        painter.line_segment(
            [self.start, self.ghost_pin],
            Stroke::new(1.0, Color32::from_rgb(0, 255, 255)),
        );

        painter.line_segment(
            [self.ghost_input, self.end],
            Stroke::new(1.0, Color32::from_rgb(255, 0, 255)),
        );
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
    path_cache: HashMap<NanoId, CacheEntry>,
    routing_generation: u64,
}

struct CacheEntry {
    generation: u64,
    points: Vec<Pos2>,
}

impl ConnectionRenderer {
    pub fn new() -> Self {
        Self {
            path_cache: HashMap::new(),
            routing_generation: 0,
        }
    }

    pub fn increment_generation(&mut self) {
        self.routing_generation = self.routing_generation.wrapping_add(1);
    }

    pub fn get_or_compute_path(&mut self, connection_id: &NanoId, waypoints: &[Pos2], source_rect: egui::Rect) -> Vec<Pos2> {
        if let Some(entry) = self.path_cache.get(connection_id)
            && entry.generation == self.routing_generation
        {
            return entry.points.clone();
        }

        let points = waypoints_to_line_segments(waypoints, source_rect);
        self.path_cache.insert(
            connection_id.clone(),
            CacheEntry {
                generation: self.routing_generation,
                points: points.clone(),
            },
        );
        points
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
}

impl Default for ConnectionRenderer {
    fn default() -> Self {
        Self::new()
    }
}

fn waypoints_to_line_segments(waypoints: &[Pos2], source_rect: egui::Rect) -> Vec<Pos2> {
    if waypoints.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::new();

    for pair in waypoints.windows(2) {
        let a = pair[0];
        let b = pair[1];

        result.push(a);

        let dx = (b.x - a.x).abs();
        let dy = (b.y - a.y).abs();

        if dx > 0.01 && dy > 0.01 {
            let corner = if dx > dy {
                Pos2::new(b.x, a.y)
            } else {
                Pos2::new(a.x, b.y)
            };

            if corner != a && corner != b {
                let corner_in_source = corner.x >= source_rect.min.x && corner.x <= source_rect.max.x
                    && corner.y >= source_rect.min.y && corner.y <= source_rect.max.y;
                if !corner_in_source {
                    result.push(corner);
                }
            }
        }
    }

    // Always add final point
    if let Some(last) = waypoints.last() {
        result.push(*last);
    }

    result
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
