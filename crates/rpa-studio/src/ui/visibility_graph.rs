use egui::Pos2;
use rpa_core::{constants::OutputDirection, Node};
use std::collections::HashMap;

pub struct RoutingPath {
    pub waypoints: Vec<Pos2>,
    pub ghost_pin: Pos2,      // output ghost pin
    pub ghost_input: Pos2,    // input ghost pin
}

/// Directional rectangular notch carved from source node for directional exit.
/// Allows A* to escape from source node in the preferred output direction only.
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct ExitNotch {
    min: Pos2,
    max: Pos2,
}

impl ExitNotch {
    #[allow(dead_code)]
    pub fn create(source_rect: egui::Rect, preferred_direction: OutputDirection, notch_length: f32, notch_thickness: f32) -> Self {
        let (min, max) = match preferred_direction {
            OutputDirection::Down => {
                let notch_min_x = source_rect.center().x - notch_thickness;
                let notch_max_x = source_rect.center().x + notch_thickness;
                let notch_min_y = source_rect.max.y;
                let notch_max_y = source_rect.max.y + notch_length;
                (Pos2::new(notch_min_x, notch_min_y), Pos2::new(notch_max_x, notch_max_y))
            }
            OutputDirection::Up => {
                let notch_min_x = source_rect.center().x - notch_thickness;
                let notch_max_x = source_rect.center().x + notch_thickness;
                let notch_min_y = source_rect.min.y - notch_length;
                let notch_max_y = source_rect.min.y;
                (Pos2::new(notch_min_x, notch_min_y), Pos2::new(notch_max_x, notch_max_y))
            }
            OutputDirection::Right => {
                let notch_min_x = source_rect.max.x;
                let notch_max_x = source_rect.max.x + notch_length;
                let notch_min_y = source_rect.center().y - notch_thickness;
                let notch_max_y = source_rect.center().y + notch_thickness;
                (Pos2::new(notch_min_x, notch_min_y), Pos2::new(notch_max_x, notch_max_y))
            }
            OutputDirection::Left => {
                let notch_min_x = source_rect.min.x - notch_length;
                let notch_max_x = source_rect.min.x;
                let notch_min_y = source_rect.center().y - notch_thickness;
                let notch_max_y = source_rect.center().y + notch_thickness;
                (Pos2::new(notch_min_x, notch_min_y), Pos2::new(notch_max_x, notch_max_y))
            }
        };

        Self { min, max }
    }

    #[allow(dead_code)]
    fn segment_intersects(&self, p1: Pos2, p2: Pos2) -> bool {
        segment_intersects_rect(p1, p2, self.min, self.max)
    }
}

#[derive(Clone, Copy)]
struct Obstacle {
    min: Pos2,
    max: Pos2,
}

impl Obstacle {
    fn from_node(node: &Node, padding: f32) -> Self {
        Self {
            min: Pos2::new(node.position.x - padding, node.position.y - padding),
            max: Pos2::new(
                node.position.x + node.width + padding,
                node.position.y + node.height + padding,
            ),
        }
    }

    #[allow(dead_code)]
    fn contains(&self, pos: Pos2) -> bool {
        pos.x >= self.min.x && pos.x <= self.max.x && pos.y >= self.min.y && pos.y <= self.max.y
    }

    fn get_corners(&self) -> [Pos2; 4] {
        [
            self.min,
            Pos2::new(self.max.x, self.min.y),
            self.max,
            Pos2::new(self.min.x, self.max.y),
        ]
    }
}

pub struct VisibilityGraph {
    obstacles: Vec<Obstacle>,
}

impl VisibilityGraph {
    #[allow(dead_code)]
    pub fn new(nodes: &[Node], padding: f32) -> Self {
        let obstacles = nodes.iter().map(|n| Obstacle::from_node(n, padding)).collect();
        Self { obstacles }
    }
    #[allow(dead_code)]
    fn find_path(&self, start: Pos2, end: Pos2) -> Vec<Pos2> {
        if start == end {
            return vec![start, end];
        }

        match self.find_path_astar(start, end) {
            Some(path) => simplify_path(&path, &self.obstacles),
            None => fallback_manhattan(start, end),
        }
    }

    pub fn find_path_with_ghost_pins(
        &self,
        start: Pos2,
        end: Pos2,
        preferred_output_direction: OutputDirection,
        ghost_distance: f32,
        _source_node_rect: egui::Rect,
    ) -> RoutingPath {
        if start == end {
            return RoutingPath {
                waypoints: vec![start, end],
                ghost_pin: start,
                ghost_input: end,
            };
        }

        let ghost_pin = match preferred_output_direction {
            OutputDirection::Down => Pos2::new(start.x, start.y + ghost_distance),
            OutputDirection::Right => Pos2::new(start.x + ghost_distance, start.y),
            OutputDirection::Left => Pos2::new(start.x - ghost_distance, start.y),
            OutputDirection::Up => Pos2::new(start.x, start.y - ghost_distance),
        };

        let ghost_input = Pos2::new(end.x, end.y - ghost_distance);

        let mut waypoints = vec![start, ghost_pin];

        let middle_path = self
            .find_path_astar(ghost_pin, ghost_input)
            .map(|path| simplify_path(&path, &self.obstacles))
            .unwrap_or_else(|| vec![ghost_pin, ghost_input]);

        waypoints.extend_from_slice(&middle_path[1..]);

        if waypoints.last() != Some(&end) {
            waypoints.push(end);
        }

        RoutingPath {
            waypoints,
            ghost_pin,
            ghost_input,
        }
    }

    fn find_path_astar(&self, start: Pos2, end: Pos2) -> Option<Vec<Pos2>> {
        let vertices = self.get_graph_vertices(start, end);

        let mut open_set: Vec<usize> = vec![0];
        let mut came_from: HashMap<usize, usize> = HashMap::new();
        let mut g_score: HashMap<usize, f32> = HashMap::new();
        let mut f_score: HashMap<usize, f32> = HashMap::new();

        let start_idx = 0;
        let end_idx = 1;

        g_score.insert(start_idx, 0.0);
        let h = heuristic(vertices[start_idx], vertices[end_idx]);
        f_score.insert(start_idx, h);

        while !open_set.is_empty() {
            let current_idx = open_set
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    f_score
                        .get(a)
                        .copied()
                        .unwrap_or(f32::INFINITY)
                        .partial_cmp(&f_score.get(b).copied().unwrap_or(f32::INFINITY))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(idx, _)| idx);

            let current_idx = match current_idx {
                Some(idx) => idx,
                None => break,
            };

            let current = open_set.remove(current_idx);

            if current == end_idx {
                return Some(reconstruct_path(&came_from, current, &vertices));
            }

            let current_g = g_score[&current];

            for neighbor_idx in 0..vertices.len() {
                if neighbor_idx == current {
                    continue;
                }

                if !self.can_see(vertices[current], vertices[neighbor_idx]) {
                    continue;
                }

                let tentative_g =
                    current_g + distance(vertices[current], vertices[neighbor_idx]);

                if let Some(&neighbor_g) = g_score.get(&neighbor_idx)
                    && tentative_g >= neighbor_g
                {
                    continue;
                }

                came_from.insert(neighbor_idx, current);
                g_score.insert(neighbor_idx, tentative_g);
                let h = heuristic(vertices[neighbor_idx], vertices[end_idx]);
                let f = tentative_g + h;
                f_score.insert(neighbor_idx, f);

                if !open_set.contains(&neighbor_idx) {
                    open_set.push(neighbor_idx);
                }
            }
        }

        None
    }

    fn get_graph_vertices(&self, start: Pos2, end: Pos2) -> Vec<Pos2> {
        let mut vertices = vec![start, end];

        for obstacle in &self.obstacles {
            for corner in &obstacle.get_corners() {
                if !vertices.contains(corner) {
                    vertices.push(*corner);
                }
            }
        }

        vertices
    }

    fn can_see(&self, a: Pos2, b: Pos2) -> bool {
        for obstacle in &self.obstacles {
            if segment_intersects_obstacle(a, b, obstacle) {
                return false;
            }
        }

        true
    }
}

fn heuristic(a: Pos2, b: Pos2) -> f32 {
    distance(a, b)
}

fn distance(a: Pos2, b: Pos2) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

fn reconstruct_path(came_from: &HashMap<usize, usize>, mut current: usize, vertices: &[Pos2]) -> Vec<Pos2> {
    let mut path = vec![vertices[current]];

    while let Some(&prev) = came_from.get(&current) {
        current = prev;
        path.push(vertices[current]);
    }

    path.reverse();
    path
}

/// Canonical rect intersection test used by obstacles, notch, and re-entry guard.
/// Uses slab test (AABB rejection + parametric slab) for consistent floating-point behavior.
fn segment_intersects_rect(p1: Pos2, p2: Pos2, min: Pos2, max: Pos2) -> bool {
    let (p1x, p1y) = (p1.x, p1.y);
    let (p2x, p2y) = (p2.x, p2.y);
    let (mx, mxx) = (min.x, max.x);
    let (my, mxy) = (min.y, max.y);

    // AABB rejection test
    if (p1x < mx && p2x < mx) || (p1x > mxx && p2x > mxx) || (p1y < my && p2y < my)
        || (p1y > mxy && p2y > mxy)
    {
        return false;
    }

    let dx = p2x - p1x;
    let dy = p2y - p1y;

    // Zero-length segment: check if point is in rect
    if dx == 0.0 && dy == 0.0 {
        return p1x >= mx && p1x <= mxx && p1y >= my && p1y <= mxy;
    }

    // Slab test for x-axis
    if dx != 0.0 {
        let t_left = (mx - p1x) / dx;
        let t_right = (mxx - p1x) / dx;
        let (t_left, t_right) = if t_left > t_right {
            (t_right, t_left)
        } else {
            (t_left, t_right)
        };

        if t_right < 0.0 || t_left > 1.0 {
            return false;
        }
    }

    // Slab test for y-axis
    if dy != 0.0 {
        let t_top = (my - p1y) / dy;
        let t_bottom = (mxy - p1y) / dy;
        let (t_top, t_bottom) = if t_top > t_bottom {
            (t_bottom, t_top)
        } else {
            (t_top, t_bottom)
        };

        if t_bottom < 0.0 || t_top > 1.0 {
            return false;
        }
    }

    true
}

fn segment_intersects_obstacle(p1: Pos2, p2: Pos2, obstacle: &Obstacle) -> bool {
    segment_intersects_rect(p1, p2, obstacle.min, obstacle.max)
}

fn simplify_path(waypoints: &[Pos2], obstacles: &[Obstacle]) -> Vec<Pos2> {
    if waypoints.len() <= 2 {
        return waypoints.to_vec();
    }

    let mut simplified = vec![waypoints[0]];

    let mut current_idx = 0;

    while current_idx < waypoints.len() - 1 {
        let mut furthest = current_idx + 1;

        for test_idx in (current_idx + 2)..waypoints.len() {
            let can_connect = !obstacles.iter().any(|obs| {
                segment_intersects_obstacle(
                    waypoints[current_idx],
                    waypoints[test_idx],
                    obs,
                )
            });

            if can_connect {
                furthest = test_idx;
            } else {
                break; // Stop on first blocking obstacle, next waypoint will be added in next iteration
            }
        }

        if furthest != current_idx + 1 {
            simplified.push(waypoints[furthest]);
            current_idx = furthest;
        } else {
            simplified.push(waypoints[current_idx + 1]);
            current_idx += 1;
        }
    }

    if simplified.last() != Some(&waypoints[waypoints.len() - 1]) {
        simplified.push(waypoints[waypoints.len() - 1]);
    }

    simplified
}

fn fallback_manhattan(start: Pos2, end: Pos2) -> Vec<Pos2> {
    let dy = end.y - start.y;
    let mid_y = start.y + dy * 0.5;
    vec![start, Pos2::new(start.x, mid_y), Pos2::new(end.x, mid_y), end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obstacle_contains() {
        let obs = Obstacle {
            min: Pos2::new(0.0, 0.0),
            max: Pos2::new(100.0, 100.0),
        };

        assert!(obs.contains(Pos2::new(50.0, 50.0)));
        assert!(obs.contains(Pos2::new(0.0, 0.0)));
        assert!(obs.contains(Pos2::new(100.0, 100.0)));
        assert!(!obs.contains(Pos2::new(150.0, 50.0)));
        assert!(!obs.contains(Pos2::new(-10.0, 50.0)));
    }

    #[test]
    fn test_obstacle_corners() {
        let obs = Obstacle {
            min: Pos2::new(0.0, 0.0),
            max: Pos2::new(100.0, 50.0),
        };

        let corners = obs.get_corners();
        assert_eq!(corners.len(), 4);
        assert_eq!(corners[0], Pos2::new(0.0, 0.0));
        assert_eq!(corners[1], Pos2::new(100.0, 0.0));
        assert_eq!(corners[2], Pos2::new(100.0, 50.0));
        assert_eq!(corners[3], Pos2::new(0.0, 50.0));
    }

    #[test]
    fn test_segment_intersects_obstacle() {
        let obs = Obstacle {
            min: Pos2::new(100.0, 100.0),
            max: Pos2::new(200.0, 150.0),
        };

        assert!(segment_intersects_obstacle(
            Pos2::new(50.0, 125.0),
            Pos2::new(250.0, 125.0),
            &obs
        ));

        assert!(!segment_intersects_obstacle(
            Pos2::new(0.0, 0.0),
            Pos2::new(50.0, 50.0),
            &obs
        ));

        assert!(!segment_intersects_obstacle(
            Pos2::new(50.0, 50.0),
            Pos2::new(75.0, 75.0),
            &obs
        ));
    }

    #[test]
    fn test_fallback_manhattan() {
        let start = Pos2::new(0.0, 0.0);
        let end = Pos2::new(100.0, 200.0);

        let path = fallback_manhattan(start, end);

        assert_eq!(path.len(), 4);
        assert_eq!(path[0], start);
        assert_eq!(path[3], end);
        assert_eq!(path[1].x, start.x);
        assert_eq!(path[2].x, end.x);
        assert_eq!(path[1].y, path[2].y);
    }

    #[test]
    fn test_visibility_graph_simple_path() {
        let graph = VisibilityGraph::new(&[], 15.0);
        let path = graph.find_path(Pos2::new(0.0, 0.0), Pos2::new(100.0, 100.0));

        assert!(!path.is_empty());
        assert_eq!(path[0], Pos2::new(0.0, 0.0));
        assert_eq!(path[path.len() - 1], Pos2::new(100.0, 100.0));
    }

    #[test]
    fn test_distance() {
        let d = distance(Pos2::new(0.0, 0.0), Pos2::new(3.0, 4.0));
        assert!((d - 5.0).abs() < 0.01);
    }
}
