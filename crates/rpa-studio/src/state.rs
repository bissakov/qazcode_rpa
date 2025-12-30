use crate::AppSettings;
use crate::dialogs::DialogState;
use crate::ui::canvas::ResizeHandle;
use crate::ui::connection_renderer::ConnectionRenderer;
use crate::undo_redo::UndoRedoManager;
use rpa_core::execution::ExecutionContext;
use rpa_core::{Connection, LogEntry, NanoId, Node, Project, Scenario, StopControl, Variables};
use std::sync::{Arc, RwLock};
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

#[derive(Clone, Default)]
pub struct ClipboardData {
    pub nodes: Vec<Node>,
    pub connections: Vec<Connection>,
}

pub struct RpaApp {
    pub project: Project,
    pub last_frame: Instant,
    pub is_executing: bool,
    pub selected_nodes: HashSet<NanoId>,
    pub current_file: Option<std::path::PathBuf>,
    pub current_scenario_index: Option<usize>,
    pub opened_scenarios: Vec<usize>,
    pub connection_from: Option<(NanoId, usize)>,
    pub settings: AppSettings,
    pub log_receiver: Option<std::sync::mpsc::Receiver<LogEntry>>,
    pub clipboard: ClipboardData,
    pub global_variables: Variables,
    pub knife_tool_active: bool,
    pub knife_path: Vec<egui::Pos2>,
    pub resizing_node: Option<(NanoId, ResizeHandle)>,
    pub stop_control: StopControl,
    pub dialogs: DialogState,
    pub undo_redo: UndoRedoManager,
    #[allow(dead_code)]
    pub property_edit_debounce: f32,
    pub scenario_views: HashMap<NanoId, ScenarioViewState>,
    pub execution_context: Option<Arc<RwLock<ExecutionContext>>>,
}

impl Default for RpaApp {
    fn default() -> Self {
        Self {
            project: Project::new("New Project", Variables::new()),
            last_frame: Instant::now(),
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
            stop_control: StopControl::new(),
            dialogs: DialogState::default(),
            undo_redo: UndoRedoManager::new(),
            property_edit_debounce: 0.0,
            scenario_views: HashMap::new(),
            execution_context: None,
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

    pub fn get_current_scenario_id(&self) -> &NanoId {
        match self.current_scenario_index {
            None => &self.project.main_scenario.id,
            Some(i) => &self.project.scenarios[i].id,
        }
    }

    pub fn open_scenario(&mut self, index: usize) {
        if !self.opened_scenarios.contains(&index) {
            self.opened_scenarios.push(index);
        }
        self.current_scenario_index = Some(index);
        self.selected_nodes.clear();
    }

    pub fn get_current_scenario_view_mut(&mut self) -> &mut ScenarioViewState {
        let scenario_id = self.get_current_scenario_id().clone();
        self.scenario_views.entry(scenario_id).or_default()
    }

    #[allow(dead_code)]
    pub fn get_current_scenario_view(&self) -> Option<&ScenarioViewState> {
        let scenario_id = self.get_current_scenario_id();
        self.scenario_views.get(scenario_id)
    }

    #[allow(dead_code)]
    pub fn get_scenario_view_mut(&mut self, scenario_id: NanoId) -> &mut ScenarioViewState {
        self.scenario_views.entry(scenario_id).or_default()
    }

    #[allow(dead_code)]
    pub fn get_scenario_view(&self, scenario_id: &NanoId) -> Option<&ScenarioViewState> {
        self.scenario_views.get(scenario_id)
    }

    pub fn init_current_scenario_view(&mut self) {
        let scenario_id = self.get_current_scenario_id().clone();
        self.scenario_views
            .insert(scenario_id, ScenarioViewState::default());
    }

    #[allow(dead_code)]
    pub fn remove_current_scenario_view(&mut self) {
        let scenario_id = self.get_current_scenario_id().clone();
        self.scenario_views.remove(&scenario_id);
    }
}

pub struct ScenarioViewState {
    pub pan_offset: egui::Vec2,
    pub zoom: f32,
    pub connection_renderer: ConnectionRenderer,
}

impl Default for ScenarioViewState {
    fn default() -> Self {
        Self {
            pan_offset: egui::Vec2::ZERO,
            zoom: 1.0,
            connection_renderer: ConnectionRenderer::new(),
        }
    }
}

impl ScenarioViewState {
    #[allow(dead_code)]
    pub fn new(pan_offset: egui::Vec2, zoom: f32) -> Self {
        Self {
            pan_offset,
            zoom,
            connection_renderer: ConnectionRenderer::new(),
        }
    }
}
