use arc_script::VariableType;
use rpa_core::{NanoId, node_graph::VariableDirection};
use egui_ltreeview::TreeViewState;
use uuid::Uuid;

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
    pub compiled_ir_program: Option<rpa_core::IrProgram>,
    pub compilation_error: Option<String>,
}

#[derive(Clone)]
pub struct UiExplorerDialog {
    pub show: bool,
    pub root_node: Option<WindowNode>,
    pub tree_state: TreeViewState<Uuid>,
    pub selected_element: Option<SelectedElement>,
    pub is_refreshing: bool,
    pub error_message: Option<String>,
}

#[derive(Clone, Debug)]
pub enum WindowNode {
    Window {
        id: Uuid,
        title: String,
        class: String,
        children: Vec<WindowNode>,
    },
    Control {
        id: Uuid,
        class: String,
        text: String,
        parent_window_title: String,
        parent_window_class: String,
    },
}

impl WindowNode {
    pub fn id(&self) -> Uuid {
        match self {
            WindowNode::Window { id, .. } => *id,
            WindowNode::Control { id, .. } => *id,
        }
    }

    pub fn name(&self) -> String {
        match self {
            WindowNode::Window { title, .. } => {
                if title.is_empty() {
                    "[untitled]".to_string()
                } else {
                    title.clone()
                }
            }
            WindowNode::Control { text, class, .. } => {
                format!(
                    "{}{}",
                    if text.is_empty() { "[no text]" } else { text },
                    if class.is_empty() { String::new() } else { format!(" [{}]", class) }
                )
            }
        }
    }

    pub fn children(&self) -> Vec<&WindowNode> {
        match self {
            WindowNode::Window { children, .. } => children.iter().collect(),
            WindowNode::Control { .. } => Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct SelectedElement {
    pub node_id: Uuid,
    pub element_type: ElementType,
    pub window_title: String,
    pub window_class: String,
    pub control_class: Option<String>,
    pub control_text: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub enum ElementType {
    Window,
    Control,
}

impl Default for UiExplorerDialog {
    fn default() -> Self {
        Self {
            show: false,
            root_node: None,
            tree_state: TreeViewState::default(),
            selected_element: None,
            is_refreshing: false,
            error_message: None,
        }
    }
}

#[derive(Default)]
pub struct DialogState {
    pub settings: SettingsDialog,
    pub add_variable: AddVariableDialog,
    pub rename_scenario: RenameScenarioDialog,
    pub var_binding_dialog: VariableBindingDialog,
    pub debug: DebugDialogs,
    pub selected_log_entry: Option<usize>,
    pub ui_explorer: UiExplorerDialog,
}
