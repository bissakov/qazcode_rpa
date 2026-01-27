pub fn snap_to_grid(value: f32, grid_size: f32) -> f32 {
    (value / grid_size).floor() * grid_size
}

pub fn snap_to_grid_outward(min: f32, max: f32, grid_size: f32) -> (f32, f32) {
    let snapped_min = (min / grid_size).floor() * grid_size;
    let snapped_max = (max / grid_size).ceil() * grid_size;
    (snapped_min, snapped_max)
}

#[allow(dead_code)]
pub fn grid_cells(logical_size: f32, grid_size: f32) -> u32 {
    (logical_size / grid_size).ceil() as u32
}

pub fn enforce_minimum_cells(min: f32, max: f32, grid_size: f32, min_cells: u32) -> (f32, f32) {
    let (snapped_min, snapped_max) = snap_to_grid_outward(min, max, grid_size);
    let size = snapped_max - snapped_min;
    let min_size = (min_cells as f32) * grid_size;

    if size < min_size {
        (snapped_min, snapped_min + min_size)
    } else {
        (snapped_min, snapped_max)
    }
}

pub struct UiConstants;

impl UiConstants {
    pub const GRID_SIZE: f32 = 32.0;

    pub const NODE_ROUNDING: f32 = 5.0;
    pub const NODE_SHADOW_OFFSET: f32 = 2.0;
    pub const NODE_SELECTED_STROKE_WIDTH: f32 = 2.0;

    pub const PIN_RADIUS: f32 = 5.0;
    pub const PIN_INTERACT_SIZE: f32 = 12.0;

    pub const GRID_SPACING: f32 = 32.0;
    pub const GRID_MIN_ZOOM: f32 = 0.1;
    pub const MAX_GRID_LINES: usize = 200;
    pub const MIN_NODE_CELLS: u32 = 2;

    pub const LEFT_PANEL_WIDTH: f32 = 200.0;
    pub const PROPERTIES_PANEL_WIDTH: f32 = 280.0;
    pub const CONSOLE_HEIGHT: f32 = 200.0;

    pub const MINIMAP_WIDTH: f32 = 200.0;
    pub const MINIMAP_HEIGHT: f32 = 150.0;
    pub const MINIMAP_OFFSET_X: f32 = 20.0;
    pub const MINIMAP_OFFSET_Y: f32 = 20.0;
    pub const MINIMAP_PADDING: f32 = 10.0;
    pub const MINIMAP_WORLD_PADDING: f32 = 20.0;
    pub const MINIMAP_NODE_STROKE_WIDTH: f32 = 2.0;

    pub const LINK_INSERT_THRESHOLD: f32 = 15.0;
    pub const MIN_NODE_SPACING: f32 = 100.0;

    pub const CONNECTION_ALIGNMENT_THRESHOLD: f32 = 5.0;
    pub const CONNECTION_PIN_EXIT_OFFSET: f32 = 64.0;
    pub const CONNECTION_CORNER_RADIUS: f32 = 12.0;

    pub const ZOOM_MIN: f32 = 0.1;
    pub const ZOOM_MAX: f32 = 3.0;
    pub const ZOOM_DELTA_MULTIPLIER: f32 = 0.001;

    pub const DEFAULT_FONT_SIZE: f32 = 18.0;
    pub const SMALL_FONT_MULTIPLIER: f32 = 0.85;

    pub const NEW_NODE_OFFSET_INCREMENT: f32 = 20.0;

    pub const NOTE_MIN_WIDTH: f32 = 100.0;
    pub const NOTE_MIN_HEIGHT: f32 = 60.0;
    pub const NOTE_PADDING: f32 = 10.0;
    pub const NOTE_RESIZE_HANDLE_SIZE: f32 = 10.0;
    pub const NOTE_FONT_SIZE: f32 = 12.0;

    pub const NODE_LABEL_FONT_SIZE: f32 = 14.0;
    pub const PIN_LABEL_FONT_SIZE: f32 = 10.0;
    pub const PIN_LABEL_OFFSET: f32 = 12.0;

    #[allow(dead_code)]
    pub const CANVAS_WORLD_PADDING: f32 = 500.0;
    pub const CANVAS_BORDER_STROKE_WIDTH: f32 = 2.0;

    pub const TABLE_HEADER_HEIGHT: f32 = 20.0;
    pub const PANEL_SECTION_SPACING: f32 = 20.0;

    pub const CONTEXT_MENU_MIN_WIDTH: f32 = 100.0;
    pub const NODE_CONTEXT_MENU_MIN_WIDTH: f32 = 150.0;

    pub const UNDO_HISTORY_LIMIT: usize = 100;
    pub const PROPERTY_EDIT_DEBOUNCE_MS: f32 = 500.0;

    pub const DEFAULT_FPS: usize = 180;
}
