use crate::Activity;
use crate::constants::ActivityDefaults;
use crate::log::LogLevel;
use arc_script::VariableType;
use shared::NanoId;
use std::sync::LazyLock;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityCategory {
    BasicActivities,
    ControlFlow,
    Scenarios,
    Scripting,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorCategory {
    FlowControlStart,
    FlowControlEnd,
    BasicOps,
    Variables,
    ControlFlow,
    Execution,
    Note,
}

#[non_exhaustive]
pub struct PinConfig {
    pub output_count: usize,
    pub pin_labels: &'static [&'static str],
}

const PIN_NONE: PinConfig = PinConfig {
    output_count: 0,
    pin_labels: &[],
};

const PIN_DEFAULT: PinConfig = PinConfig {
    output_count: 1,
    pin_labels: &["Default"],
};

const PIN_SUCCESS_ERROR: PinConfig = PinConfig {
    output_count: 2,
    pin_labels: &["Success", "Error"],
};

const PIN_BODY_NEXT: PinConfig = PinConfig {
    output_count: 2,
    pin_labels: &["Body", "Next"],
};

const PIN_TRUE_FALSE: PinConfig = PinConfig {
    output_count: 2,
    pin_labels: &["True", "False"],
};

const PIN_TRY_CATCH: PinConfig = PinConfig {
    output_count: 2,
    pin_labels: &["Try", "Catch"],
};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyType {
    Description,
    TextSingleLine,
    TextMultiLine,
    Slider,
    DragInt,
    ScenarioSelector,
    CodeEditor,
    Combobox,
}

#[non_exhaustive]
pub struct PropertyDef {
    pub label_key: &'static str,
    pub tooltip_key: Option<&'static str>,
    pub property_type: PropertyType,
}

#[non_exhaustive]
pub struct ActivityMetadata {
    pub name_key: &'static str,
    pub button_key: &'static str,
    pub category: ActivityCategory,
    pub color_category: ColorCategory,
    pub pin_config: PinConfig,
    pub can_have_error_output: bool,
    pub properties: &'static [PropertyDef],
}

static ACTIVITY_DEFINITIONS: LazyLock<Vec<(&'static ActivityMetadata, Activity)>> =
    LazyLock::new(|| {
        vec![
            (
                &START_METADATA,
                Activity::Start {
                    scenario_id: NanoId::default(),
                },
            ),
            (
                &END_METADATA,
                Activity::End {
                    scenario_id: NanoId::default(),
                },
            ),
            (
                &LOG_METADATA,
                Activity::Log {
                    level: LogLevel::Info,
                    message: String::new(),
                },
            ),
            (
                &DELAY_METADATA,
                Activity::Delay {
                    milliseconds: ActivityDefaults::DELAY_MS,
                },
            ),
            (
                &SET_VARIABLE_METADATA,
                Activity::SetVariable {
                    name: String::new(),
                    value: String::new(),
                    var_type: VariableType::String,
                    is_global: false,
                },
            ),
            (
                &EVALUATE_METADATA,
                Activity::Evaluate {
                    expression: String::new(),
                },
            ),
            (
                &IF_CONDITION_METADATA,
                Activity::IfCondition {
                    condition: String::new(),
                },
            ),
            (
                &LOOP_METADATA,
                Activity::Loop {
                    start: ActivityDefaults::LOOP_START,
                    end: ActivityDefaults::LOOP_END,
                    step: ActivityDefaults::LOOP_STEP,
                    index: String::from(ActivityDefaults::LOOP_INDEX),
                },
            ),
            (
                &WHILE_METADATA,
                Activity::While {
                    condition: String::new(),
                },
            ),
            (&CONTINUE_METADATA, Activity::Continue),
            (&BREAK_METADATA, Activity::Break),
            (&TRY_CATCH_METADATA, Activity::TryCatch),
            (
                &CALL_SCENARIO_METADATA,
                Activity::CallScenario {
                    scenario_id: NanoId::default(),
                    parameters: Vec::new(),
                },
            ),
            (
                &RUN_POWERSHELL_METADATA,
                Activity::RunPowershell {
                    code: String::new(),
                },
            ),
            (
                &NOTE_METADATA,
                Activity::Note {
                    text: String::new(),
                    width: 128.0,
                    height: 64.0,
                },
            ),
        ]
    });

impl ActivityMetadata {
    pub fn for_activity(activity: &Activity) -> &'static ActivityMetadata {
        match activity {
            Activity::Start { .. } => &START_METADATA,
            Activity::End { .. } => &END_METADATA,
            Activity::Log { .. } => &LOG_METADATA,
            Activity::Delay { .. } => &DELAY_METADATA,
            Activity::SetVariable { .. } => &SET_VARIABLE_METADATA,
            Activity::Evaluate { .. } => &EVALUATE_METADATA,
            Activity::IfCondition { .. } => &IF_CONDITION_METADATA,
            Activity::Loop { .. } => &LOOP_METADATA,
            Activity::While { .. } => &WHILE_METADATA,
            Activity::Continue => &CONTINUE_METADATA,
            Activity::Break => &BREAK_METADATA,
            Activity::CallScenario { .. } => &CALL_SCENARIO_METADATA,
            Activity::RunPowershell { .. } => &RUN_POWERSHELL_METADATA,
            Activity::Note { .. } => &NOTE_METADATA,
            Activity::TryCatch => &TRY_CATCH_METADATA,
        }
    }

    pub fn all_activities() -> Vec<(&'static ActivityMetadata, Activity)> {
        ACTIVITY_DEFINITIONS.clone()
    }

    #[allow(clippy::type_complexity)]
    pub fn activities_by_category() -> Vec<(
        ActivityCategory,
        Vec<(&'static ActivityMetadata, Activity)>,
        bool,
    )> {
        let all = &*ACTIVITY_DEFINITIONS;

        vec![
            (
                ActivityCategory::Scenarios,
                all.iter()
                    .filter(|(m, _)| m.category == ActivityCategory::Scenarios)
                    .cloned()
                    .collect(),
                true,
            ),
            (
                ActivityCategory::BasicActivities,
                all.iter()
                    .filter(|(m, _)| m.category == ActivityCategory::BasicActivities)
                    .cloned()
                    .collect(),
                false,
            ),
            (
                ActivityCategory::ControlFlow,
                all.iter()
                    .filter(|(m, _)| m.category == ActivityCategory::ControlFlow)
                    .cloned()
                    .collect(),
                false,
            ),
            (
                ActivityCategory::Scripting,
                all.iter()
                    .filter(|(m, _)| m.category == ActivityCategory::Scripting)
                    .cloned()
                    .collect(),
                false,
            ),
        ]
    }
}

impl ActivityCategory {
    pub fn translation_key(self) -> &'static str {
        match self {
            Self::Scenarios => "activity_groups.scenarios",
            Self::BasicActivities => "activity_groups.basic",
            Self::ControlFlow => "activity_groups.control_flow",
            Self::Scripting => "activity_groups.scripting",
        }
    }
}

static START_METADATA: ActivityMetadata = ActivityMetadata {
    name_key: "activity_names.start",
    button_key: "activity_buttons.start",
    category: ActivityCategory::Scenarios,
    color_category: ColorCategory::FlowControlStart,
    pin_config: PIN_DEFAULT,
    can_have_error_output: false,
    properties: &[PropertyDef {
        label_key: "activity_descriptions.start",
        tooltip_key: None,
        property_type: PropertyType::Description,
    }],
};

static END_METADATA: ActivityMetadata = ActivityMetadata {
    name_key: "activity_names.end",
    button_key: "activity_buttons.end",
    category: ActivityCategory::Scenarios,
    color_category: ColorCategory::FlowControlEnd,
    pin_config: PIN_NONE,
    can_have_error_output: false,
    properties: &[PropertyDef {
        label_key: "activity_descriptions.end",
        tooltip_key: None,
        property_type: PropertyType::Description,
    }],
};

static CALL_SCENARIO_METADATA: ActivityMetadata = ActivityMetadata {
    name_key: "activity_names.call_scenario",
    button_key: "activity_buttons.call_scenario",
    category: ActivityCategory::Scenarios,
    color_category: ColorCategory::Execution,
    pin_config: PIN_SUCCESS_ERROR,
    can_have_error_output: true,
    properties: &[PropertyDef {
        label_key: "properties.scenario",
        tooltip_key: None,
        property_type: PropertyType::ScenarioSelector,
    }],
};

static LOG_METADATA: ActivityMetadata = ActivityMetadata {
    name_key: "activity_names.log",
    button_key: "activity_buttons.log",
    category: ActivityCategory::BasicActivities,
    color_category: ColorCategory::BasicOps,
    pin_config: PIN_DEFAULT,
    can_have_error_output: false,
    properties: &[PropertyDef {
        label_key: "properties.message",
        tooltip_key: Some("tooltips.message_help"),
        property_type: PropertyType::TextMultiLine,
    }],
};

static DELAY_METADATA: ActivityMetadata = ActivityMetadata {
    name_key: "activity_names.delay",
    button_key: "activity_buttons.delay",
    category: ActivityCategory::BasicActivities,
    color_category: ColorCategory::BasicOps,
    pin_config: PIN_DEFAULT,
    can_have_error_output: false,
    properties: &[PropertyDef {
        label_key: "properties.delay_ms",
        tooltip_key: None,
        property_type: PropertyType::DragInt,
    }],
};

static SET_VARIABLE_METADATA: ActivityMetadata = ActivityMetadata {
    name_key: "activity_names.set_variable",
    button_key: "activity_buttons.set_variable",
    category: ActivityCategory::BasicActivities,
    color_category: ColorCategory::Variables,
    pin_config: PIN_DEFAULT,
    can_have_error_output: false,
    properties: &[
        PropertyDef {
            label_key: "properties.variable_name",
            tooltip_key: Some("tooltips.set_variable_help"),
            property_type: PropertyType::TextSingleLine,
        },
        PropertyDef {
            label_key: "properties.variable_type",
            tooltip_key: Some("tooltips.variable_type_help"),
            property_type: PropertyType::TextSingleLine,
        },
        PropertyDef {
            label_key: "properties.value",
            tooltip_key: Some("tooltips.value_help"),
            property_type: PropertyType::TextSingleLine,
        },
        PropertyDef {
            label_key: "properties.scope",
            tooltip_key: Some("tooltips.scope_help"),
            property_type: PropertyType::Combobox,
        },
    ],
};

static EVALUATE_METADATA: ActivityMetadata = ActivityMetadata {
    name_key: "activity_names.evaluate",
    button_key: "activity_buttons.evaluate",
    category: ActivityCategory::BasicActivities,
    color_category: ColorCategory::Variables,
    pin_config: PIN_SUCCESS_ERROR,
    can_have_error_output: true,
    properties: &[PropertyDef {
        label_key: "properties.evaluate_expression",
        tooltip_key: Some("tooltips.get_variable_help"),
        property_type: PropertyType::TextSingleLine,
    }],
};

static NOTE_METADATA: ActivityMetadata = ActivityMetadata {
    name_key: "activity_names.note",
    button_key: "activity_buttons.note",
    category: ActivityCategory::BasicActivities,
    color_category: ColorCategory::Note,
    pin_config: PIN_NONE,
    can_have_error_output: false,
    properties: &[
        PropertyDef {
            label_key: "properties.note_text",
            tooltip_key: None,
            property_type: PropertyType::TextMultiLine,
        },
        PropertyDef {
            label_key: "tooltips.note_resize",
            tooltip_key: None,
            property_type: PropertyType::Description,
        },
    ],
};

static IF_CONDITION_METADATA: ActivityMetadata = ActivityMetadata {
    name_key: "activity_names.if_condition",
    button_key: "activity_buttons.if_condition",
    category: ActivityCategory::ControlFlow,
    color_category: ColorCategory::ControlFlow,
    pin_config: PIN_TRUE_FALSE,
    can_have_error_output: false,
    properties: &[PropertyDef {
        label_key: "properties.condition",
        tooltip_key: Some("tooltips.condition_help"),
        property_type: PropertyType::TextSingleLine,
    }],
};

static LOOP_METADATA: ActivityMetadata = ActivityMetadata {
    name_key: "activity_names.loop",
    button_key: "activity_buttons.loop",
    category: ActivityCategory::ControlFlow,
    color_category: ColorCategory::ControlFlow,
    pin_config: PIN_BODY_NEXT,
    can_have_error_output: false,
    properties: &[
        PropertyDef {
            label_key: "properties.loop_index",
            tooltip_key: Some("tooltips.loop_index_help"),
            property_type: PropertyType::TextSingleLine,
        },
        PropertyDef {
            label_key: "properties.loop_start",
            tooltip_key: None,
            property_type: PropertyType::DragInt,
        },
        PropertyDef {
            label_key: "properties.loop_end",
            tooltip_key: None,
            property_type: PropertyType::DragInt,
        },
        PropertyDef {
            label_key: "properties.loop_step",
            tooltip_key: None,
            property_type: PropertyType::DragInt,
        },
    ],
};

static WHILE_METADATA: ActivityMetadata = ActivityMetadata {
    name_key: "activity_names.while",
    button_key: "activity_buttons.while",
    category: ActivityCategory::ControlFlow,
    color_category: ColorCategory::ControlFlow,
    pin_config: PIN_BODY_NEXT,
    can_have_error_output: false,
    properties: &[PropertyDef {
        label_key: "properties.condition",
        tooltip_key: Some("tooltips.condition_help"),
        property_type: PropertyType::TextSingleLine,
    }],
};

static CONTINUE_METADATA: ActivityMetadata = ActivityMetadata {
    name_key: "activity_names.continue",
    button_key: "activity_buttons.continue",
    category: ActivityCategory::ControlFlow,
    color_category: ColorCategory::ControlFlow,
    pin_config: PIN_NONE,
    can_have_error_output: false,
    properties: &[PropertyDef {
        label_key: "activity_descriptions.continue",
        tooltip_key: None,
        property_type: PropertyType::Description,
    }],
};

static BREAK_METADATA: ActivityMetadata = ActivityMetadata {
    name_key: "activity_names.break",
    button_key: "activity_buttons.break",
    category: ActivityCategory::ControlFlow,
    color_category: ColorCategory::ControlFlow,
    pin_config: PIN_NONE,
    can_have_error_output: false,
    properties: &[PropertyDef {
        label_key: "activity_descriptions.break",
        tooltip_key: None,
        property_type: PropertyType::Description,
    }],
};

static TRY_CATCH_METADATA: ActivityMetadata = ActivityMetadata {
    name_key: "activity_names.try_catch",
    button_key: "activity_buttons.try_catch",
    category: ActivityCategory::ControlFlow,
    color_category: ColorCategory::ControlFlow,
    pin_config: PIN_TRY_CATCH,
    can_have_error_output: false,
    properties: &[PropertyDef {
        label_key: "properties.try_catch_info",
        tooltip_key: None,
        property_type: PropertyType::Description,
    }],
};

static RUN_POWERSHELL_METADATA: ActivityMetadata = ActivityMetadata {
    name_key: "activity_names.run_powershell",
    button_key: "activity_buttons.run_powershell",
    category: ActivityCategory::Scripting,
    color_category: ColorCategory::Execution,
    pin_config: PIN_SUCCESS_ERROR,
    can_have_error_output: true,
    properties: &[PropertyDef {
        label_key: "properties.run_powershell",
        tooltip_key: None,
        property_type: PropertyType::CodeEditor,
    }],
};
