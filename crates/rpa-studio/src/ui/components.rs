use crate::colors::ColorPalette;
use crate::ext::{ActivityExt, NodeExt};
use crate::ui::config::{GridStyle, NodeStyle, PinStyle};
use crate::ui_constants::UiConstants;
use egui::{Color32, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2};
use rpa_core::{Activity, BranchType, Node, Scenario};
use shared::NanoId;
use std::collections::HashMap;

type NodeIndex = HashMap<NanoId, usize>;

fn build_node_index(nodes: &[Node]) -> NodeIndex {
    nodes
        .iter()
        .enumerate()
        .map(|(idx, n)| (n.id.clone(), idx))
        .collect()
}

fn get_node_from_index<'a>(nodes: &'a [Node], index: &NodeIndex, id: &NanoId) -> Option<&'a Node> {
    index.get(id).and_then(|&idx| nodes.get(idx))
}

pub struct NodeRenderer {
    style: NodeStyle,
}

impl NodeRenderer {
    pub fn new(style: NodeStyle) -> Self {
        Self { style }
    }

    pub fn draw<F>(
        &self,
        painter: &Painter,
        node: &Node,
        is_selected: bool,
        is_hovering_connection: bool,
        is_being_resized: bool,
        to_screen: F,
        zoom: f32,
    ) where
        F: Fn(Pos2) -> Pos2,
    {
        let rect_world = node.get_rect();
        let rect = Rect::from_min_max(to_screen(rect_world.min), to_screen(rect_world.max));
        let mut color = node.activity.get_color();
        let rounding = self.style.rounding * zoom;

        if is_hovering_connection {
            let boost = self.style.hover_brightness_boost;
            let r = color.r().saturating_add(boost);
            let g = color.g().saturating_add(boost);
            let b = color.b().saturating_add(boost);
            color = Color32::from_rgb(r, g, b);
        }

        let shadow_offset = self.style.shadow_offset * zoom;
        painter.rect_filled(
            rect.translate(shadow_offset),
            rounding,
            self.style.shadow_color,
        );

        painter.rect_filled(rect, rounding, color);

        if is_being_resized {
            painter.rect_stroke(
                rect,
                rounding,
                Stroke::new(
                    self.style.resizing_stroke_width * zoom,
                    self.style.resizing_stroke_color,
                ),
                StrokeKind::Outside,
            );
        } else if is_selected {
            painter.rect_stroke(
                rect,
                rounding,
                Stroke::new(
                    self.style.selected_stroke_width * zoom,
                    self.style.selected_stroke_color,
                ),
                StrokeKind::Outside,
            );
        } else {
            painter.rect_stroke(
                rect,
                rounding,
                Stroke::new(1.0 * zoom, Color32::from_rgb(20, 20, 20)),
                StrokeKind::Outside,
            );
        }

        if let Activity::Note { text, .. } = &node.activity {
            let padding = UiConstants::NOTE_PADDING * zoom;
            let text_rect = Rect::from_min_max(
                rect.min + Vec2::new(padding, padding),
                rect.max - Vec2::new(padding, padding),
            );

            let font_id = egui::FontId::proportional(UiConstants::NOTE_FONT_SIZE * zoom);
            let mut job = egui::text::LayoutJob::default();
            job.wrap.max_width = text_rect.width();
            job.append(
                text,
                0.0,
                egui::text::TextFormat {
                    font_id: font_id.clone(),
                    color: Color32::from_rgb(60, 60, 60),
                    ..Default::default()
                },
            );

            let galley = painter.layout_job(job);
            painter.galley(text_rect.min, galley, Color32::from_rgb(60, 60, 60));
        } else if zoom >= UiConstants::GRID_MIN_ZOOM {
            let text_pos = rect.min
                + Vec2::new(
                    UiConstants::NOTE_PADDING * zoom,
                    UiConstants::NOTE_PADDING * zoom,
                );
            painter.text(
                text_pos,
                egui::Align2::LEFT_TOP,
                node.activity.get_name(),
                egui::FontId::proportional(UiConstants::NODE_LABEL_FONT_SIZE * zoom),
                Color32::WHITE,
            );
        }
    }
}

impl Default for NodeRenderer {
    fn default() -> Self {
        Self::new(NodeStyle::default())
    }
}

pub struct PinRenderer {
    style: PinStyle,
}

impl PinRenderer {
    pub fn new(style: PinStyle) -> Self {
        Self { style }
    }

    pub fn draw_input_pin<F>(&self, painter: &Painter, node: &Node, to_screen: F, zoom: f32)
    where
        F: Fn(Pos2) -> Pos2,
    {
        if !node.has_input_pin() || zoom < UiConstants::GRID_MIN_ZOOM {
            return;
        }

        let input_pin = to_screen(node.get_input_pin_pos());
        painter.circle_filled(
            input_pin,
            self.style.radius * zoom,
            Color32::from_rgb(150, 150, 150),
        );
        painter.circle_stroke(
            input_pin,
            self.style.radius * zoom,
            Stroke::new(1.0 * zoom, Color32::from_rgb(80, 80, 80)),
        );
    }

    pub fn draw_output_pins<F>(&self, painter: &Painter, node: &Node, to_screen: F, zoom: f32)
    where
        F: Fn(Pos2) -> Pos2 + Copy,
    {
        if !node.has_output_pin() {
            return;
        }

        let positions = node.get_output_pin_positions();
        for (pin_index, _) in positions.iter().enumerate() {
            let pin_screen = to_screen(node.get_output_pin_pos_by_index(pin_index));
            let branch_type = node.get_branch_type_for_pin(pin_index);

            let (color, stroke_color, label) =
                self.get_pin_appearance(&branch_type, node.get_output_pin_count());

            if zoom >= UiConstants::GRID_MIN_ZOOM {
                painter.circle_filled(pin_screen, self.style.radius * zoom, color);
                painter.circle_stroke(
                    pin_screen,
                    self.style.radius * zoom,
                    Stroke::new(1.0 * zoom, stroke_color),
                );

                if !label.is_empty() {
                    let label_offset = Vec2::new(0.0, -self.style.label_offset * zoom);
                    let label_align = egui::Align2::CENTER_BOTTOM;
                    painter.text(
                        pin_screen + label_offset,
                        label_align,
                        label,
                        egui::FontId::proportional(self.style.label_font_size * zoom),
                        color,
                    );
                }
            }
        }
    }

    fn get_pin_appearance(
        &self,
        branch_type: &BranchType,
        pin_count: usize,
    ) -> (Color32, Color32, &'static str) {
        match branch_type {
            BranchType::TrueBranch => (ColorPalette::PIN_TRUE, Color32::from_rgb(60, 120, 60), "T"),
            BranchType::FalseBranch => {
                (ColorPalette::PIN_FALSE, Color32::from_rgb(120, 60, 60), "F")
            }
            BranchType::LoopBody => (
                ColorPalette::PIN_LOOP_BODY,
                Color32::from_rgb(200, 120, 0),
                "B",
            ),
            BranchType::ErrorBranch => {
                (ColorPalette::PIN_ERROR, Color32::from_rgb(120, 60, 60), "E")
            }
            BranchType::TryBranch => (
                ColorPalette::PIN_SUCCESS,
                Color32::from_rgb(60, 120, 60),
                "T",
            ),
            BranchType::CatchBranch => {
                (ColorPalette::PIN_ERROR, Color32::from_rgb(120, 60, 60), "C")
            }
            BranchType::Default => {
                if pin_count > 1 {
                    (
                        ColorPalette::PIN_LOOP_NEXT,
                        Color32::from_rgb(80, 80, 80),
                        "N",
                    )
                } else {
                    (ColorPalette::PIN_DEFAULT, Color32::from_rgb(80, 80, 80), "")
                }
            }
        }
    }
}

impl Default for PinRenderer {
    fn default() -> Self {
        Self::new(PinStyle::default())
    }
}

pub struct GridRenderer {
    style: GridStyle,
}

impl GridRenderer {
    pub fn new(style: GridStyle) -> Self {
        Self { style }
    }

    pub fn draw(
        &self,
        painter: &Painter,
        rect: Rect,
        pan_offset: Vec2,
        zoom: f32,
        show_grid: bool,
    ) {
        if !show_grid {
            return;
        }

        if zoom < self.style.min_zoom {
            return;
        }

        let world_min_x = (rect.left() - pan_offset.x) / zoom;
        let world_max_x = (rect.right() - pan_offset.x) / zoom;
        let world_min_y = (rect.top() - pan_offset.y) / zoom;
        let world_max_y = (rect.bottom() - pan_offset.y) / zoom;

        let grid_min_x = (world_min_x / self.style.spacing).floor() * self.style.spacing;
        let grid_max_x = (world_max_x / self.style.spacing).ceil() * self.style.spacing;
        let grid_min_y = (world_min_y / self.style.spacing).floor() * self.style.spacing;
        let grid_max_y = (world_max_y / self.style.spacing).ceil() * self.style.spacing;

        let num_x_lines = ((grid_max_x - grid_min_x) / self.style.spacing).ceil() as usize;
        let num_y_lines = ((grid_max_y - grid_min_y) / self.style.spacing).ceil() as usize;

        if num_x_lines > self.style.max_lines || num_y_lines > self.style.max_lines {
            return;
        }

        let mut grid_x = grid_min_x;
        while grid_x <= grid_max_x {
            let screen_x = grid_x * zoom + pan_offset.x;
            painter.line_segment(
                [
                    Pos2::new(screen_x, rect.top()),
                    Pos2::new(screen_x, rect.bottom()),
                ],
                Stroke::new(1.0, self.style.color),
            );
            grid_x += self.style.spacing;
        }

        let mut grid_y = grid_min_y;
        while grid_y <= grid_max_y {
            let screen_y = grid_y * zoom + pan_offset.y;
            painter.line_segment(
                [
                    Pos2::new(rect.left(), screen_y),
                    Pos2::new(rect.right(), screen_y),
                ],
                Stroke::new(1.0, self.style.color),
            );
            grid_y += self.style.spacing;
        }
    }

    pub fn draw_border(&self, painter: &Painter, rect: Rect, is_executing: bool) {
        painter.rect_stroke(
            rect,
            0.0,
            if is_executing {
                Stroke::new(
                    UiConstants::CANVAS_BORDER_STROKE_WIDTH,
                    ColorPalette::CANVAS_EXECUTING_BG_COLOR,
                )
            } else {
                Stroke::new(
                    UiConstants::CANVAS_BORDER_STROKE_WIDTH,
                    Color32::TRANSPARENT,
                )
            },
            StrokeKind::Inside,
        );
    }
}

impl Default for GridRenderer {
    fn default() -> Self {
        Self::new(GridStyle::default())
    }
}

pub struct MinimapRenderer;

impl MinimapRenderer {
    pub fn new() -> Self {
        Self
    }

    pub fn draw(
        &self,
        painter: &Painter,
        scenario: &Scenario,
        pan_offset: Vec2,
        zoom: f32,
        canvas_rect: Rect,
    ) {
        let node_index = build_node_index(&scenario.nodes);

        let minimap_size = egui::vec2(UiConstants::MINIMAP_WIDTH, UiConstants::MINIMAP_HEIGHT);
        let minimap_pos = canvas_rect.max
            - minimap_size
            - egui::vec2(UiConstants::MINIMAP_OFFSET_X, UiConstants::MINIMAP_OFFSET_Y);
        let minimap_rect = Rect::from_min_size(minimap_pos, minimap_size);

        painter.rect_filled(
            minimap_rect,
            UiConstants::NODE_ROUNDING,
            Color32::from_rgba_unmultiplied(30, 30, 30, 200),
        );
        painter.rect_stroke(
            minimap_rect,
            UiConstants::NODE_ROUNDING,
            Stroke::new(1.0, Color32::from_rgb(100, 100, 100)),
            StrokeKind::Outside,
        );

        if scenario.nodes.is_empty() {
            return;
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for node in &scenario.nodes {
            let rect = node.get_rect();
            min_x = min_x.min(rect.min.x);
            min_y = min_y.min(rect.min.y);
            max_x = max_x.max(rect.max.x);
            max_y = max_y.max(rect.max.y);
        }

        min_x -= UiConstants::MINIMAP_WORLD_PADDING;
        min_y -= UiConstants::MINIMAP_WORLD_PADDING;
        max_x += UiConstants::MINIMAP_WORLD_PADDING;
        max_y += UiConstants::MINIMAP_WORLD_PADDING;

        let world_width = max_x - min_x;
        let world_height = max_y - min_y;

        let scale_x = (minimap_size.x - UiConstants::MINIMAP_PADDING * 2.0) / world_width;
        let scale_y = (minimap_size.y - UiConstants::MINIMAP_PADDING * 2.0) / world_height;
        let minimap_scale = scale_x.min(scale_y);

        let to_minimap = |world_pos: Pos2| -> Pos2 {
            let relative_x = (world_pos.x - min_x) * minimap_scale;
            let relative_y = (world_pos.y - min_y) * minimap_scale;
            minimap_rect.min
                + egui::vec2(
                    relative_x + UiConstants::MINIMAP_PADDING,
                    relative_y + UiConstants::MINIMAP_PADDING,
                )
        };

        for node in &scenario.nodes {
            let node_rect = node.get_rect();
            let minimap_min = to_minimap(node_rect.min);
            let minimap_max = to_minimap(node_rect.max);
            let node_minimap_rect = Rect::from_min_max(minimap_min, minimap_max);

            painter.rect_filled(node_minimap_rect, 1.0, node.activity.get_color());
        }

        for connection in &scenario.connections {
            if let (Some(from_node), Some(to_node)) = (
                get_node_from_index(&scenario.nodes, &node_index, &connection.from_node),
                get_node_from_index(&scenario.nodes, &node_index, &connection.to_node),
            ) {
                let from_pos = to_minimap(from_node.get_output_pin_pos());
                let to_pos = to_minimap(to_node.get_input_pin_pos());
                painter.line_segment(
                    [from_pos, to_pos],
                    Stroke::new(1.0, Color32::from_rgb(150, 150, 150)),
                );
            }
        }

        let viewport_world_min = (-pan_offset) / zoom;
        let viewport_world_max = (canvas_rect.size() - pan_offset) / zoom;

        let viewport_minimap_min = to_minimap(viewport_world_min.to_pos2());
        let viewport_minimap_max = to_minimap(viewport_world_max.to_pos2());
        let viewport_minimap_rect = Rect::from_min_max(viewport_minimap_min, viewport_minimap_max);

        let clipped_viewport_rect = viewport_minimap_rect.intersect(minimap_rect);

        painter.rect_stroke(
            clipped_viewport_rect,
            UiConstants::NODE_ROUNDING,
            Stroke::new(
                UiConstants::MINIMAP_NODE_STROKE_WIDTH,
                Color32::from_rgb(100, 150, 255),
            ),
            StrokeKind::Outside,
        );
        painter.rect_filled(
            clipped_viewport_rect,
            UiConstants::NODE_ROUNDING,
            Color32::from_rgba_unmultiplied(100, 150, 255, 30),
        );
    }
}

impl Default for MinimapRenderer {
    fn default() -> Self {
        Self::new()
    }
}
