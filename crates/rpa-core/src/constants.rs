#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowDirection {
    Horizontal,
    Vertical,
}

pub struct UiConstants;

impl UiConstants {
    pub const FLOW_DIRECTION: FlowDirection = FlowDirection::Vertical;
    pub const NODE_WIDTH: f32 = 180.0;
    pub const NODE_HEIGHT: f32 = 60.0;
    pub const NODE_ROUNDING: f32 = 5.0;
    pub const NODE_SHADOW_OFFSET: f32 = 2.0;

    pub const PIN_RADIUS: f32 = 5.0;
    pub const PIN_INTERACT_SIZE: f32 = 12.0;

    pub const GRID_SPACING: f32 = 20.0;

    pub const LEFT_PANEL_WIDTH: f32 = 200.0;
    pub const PROPERTIES_PANEL_WIDTH: f32 = 280.0;
    pub const CONSOLE_HEIGHT: f32 = 200.0;

    pub const MINIMAP_WIDTH: f32 = 200.0;
    pub const MINIMAP_HEIGHT: f32 = 150.0;
    pub const MINIMAP_OFFSET_X: f32 = 20.0;
    pub const MINIMAP_OFFSET_Y: f32 = 20.0;
    pub const MINIMAP_PADDING: f32 = 10.0;
    pub const MINIMAP_WORLD_PADDING: f32 = 20.0;

    pub const BEZIER_STEPS: usize = 50;
    pub const BEZIER_CONTROL_OFFSET: f32 = 50.0;
    pub const LINK_INSERT_THRESHOLD: f32 = 15.0;
    pub const MIN_NODE_SPACING: f32 = 250.0;

    pub const ZOOM_MIN: f32 = 0.1;
    pub const ZOOM_MAX: f32 = 3.0;
    pub const ZOOM_DELTA_MULTIPLIER: f32 = 0.001;

    pub const DEFAULT_FONT_SIZE: f32 = 18.0;
    pub const SMALL_FONT_MULTIPLIER: f32 = 0.85;

    pub const DELAY_MAX_MS: u64 = 10000;
    pub const LOOP_MAX_ITERATIONS: usize = 100;
    pub const LOOP_ITERATIONS_MIN: usize = 0;
    pub const LOOP_ITERATIONS_MAX: usize = 10000;

    pub const NEW_NODE_OFFSET_INCREMENT: f32 = 20.0;

    pub const EXECUTION_COMPLETE_MARKER: &'static str = "__EXECUTION_COMPLETE__";

    pub const VARIABLE_PLACEHOLDER_OPEN: char = '{';
    pub const VARIABLE_PLACEHOLDER_CLOSE: char = '}';

    pub const NOTE_MIN_WIDTH: f32 = 100.0;
    pub const NOTE_MIN_HEIGHT: f32 = 60.0;
    pub const NOTE_PADDING: f32 = 10.0;
    pub const NOTE_RESIZE_HANDLE_SIZE: f32 = 10.0;

    pub const DEFAULT_LOG_ENTRIES: usize = 100;
    pub const MAX_LOG_ENTRIES: usize = 10_000;

    pub const NANOID_LENGTH: usize = 10;
}

pub struct ActivityCategories;

impl ActivityCategories {
    pub const SCENARIOS: &'static str = "activity_groups.scenarios";
    pub const BASIC: &'static str = "activity_groups.basic";
    pub const CONTROL_FLOW: &'static str = "activity_groups.control_flow";
    pub const SCRIPTING: &'static str = "activity_groups.scripting";
    pub const DOCUMENTATION: &'static str = "activity_groups.documentation";
}

pub struct ActivityDefaults;

impl ActivityDefaults {
    pub const LOG_MESSAGE: &'static str = "default_values.log_message";
    pub const DELAY_MS: u64 = 1000;
    pub const VARIABLE_NAME: &'static str = "default_values.variable_name";
    pub const VARIABLE_VALUE: &'static str = "default_values.variable_value";
    pub const CONDITION_EXAMPLE: &'static str = "default_values.condition_example";
    pub const LOOP_START: i64 = 0;
    pub const LOOP_END: i64 = 10;
    pub const LOOP_STEP: i64 = 1;
    pub const LOOP_INDEX: &'static str = "i";
    pub const POWERSHELL_CODE: &'static str = "";
    pub const NOTE_TEXT: &'static str = "default_values.note_text";
}
