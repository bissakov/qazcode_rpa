use egui::{Color32, Painter, Pos2, Stroke};
use rpa_core::{BranchType, NanoId, Node};
use std::collections::HashMap;

fn point_to_polyline_distance(p: Pos2, pts: &[Pos2]) -> f32 {
    let mut min = f32::MAX;
    for w in pts.windows(2) {
        min = min.min(point_segment_distance(p, w[0], w[1]));
    }
    min
}

fn point_segment_distance(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let ap = p - a;
    let len2 = ab.length_sq();
    if len2 < 1e-6 {
        return ap.length();
    }
    let t = (ap.dot(ab) / len2).clamp(0.0, 1.0);
    let proj = a + ab * t;
    (p - proj).length()
}

fn segments_intersect(a1: Pos2, a2: Pos2, b1: Pos2, b2: Pos2) -> Option<Pos2> {
    fn orient(a: Pos2, b: Pos2, c: Pos2) -> f32 {
        (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
    }

    fn on_segment(a: Pos2, b: Pos2, c: Pos2) -> bool {
        b.x <= a.x.max(c.x) && b.x >= a.x.min(c.x) && b.y <= a.y.max(c.y) && b.y >= a.y.min(c.y)
    }

    let d1 = a2 - a1;
    let d2 = b2 - b1;
    let d = d1.x * d2.y - d1.y * d2.x;

    if d.abs() < 1e-6 {
        let o1 = orient(a1, a2, b1);
        if o1.abs() < 1e-6 && on_segment(a1, b1, a2) {
            return Some(b1);
        }
        return None;
    }

    let t1 = ((b1.x - a1.x) * d2.y - (b1.y - a1.y) * d2.x) / d;
    let t2 = ((b1.x - a1.x) * d1.y - (b1.y - a1.y) * d1.x) / d;

    if (0.0..=1.0).contains(&t1) && (0.0..=1.0).contains(&t2) {
        let ix = a1.x + t1 * d1.x;
        let iy = a1.y + t1 * d1.y;
        return Some(Pos2::new(ix, iy));
    }

    None
}

pub struct ConnectionPath {
    waypoints: Vec<Pos2>,
}

impl ConnectionPath {
    pub fn new(from: &Node, to: &Node, _nodes: &[Node], branch: &BranchType) -> Self {
        let start = Self::get_output_pin_for_branch(from, branch);
        let end = to.get_input_pin_pos();

        Self {
            waypoints: vec![start, end],
        }
    }

    fn get_output_pin_for_branch(node: &Node, branch: &BranchType) -> Pos2 {
        match branch {
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
        }
    }

    pub fn get_path_points(&self, renderer: &mut ConnectionRenderer, id: &NanoId) -> Vec<Pos2> {
        renderer.get_or_compute(id, &self.waypoints)
    }

    pub fn draw(
        &self,
        painter: &Painter,
        color: Color32,
        renderer: &mut ConnectionRenderer,
        id: &NanoId,
        transform: impl Fn(Pos2) -> Pos2,
    ) {
        let pts: Vec<Pos2> = self
            .get_path_points(renderer, id)
            .into_iter()
            .map(transform)
            .collect();

        if !pts.is_empty() {
            painter.add(egui::Shape::line(pts, Stroke::new(2.0, color)));
        }
    }

    pub fn hit_test(
        &self,
        point: Pos2,
        renderer: &mut ConnectionRenderer,
        id: &NanoId,
        threshold: f32,
    ) -> bool {
        let pts = self.get_path_points(renderer, id);
        point_to_polyline_distance(point, &pts) <= threshold
    }

    pub fn intersects_line(
        &self,
        p1: Pos2,
        p2: Pos2,
        renderer: &mut ConnectionRenderer,
        id: &NanoId,
    ) -> Option<Pos2> {
        let pts = self.get_path_points(renderer, id);
        for w in pts.windows(2) {
            if let Some(intersection_point) = segments_intersect(p1, p2, w[0], w[1]) {
                return Some(intersection_point);
            }
        }
        None
    }
}

pub struct ConnectionRenderer {
    generation: u64,
    cache: HashMap<NanoId, CacheEntry>,
}

struct CacheEntry {
    generation: u64,
    points: Vec<Pos2>,
}

impl ConnectionRenderer {
    pub fn new() -> Self {
        Self {
            generation: 0,
            cache: HashMap::new(),
        }
    }

    pub fn increment_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    pub fn get_or_compute(&mut self, id: &NanoId, wp: &[Pos2]) -> Vec<Pos2> {
        if let Some(e) = self.cache.get(id)
            && e.generation == self.generation
        {
            return e.points.clone();
        }
        let pts = wp.to_vec();
        self.cache.insert(
            id.clone(),
            CacheEntry {
                generation: self.generation,
                points: pts.clone(),
            },
        );
        pts
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

impl Default for ConnectionRenderer {
    fn default() -> Self {
        Self::new()
    }
}
