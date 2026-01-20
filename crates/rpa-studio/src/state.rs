use crate::canvas_grid::CanvasObstacleGrid;
use crate::dialogs::DialogState;
use crate::ext::{NodeExt, ProjectExt};
use crate::settings::AppSettings;
use crate::ui::canvas::ResizeHandle;
use crate::ui::connection_renderer::ConnectionRenderer;
use crate::undo_redo::UndoRedoManager;
use rpa_core::execution::ExecutionContext;
use rpa_core::log::LogEntry;
use rpa_core::{Connection, Node, Project, Scenario, StopControl, Variables};
use shared::NanoId;
use std::sync::{Arc, RwLock};
use std::time::Duration;
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
    pub searched_activity: String,
    pub stop_control: StopControl,
    pub dialogs: DialogState,
    pub undo_redo: UndoRedoManager,
    #[allow(dead_code)]
    pub property_edit_debounce: f32,
    pub scenario_views: HashMap<NanoId, ScenarioViewState>,
    pub obstacle_grids: HashMap<NanoId, CanvasObstacleGrid>,
    pub execution_context: Option<Arc<RwLock<ExecutionContext>>>,
    pub pending_node_focus: Option<NanoId>,
    pub last_canvas_rect: Option<egui::Rect>,
    pub last_frame: Instant,
    pub title_timer: Duration,
    pub smoothed_frame_time: Duration,
}

impl Default for RpaApp {
    fn default() -> Self {
        Self {
            project: Project::new_empty("New Project", Variables::new()),
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
            searched_activity: String::new(),
            stop_control: StopControl::new(),
            dialogs: DialogState::default(),
            undo_redo: UndoRedoManager::new(),
            property_edit_debounce: 0.0,
            scenario_views: HashMap::new(),
            obstacle_grids: HashMap::new(),
            execution_context: None,
            pending_node_focus: None,
            last_canvas_rect: None,
            last_frame: Instant::now(),
            title_timer: Duration::ZERO,
            smoothed_frame_time: Duration::from_secs_f64(1.0 / 60.0),
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

    pub fn get_current_scenario_key(&self) -> NanoId {
        self.get_current_scenario_id().clone()
    }

    #[allow(dead_code)]
    pub fn get_obstacle_grid_mut(&mut self, scenario_id: &NanoId) -> &mut CanvasObstacleGrid {
        use crate::ui_constants::UiConstants;
        self.obstacle_grids
            .entry(scenario_id.clone())
            .or_insert_with(|| CanvasObstacleGrid::new(UiConstants::ROUTING_GRID_SIZE))
    }

    #[allow(dead_code)]
    pub fn get_current_obstacle_grid_mut(&mut self) -> &mut CanvasObstacleGrid {
        let scenario_id = self.get_current_scenario_id().clone();
        self.get_obstacle_grid_mut(&scenario_id)
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

    pub fn focus_on_node_from_log(&mut self, node_id: &NanoId) {
        self.pending_node_focus = Some(node_id.clone());
    }

    pub fn handle_pending_node_focus(&mut self) {
        if let Some(node_id) = self.pending_node_focus.take() {
            // Find which scenario contains the node
            let (scenario_index, _) = self
                .find_node_across_scenarios(&node_id)
                .unwrap_or((0, None)); // Default to main scenario if not found

            // Switch to the correct scenario if needed
            let target_index = if scenario_index == 0 {
                None
            } else {
                Some(scenario_index - 1) // Convert to 0-based index for scenarios vector
            };

            // Always clear selection and set current scenario when focusing
            self.current_scenario_index = target_index;
            self.selected_nodes.clear();

            // Select the target node
            self.selected_nodes.insert(node_id.clone());

            // Center view on the node using stored canvas size or fallback
            // Extract node info first to avoid borrow conflicts
            let node_center_opt = self
                .get_current_scenario_mut()
                .get_node(node_id.clone())
                .map(|node| node.get_rect().center());

            if let Some(node_center) = node_center_opt {
                // Extract canvas size before borrowing view
                let canvas_size = self
                    .last_canvas_rect
                    .map(|rect| rect.size())
                    .unwrap_or_else(|| egui::Vec2::new(800.0, 600.0));

                let view = self.get_current_scenario_view_mut();

                // Calculate pan offset to center node in canvas
                let new_pan_offset = egui::Vec2::new(
                    -node_center.x * view.zoom + canvas_size.x / 2.0,
                    -node_center.y * view.zoom + canvas_size.y / 2.0,
                );

                // Debug output (remove after fixing)
                #[cfg(debug_assertions)]
                {
                    println!(
                        "Node centering: node=({}, {}) zoom={} canvas=({}, {}) new_offset=({}, {})",
                        node_center.x,
                        node_center.y,
                        view.zoom,
                        canvas_size.x,
                        canvas_size.y,
                        new_pan_offset.x,
                        new_pan_offset.y
                    );
                }

                view.pan_offset = new_pan_offset;
            }
        }
    }

    fn find_node_across_scenarios(
        &self,
        node_id: &NanoId,
    ) -> Result<(usize, Option<&Node>), String> {
        // Check main scenario
        if let Some(node) = self.project.main_scenario.get_node(node_id.clone()) {
            return Ok((0, Some(node)));
        }

        // Check additional scenarios
        for (index, scenario) in self.project.scenarios.iter().enumerate() {
            if let Some(node) = scenario.get_node(node_id.clone()) {
                return Ok((index + 1, Some(node))); // +1 because main scenario is index 0
            }
        }

        Err(format!("Node {} not found", node_id))
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
