use crate::ext::NodeExt;
use crate::ui_constants::UiConstants;
use egui::{Color32, Painter, Pos2, Rect, Stroke, StrokeKind, pos2};
use rpa_core::Node;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Free,
    Occupied,
    SoftOccupied,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasObstacleGrid {
    cells: Vec<Vec<CellState>>,
    world_bounds: Rect,
    cell_size: f32,
    grid_width: usize,
    grid_height: usize,
    dirty: bool,
}

impl Default for CanvasObstacleGrid {
    fn default() -> Self {
        Self::new(UiConstants::ROUTING_GRID_SIZE)
    }
}

impl CanvasObstacleGrid {
    pub fn new(cell_size: f32) -> Self {
        let empty_bounds = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        Self {
            cells: vec![],
            world_bounds: empty_bounds,
            cell_size,
            grid_width: 0,
            grid_height: 0,
            dirty: true,
        }
    }

    pub fn rebuild(&mut self, nodes: &[Node], connections: &[(Pos2, Pos2)]) {
        self.world_bounds = Self::compute_world_bounds(nodes, self.cell_size);

        let width = ((self.world_bounds.width() / self.cell_size).ceil() as usize).max(1);
        let height = ((self.world_bounds.height() / self.cell_size).ceil() as usize).max(1);

        self.grid_width = width;
        self.grid_height = height;

        self.cells = vec![vec![CellState::Free; width]; height];

        for node in nodes {
            if node.is_routable() {
                let footprint = node.get_routing_footprint(self.cell_size);
                let padded = Self::add_padding_to_rect(footprint);
                self.mark_obstacle_region(padded);
            }
        }

        for &(start, end) in connections {
            self.mark_connection_region(start, end);
        }

        self.dirty = false;
    }

    pub fn world_to_grid(&self, world_pos: Pos2) -> Option<(usize, usize)> {
        if !self.world_bounds.contains(world_pos) {
            return None;
        }

        let grid_x = ((world_pos.x - self.world_bounds.left()) / self.cell_size) as usize;
        let grid_y = ((world_pos.y - self.world_bounds.top()) / self.cell_size) as usize;

        if grid_x < self.grid_width && grid_y < self.grid_height {
            Some((grid_x, grid_y))
        } else {
            None
        }
    }

    pub fn grid_to_world(&self, grid_x: usize, grid_y: usize) -> Pos2 {
        let world_x =
            self.world_bounds.left() + grid_x as f32 * self.cell_size + self.cell_size / 2.0;
        let world_y =
            self.world_bounds.top() + grid_y as f32 * self.cell_size + self.cell_size / 2.0;
        pos2(world_x, world_y)
    }

    #[allow(dead_code)]
    pub fn is_occupied(&self, grid_x: usize, grid_y: usize) -> bool {
        if grid_x >= self.grid_width || grid_y >= self.grid_height {
            return false;
        }
        self.cells[grid_y][grid_x] == CellState::Occupied
    }

    pub fn invalidate(&mut self) {
        self.dirty = true;
    }

    #[allow(dead_code)]
    pub fn dimensions(&self) -> (usize, usize) {
        (self.grid_width, self.grid_height)
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn paint_debug(&self, painter: &Painter, to_screen: impl Fn(Pos2) -> Pos2, viewport: Rect) {
        let circle_radius = UiConstants::DEBUG_CIRCLE_RADIUS;

        for grid_y in 0..self.grid_height {
            for grid_x in 0..self.grid_width {
                let cell_state = self.cells[grid_y][grid_x];
                if cell_state == CellState::Free {
                    continue;
                }

                let world_pos = self.grid_to_world(grid_x, grid_y);
                let screen_pos = to_screen(world_pos);

                if !viewport.contains(screen_pos) {
                    continue;
                }

                match cell_state {
                    CellState::Occupied => {
                        let color = Color32::from_rgba_unmultiplied(255, 0, 0, 50);
                        painter.circle_filled(screen_pos, circle_radius, color);
                    }
                    CellState::SoftOccupied => {
                        let color = Color32::from_rgba_unmultiplied(255, 255, 0, 50);
                        painter.circle_filled(screen_pos, circle_radius, color);
                    }
                    _ => {}
                };
            }
        }

        let screen_min = to_screen(self.world_bounds.min);
        let screen_max = to_screen(self.world_bounds.max);
        let bounds_screen = Rect::from_min_max(screen_min, screen_max);
        painter.rect_stroke(
            bounds_screen,
            0.0,
            Stroke::new(1.0, Color32::GRAY),
            StrokeKind::Outside,
        );
    }

    fn compute_world_bounds(nodes: &[Node], cell_size: f32) -> Rect {
        let padding = UiConstants::CANVAS_WORLD_PADDING;

        if nodes.is_empty() {
            return Rect::from_min_max(pos2(-padding, -padding), pos2(padding, padding));
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for node in nodes {
            let footprint = node.get_routing_footprint(cell_size);
            min_x = min_x.min(footprint.min.x);
            min_y = min_y.min(footprint.min.y);
            max_x = max_x.max(footprint.max.x);
            max_y = max_y.max(footprint.max.y);
        }

        let min_x = (min_x - padding).floor();
        let min_y = (min_y - padding).floor();
        let max_x = (max_x + padding).ceil();
        let max_y = (max_y + padding).ceil();

        Rect::from_min_max(pos2(min_x, min_y), pos2(max_x, max_y))
    }

    fn add_padding_to_rect(rect: Rect) -> Rect {
        let padding = UiConstants::ROUTING_OBSTACLE_PADDING;
        let min = pos2(rect.min.x - padding, rect.min.y - padding);
        let max = pos2(rect.max.x + padding, rect.max.y + padding);
        Rect::from_min_max(min, max)
    }

    fn mark_obstacle_region(&mut self, region: Rect) {
        let (min_x, min_y) = match self.world_to_grid(region.min) {
            Some((x, y)) => (x, y),
            None => (0, 0),
        };

        let (max_x, max_y) = match self.world_to_grid(region.max) {
            Some((x, y)) => ((x + 1).min(self.grid_width), (y + 1).min(self.grid_height)),
            None => (self.grid_width, self.grid_height),
        };

        for y in min_y..max_y {
            for x in min_x..max_x {
                if x < self.grid_width && y < self.grid_height {
                    self.cells[y][x] = CellState::Occupied;
                }
            }
        }
    }

    fn mark_connection_region(&mut self, start: Pos2, end: Pos2) {
        let padding = UiConstants::ROUTING_OBSTACLE_PADDING;

        let dx = (end.x - start.x).abs();
        let dy = (end.y - start.y).abs();

        let steps = (dx.max(dy) / self.cell_size).ceil() as usize;
        if steps == 0 {
            return;
        }

        for i in 0..=steps {
            let t = if steps > 0 {
                i as f32 / steps as f32
            } else {
                0.0
            };

            let point = pos2(
                start.x + (end.x - start.x) * t,
                start.y + (end.y - start.y) * t,
            );

            let padded_min = pos2(point.x - padding, point.y - padding);
            let padded_max = pos2(point.x + padding, point.y + padding);
            let padded_rect = Rect::from_min_max(padded_min, padded_max);

            let (min_x, min_y) = match self.world_to_grid(padded_rect.min) {
                Some((x, y)) => (x, y),
                None => continue,
            };

            let (max_x, max_y) = match self.world_to_grid(padded_rect.max) {
                Some((x, y)) => ((x + 1).min(self.grid_width), (y + 1).min(self.grid_height)),
                None => continue,
            };

            for y in min_y..max_y {
                for x in min_x..max_x {
                    if x < self.grid_width
                        && y < self.grid_height
                        && self.cells[y][x] != CellState::Occupied
                    {
                        self.cells[y][x] = CellState::SoftOccupied;
                    }
                }
            }
        }
    }
}
