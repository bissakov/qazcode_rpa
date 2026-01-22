use crate::ui_constants::UiConstants;
use egui::{Color32, Vec2};

#[derive(Clone)]
pub struct CanvasConfig {
    pub show_grid: bool,
    pub show_minimap: bool,
    pub is_executing: bool,
    pub allow_node_resize: bool,
}

impl Default for CanvasConfig {
    fn default() -> Self {
        Self {
            show_grid: true,
            show_minimap: false,
            is_executing: false,
            allow_node_resize: false,
        }
    }
}

#[derive(Clone)]
pub struct NodeStyle {
    pub rounding: f32,
    pub shadow_offset: Vec2,
    pub shadow_color: Color32,
    pub selected_stroke_width: f32,
    pub selected_stroke_color: Color32,
    pub hover_brightness_boost: u8,
    pub resizing_stroke_width: f32,
    pub resizing_stroke_color: Color32,
}

impl Default for NodeStyle {
    fn default() -> Self {
        Self {
            rounding: UiConstants::NODE_ROUNDING,
            shadow_offset: Vec2::splat(UiConstants::NODE_SHADOW_OFFSET),
            shadow_color: Color32::from_black_alpha(100),
            selected_stroke_width: UiConstants::NODE_SELECTED_STROKE_WIDTH,
            selected_stroke_color: Color32::from_rgb(255, 255, 0),
            hover_brightness_boost: 30,
            resizing_stroke_width: 4.0,
            resizing_stroke_color: Color32::from_rgb(100, 150, 255),
        }
    }
}

#[derive(Clone)]
pub struct PinStyle {
    pub radius: f32,
    pub label_offset: f32,
    pub label_font_size: f32,
}

impl Default for PinStyle {
    fn default() -> Self {
        Self {
            radius: UiConstants::PIN_RADIUS,
            label_offset: UiConstants::PIN_LABEL_OFFSET,
            label_font_size: UiConstants::PIN_LABEL_FONT_SIZE,
        }
    }
}

#[derive(Clone)]
pub struct GridStyle {
    pub spacing: f32,
    pub color: Color32,
    pub min_zoom: f32,
    pub max_lines: usize,
}

impl Default for GridStyle {
    fn default() -> Self {
        Self {
            spacing: UiConstants::GRID_SPACING,
            color: Color32::from_rgb(50, 50, 50),
            min_zoom: UiConstants::GRID_MIN_ZOOM,
            max_lines: UiConstants::MAX_GRID_LINES,
        }
    }
}
