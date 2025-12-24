use rpa_core::node_graph::VariableType;

use crate::AppSettings;

#[derive(Default)]
pub struct SettingsDialog {
    pub show: bool,
    pub temp_settings: Option<AppSettings>,
}

pub struct AddVariableDialog {
    pub show: bool,
    pub name: String,
    pub value: String,
    pub var_type: VariableType,
}

impl Default for AddVariableDialog {
    fn default() -> Self {
        Self {
            show: false,
            name: String::new(),
            value: String::new(),
            var_type: VariableType::String,
        }
    }
}

#[derive(Default)]
pub struct RenameScenarioDialog {
    pub scenario_index: Option<usize>,
}

#[derive(Default)]
pub struct DebugDialogs {
    pub show_debug: bool,
    pub show_debug_ir: bool,
}

#[derive(Default)]
pub struct DialogState {
    pub settings: SettingsDialog,
    pub add_variable: AddVariableDialog,
    pub rename_scenario: RenameScenarioDialog,
    pub debug: DebugDialogs,
    pub selected_log_entry: Option<usize>,
}
