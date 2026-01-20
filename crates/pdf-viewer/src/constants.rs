use egui::Color32;

pub const MIN_ZOOM: f32 = 0.1;
pub const MAX_ZOOM: f32 = 5.0;
pub const DEFAULT_ZOOM: f32 = 1.0;

pub const ZOOM_BUTTON_FACTOR: f32 = 1.1;
pub const ZOOM_WHEEL_SENSITIVITY: f32 = 0.01;

pub const MIN_VISIBLE_PIXELS: f32 = 50.0;
pub const KEYBOARD_PAN_STEP: f32 = 10.0;

pub const DEFAULT_PAN_OFFSET_X: f32 = 0.0;
pub const DEFAULT_PAN_OFFSET_Y: f32 = 0.0;

pub const WORD_BOUNDARY_COLOR: Color32 = Color32::from_rgb(255, 0, 0);
pub const WORD_BOUNDARY_COLOR_HOVER: Color32 = Color32::from_rgb(0, 128, 255);
pub const WORD_BOUNDARY_STROKE_WIDTH: f32 = 1.5;

pub const ANCHOR_MARKER_RADIUS: f32 = 6.0;
pub const ANCHOR_STROKE_WIDTH: f32 = 2.0;

pub const REGION_POLYGON_STROKE_WIDTH: f32 = 2.0;
pub const REGION_FILL_ALPHA: u8 = 10;

pub const DEFAULT_ANCHOR_OVERLAP_THRESHOLD: f32 = 0.5;
