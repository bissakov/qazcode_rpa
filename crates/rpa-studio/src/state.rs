use crate::AppSettings;
use crate::dialogs::DialogState;
use crate::ui::canvas::ResizeHandle;
use crate::undo_redo::UndoRedoManager;
use rpa_core::{Connection, LogEntry, Node, Project, Scenario, Variables};
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[derive(Clone, Default)]
pub struct ClipboardData {
    pub nodes: Vec<Node>,
    pub connections: Vec<Connection>,
}

pub struct RpaApp {
    pub project: Project,
    pub is_executing: bool,
    pub selected_nodes: HashSet<String>,
    pub current_file: Option<std::path::PathBuf>,
    pub current_scenario_index: Option<usize>,
    pub opened_scenarios: Vec<usize>,
    pub connection_from: Option<(String, usize)>,
    pub settings: AppSettings,
    pub log_receiver: Option<std::sync::mpsc::Receiver<LogEntry>>,
    pub clipboard: ClipboardData,
    pub global_variables: Variables,
    pub knife_tool_active: bool,
    pub knife_path: Vec<egui::Pos2>,
    pub resizing_node: Option<(String, ResizeHandle)>,
    pub stop_flag: Arc<AtomicBool>,
    pub dialogs: DialogState,
    pub undo_redo: UndoRedoManager,
    #[allow(dead_code)]
    pub property_edit_debounce: f32,
}

impl Default for RpaApp {
    fn default() -> Self {
        Self {
            project: Project::new("New Project", Variables::new()),
            is_executing: false,
            selected_nodes: std::collections::HashSet::new(),
            current_file: None,
            current_scenario_index: None,
            opened_scenarios: Vec::new(),
            connection_from: None,
            settings: AppSettings::default(),
            log_receiver: None,
            clipboard: ClipboardData::default(),
            global_variables: Variables::new(),
            knife_tool_active: false,
            knife_path: Vec::new(),
            resizing_node: None,
            stop_flag: Arc::new(AtomicBool::new(false)),
            dialogs: DialogState::default(),
            undo_redo: UndoRedoManager::new(),
            property_edit_debounce: 0.0,
        }
    }
}

impl RpaApp {
    pub fn with_initial_snapshot(mut self) -> Self {
        self.undo_redo.add_undo(&self.project);
        self
    }

    pub fn get_current_scenario(&self) -> &Scenario {
        match self.current_scenario_index {
            None => &self.project.main_scenario,
            Some(i) => &self.project.scenarios[i],
        }
    }

    pub fn get_current_scenario_mut(&mut self) -> &mut Scenario {
        match self.current_scenario_index {
            None => &mut self.project.main_scenario,
            Some(i) => &mut self.project.scenarios[i],
        }
    }

    pub fn open_scenario(&mut self, index: usize) {
        if !self.opened_scenarios.contains(&index) {
            self.opened_scenarios.push(index);
        }
        self.current_scenario_index = Some(index);
        self.selected_nodes.clear();
    }
}
