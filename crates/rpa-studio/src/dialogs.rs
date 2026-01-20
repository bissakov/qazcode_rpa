use arc_script::VariableType;
use rpa_core::IrProgram;
use rpa_core::node_graph::VariableDirection;
use shared::NanoId;
use ui_explorer::state::UiExplorerState;

use crate::settings::SettingsDialog;

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
    pub scenario_id: NanoId,
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
            scenario_id: NanoId::new(""),
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
    pub show_inspection_ui: bool,
    pub show_grid_debug: bool,
    pub show_ir_view: bool,
    pub compiled_ir_program: Option<IrProgram>,
    pub compilation_error: Option<String>,
}

#[derive(Default)]
pub struct DialogState {
    pub settings: SettingsDialog,
    pub add_variable: AddVariableDialog,
    pub rename_scenario: RenameScenarioDialog,
    pub var_binding_dialog: VariableBindingDialog,
    pub debug: DebugDialogs,
    pub selected_log_entry: Option<usize>,
    pub ui_explorer: UiExplorerState,
}
