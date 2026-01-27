use crate::ext::NodeExt;
use crate::ui_constants::UiConstants;
use egui::epaint::CubicBezierShape;
use egui::{Color32, Painter, Pos2, Stroke};
use rpa_core::{BranchType, Node};
use shared::NanoId;
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

pub fn calculate_manhattan_waypoints(start: Pos2, end: Pos2) -> Vec<Pos2> {
    let dx = (end.x - start.x).abs();

    if dx < UiConstants::CONNECTION_ALIGNMENT_THRESHOLD {
        return vec![start, end];
    }

    let mid_y = start.y + UiConstants::CONNECTION_PIN_EXIT_OFFSET;

    vec![
        start,
        Pos2::new(start.x, mid_y),
        Pos2::new(end.x, mid_y),
        end,
    ]
}

enum PathSegment {
    Line(Pos2, Pos2),
    Bezier {
        start: Pos2,
        ctrl1: Pos2,
        ctrl2: Pos2,
        end: Pos2,
    },
}

fn round_manhattan_corners(waypoints: &[Pos2], radius: f32) -> Vec<PathSegment> {
    if waypoints.len() < 3 {
        if waypoints.len() == 2 {
            return vec![PathSegment::Line(waypoints[0], waypoints[1])];
        }
        return vec![];
    }

    let mut segments = Vec::new();
    let mut current_pos = waypoints[0];

    for i in 0..waypoints.len() - 1 {
        let p0 = waypoints[i];
        let p1 = waypoints[i + 1];

        let v = p1 - p0;
        let length = v.length();

        if i == waypoints.len() - 2 {
            segments.push(PathSegment::Line(current_pos, p1));
            break;
        }

        let p2 = waypoints[i + 2];
        let v_next = p2 - p1;
        let next_length = v_next.length();

        let use_radius = radius.min(length * 0.5).min(next_length * 0.5);

        if use_radius < 1.0 || length < 0.1 {
            segments.push(PathSegment::Line(current_pos, p1));
            current_pos = p1;
            continue;
        }

        let dir = v / length;
        let dir_next = v_next / next_length;

        let corner_start = p1 - dir * use_radius;
        let corner_end = p1 + dir_next * use_radius;

        if (current_pos - corner_start).length() > 0.1 {
            segments.push(PathSegment::Line(current_pos, corner_start));
        }

        let ctrl_factor = 0.5522847498;
        let ctrl1 = corner_start + dir * use_radius * ctrl_factor;
        let ctrl2 = corner_end - dir_next * use_radius * ctrl_factor;

        segments.push(PathSegment::Bezier {
            start: corner_start,
            ctrl1,
            ctrl2,
            end: corner_end,
        });

        current_pos = corner_end;
    }

    segments
}

fn sample_bezier(start: Pos2, ctrl1: Pos2, ctrl2: Pos2, end: Pos2, steps: usize) -> Vec<Pos2> {
    let mut points = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;

        let x = mt3 * start.x + 3.0 * mt2 * t * ctrl1.x + 3.0 * mt * t2 * ctrl2.x + t3 * end.x;
        let y = mt3 * start.y + 3.0 * mt2 * t * ctrl1.y + 3.0 * mt * t2 * ctrl2.y + t3 * end.y;

        points.push(Pos2::new(x, y));
    }
    points
}

fn segments_to_points(segments: &[PathSegment]) -> Vec<Pos2> {
    let mut points = Vec::new();
    for segment in segments {
        match segment {
            PathSegment::Line(start, end) => {
                if points.is_empty() {
                    points.push(*start);
                }
                points.push(*end);
            }
            PathSegment::Bezier {
                start,
                ctrl1,
                ctrl2,
                end,
            } => {
                let sampled = sample_bezier(*start, *ctrl1, *ctrl2, *end, 10);
                if points.is_empty() {
                    points.extend(sampled);
                } else {
                    points.extend(&sampled[1..]);
                }
            }
        }
    }
    points
}

pub struct ConnectionPath {
    waypoints: Vec<Pos2>,
}

impl ConnectionPath {
    pub fn new(from: &Node, to: &Node, _nodes: &[Node], branch: &BranchType) -> Self {
        let start = Self::get_output_pin_for_branch(from, branch);
        let end = to.get_input_pin_pos();

        Self {
            waypoints: calculate_manhattan_waypoints(start, end),
        }
    }

    pub fn get_output_pin_for_branch(node: &Node, branch: &BranchType) -> Pos2 {
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
        let segments =
            round_manhattan_corners(&self.waypoints, UiConstants::CONNECTION_CORNER_RADIUS);
        let stroke = Stroke::new(2.0, color);

        for segment in segments {
            match segment {
                PathSegment::Line(start, end) => {
                    painter.line_segment([transform(start), transform(end)], stroke);
                }
                PathSegment::Bezier {
                    start,
                    ctrl1,
                    ctrl2,
                    end,
                } => {
                    let points = [
                        transform(start),
                        transform(ctrl1),
                        transform(ctrl2),
                        transform(end),
                    ];
                    painter.add(CubicBezierShape::from_points_stroke(
                        points,
                        false,
                        Color32::TRANSPARENT,
                        stroke,
                    ));
                }
            }
        }

        renderer.cache_segments(id, &self.waypoints);
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
        let segments = round_manhattan_corners(wp, UiConstants::CONNECTION_CORNER_RADIUS);
        let pts = segments_to_points(&segments);
        self.cache.insert(
            id.clone(),
            CacheEntry {
                generation: self.generation,
                points: pts.clone(),
            },
        );
        pts
    }

    pub fn cache_segments(&mut self, id: &NanoId, wp: &[Pos2]) {
        if let Some(e) = self.cache.get(id) {
            if e.generation == self.generation {
                return;
            }
        }
        let segments = round_manhattan_corners(wp, UiConstants::CONNECTION_CORNER_RADIUS);
        let pts = segments_to_points(&segments);
        self.cache.insert(
            id.clone(),
            CacheEntry {
                generation: self.generation,
                points: pts,
            },
        );
    }
}

impl Default for ConnectionRenderer {
    fn default() -> Self {
        Self::new()
    }
}
