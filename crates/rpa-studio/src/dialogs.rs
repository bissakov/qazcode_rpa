use rpa_core::node_graph::{VariableDirection, VariableType};
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
    pub is_global: bool,
}

impl Default for AddVariableDialog {
    fn default() -> Self {
        Self {
            show: false,
            name: String::new(),
            value: String::new(),
            var_type: VariableType::String,
            is_global: false,
        }
    }
}

#[derive(Default)]
pub struct RenameScenarioDialog {
    pub scenario_index: Option<usize>,
}

pub struct VariableBindingDialog {
    pub show: bool,
    pub scenario_id: String,
    pub target_var_id: Option<VarId>,
    pub source_var_name: String,
    pub target_var_name: String,
    pub direction: VariableDirection,
    pub editing_index: Option<usize>,
    pub error_message: Option<String>,
}

impl Clone for VariableBindingDialog {
    fn clone(&self) -> Self {
        Self {
            show: self.show,
            scenario_id: self.scenario_id.clone(),
            target_var_id: self.target_var_id,
            source_var_name: self.source_var_name.clone(),
            target_var_name: self.target_var_name.clone(),
            direction: self.direction,
            editing_index: self.editing_index,
            error_message: self.error_message.clone(),
        }
    }
}

impl Default for VariableBindingDialog {
    fn default() -> Self {
        Self {
            show: false,
            scenario_id: String::new(),
            target_var_id: None,
            source_var_name: String::new(),
            target_var_name: String::new(),
            direction: VariableDirection::In,
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
    pub var_binding_dialog: VariableBindingDialog,
    pub debug: DebugDialogs,
    pub selected_log_entry: Option<usize>,
}
