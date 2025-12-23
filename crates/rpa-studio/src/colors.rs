use egui::Color32;
use rpa_core::{Activity, ActivityMetadata, ColorCategory};

pub struct ColorPalette;

impl ColorPalette {
    pub const FLOW_CONTROL_START: Color32 = Color32::from_rgb(60, 170, 100);
    pub const FLOW_CONTROL_END: Color32 = Color32::from_rgb(200, 80, 80);

    pub const BASIC_OPS: Color32 = Color32::from_rgb(80, 120, 180);
    pub const VARIABLES: Color32 = Color32::from_rgb(70, 140, 200);
    pub const CONTROL_FLOW: Color32 = Color32::from_rgb(220, 140, 60);
    pub const EXECUTION: Color32 = Color32::from_rgb(140, 100, 180);
    pub const NOTE: Color32 = Color32::from_rgb(255, 255, 200);

    pub const CONNECTION_TRUE: Color32 = Color32::from_rgb(50, 220, 100);
    pub const CONNECTION_FALSE: Color32 = Color32::from_rgb(220, 80, 80);
    pub const CONNECTION_LOOP_BODY: Color32 = Color32::from_rgb(255, 180, 50);
    pub const CONNECTION_ERROR: Color32 = Color32::from_rgb(255, 50, 50);
    pub const CONNECTION_DEFAULT: Color32 = Color32::from_rgb(160, 160, 160);

    pub const PIN_SUCCESS: Color32 = Color32::from_rgb(100, 200, 100);
    pub const PIN_ERROR: Color32 = Color32::from_rgb(200, 100, 100);
    pub const PIN_TRUE: Color32 = Color32::from_rgb(100, 200, 100);
    pub const PIN_FALSE: Color32 = Color32::from_rgb(200, 100, 100);
    pub const PIN_LOOP_BODY: Color32 = Color32::from_rgb(255, 165, 0);
    pub const PIN_LOOP_NEXT: Color32 = Color32::from_rgb(150, 150, 150);
    pub const PIN_DEFAULT: Color32 = Color32::from_rgb(150, 150, 150);

    pub fn for_activity(activity: &Activity) -> Color32 {
        Self::for_color_category(&ActivityMetadata::for_activity(activity).color_category)
    }

    pub fn for_color_category(category: &ColorCategory) -> Color32 {
        match category {
            ColorCategory::FlowControlStart => Self::FLOW_CONTROL_START,
            ColorCategory::FlowControlEnd => Self::FLOW_CONTROL_END,
            ColorCategory::BasicOps => Self::BASIC_OPS,
            ColorCategory::Variables => Self::VARIABLES,
            ColorCategory::ControlFlow => Self::CONTROL_FLOW,
            ColorCategory::Execution => Self::EXECUTION,
            ColorCategory::Note => Self::NOTE,
        }
    }
}
