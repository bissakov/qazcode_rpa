use rpa_core::node_graph::{ParameterDirection, VariableType};
use rpa_core::variables::VarId;

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

pub struct ParameterBindingDialog {
    pub show: bool,
    pub scenario_id: String,
    pub param_var_id: Option<VarId>,
    pub source_var_name: String,
    pub direction: ParameterDirection,
    pub editing_index: Option<usize>,
    pub error_message: Option<String>,
}

impl Clone for ParameterBindingDialog {
    fn clone(&self) -> Self {
        Self {
            show: self.show,
            scenario_id: self.scenario_id.clone(),
            param_var_id: self.param_var_id,
            source_var_name: self.source_var_name.clone(),
            direction: self.direction,
            editing_index: self.editing_index,
            error_message: self.error_message.clone(),
        }
    }
}

impl Default for ParameterBindingDialog {
    fn default() -> Self {
        Self {
            show: false,
            scenario_id: String::new(),
            param_var_id: None,
            source_var_name: String::new(),
            direction: ParameterDirection::In,
            editing_index: None,
            error_message: None,
        }
    }
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
    pub parameter_binding_dialog: ParameterBindingDialog,
    pub debug: DebugDialogs,
    pub selected_log_entry: Option<usize>,
}
