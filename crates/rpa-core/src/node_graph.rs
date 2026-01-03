use crate::canvas_grid::CanvasObstacleGrid;
use crate::constants::{ALPHABET, OutputDirection, UiConstants, enforce_minimum_cells};
use crate::log::LogLevel;
use crate::log::LogStorage;
use crate::variables::{VariableScope, Variables};
use arc_script::VariableType;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub main_scenario: Scenario,
    pub scenarios: Vec<Scenario>,
    #[serde(skip)]
    pub execution_log: LogStorage,
    pub variables: Variables,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    pub project: Project,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiState {
    pub current_scenario_index: Option<usize>,
    pub font_size: f32,
    pub show_minimap: bool,
    #[serde(default = "default_allow_node_resize")]
    pub allow_node_resize: bool,
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_language() -> String {
    "en".to_string()
}

fn default_allow_node_resize() -> bool {
    true
}

impl UiState {
    pub fn is_unlimited(value: usize) -> bool {
        value == 0
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            current_scenario_index: None,
            font_size: UiConstants::DEFAULT_FONT_SIZE,
            show_minimap: true,
            allow_node_resize: true,
            language: "en".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scenario {
    pub id: NanoId,
    pub name: String,
    pub nodes: Vec<Node>,
    pub connections: Vec<Connection>,
    #[serde(default)]
    pub parameters: Vec<ScenarioParameter>,
    #[serde(default)]
    pub variables: Variables,
    #[serde(skip)]
    pub obstacle_grid: CanvasObstacleGrid,
}

impl Project {
    pub fn new(name: &str, variables: Variables) -> Self {
        Self {
            name: name.to_string(),
            main_scenario: Scenario::new("Main"),
            scenarios: Vec::new(),
            execution_log: LogStorage::new(),
            variables,
        }
    }
}

impl Scenario {
    pub fn new(name: &str) -> Self {
        let mut scenario = Self {
            id: NanoId::new_with_nanoid(),
            name: name.to_string(),
            nodes: Vec::new(),
            connections: Vec::new(),
            parameters: Vec::new(),
            variables: Variables::new(),
            obstacle_grid: CanvasObstacleGrid::new(UiConstants::ROUTING_GRID_SIZE),
        };

        let grid_size = UiConstants::GRID_CELL_SIZE;
        let start_x = (1000.0 / grid_size).floor() * grid_size;
        let start_y = (550.0 / grid_size).floor() * grid_size;

        scenario.add_node(
            Activity::Start {
                scenario_id: scenario.id.clone(),
            },
            egui::pos2(start_x, start_y),
        );
        scenario.add_node(
            Activity::End {
                scenario_id: scenario.id.clone(),
            },
            egui::pos2(
                start_x,
                start_y + UiConstants::NODE_HEIGHT + (64.0 / grid_size).floor() * grid_size,
            ),
        );

        if scenario.nodes.len() >= 2 {
            let start_id = scenario.nodes[0].id.clone();
            let end_id = scenario.nodes[1].id.clone();
            scenario.add_connection_with_branch(start_id, end_id, BranchType::Default);
        }

        scenario
    }

    pub fn add_node(&mut self, activity: Activity, position: egui::Pos2) {
        let (width, height) = match &activity {
            Activity::Note { width, height, .. } => (*width, *height),
            _ => (UiConstants::NODE_WIDTH, UiConstants::NODE_HEIGHT),
        };
        let node = Node {
            id: NanoId::new_with_nanoid(),
            activity,
            position,
            width,
            height,
        };
        self.nodes.push(node);
    }

    pub fn get_node_mut(&mut self, id: NanoId) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn get_node(&self, id: NanoId) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn remove_node(&mut self, id: NanoId) {
        self.nodes.retain(|n| n.id != id);
        self.connections
            .retain(|c| c.from_node != id && c.to_node != id);
    }

    pub fn add_connection_with_branch(
        &mut self,
        from: NanoId,
        to: NanoId,
        branch_type: BranchType,
    ) {
        if self
            .connections
            .iter()
            .any(|c| c.from_node == from && c.to_node == to && c.branch_type == branch_type)
        {
            return;
        }

        self.connections
            .push(Connection::new_with_nanoid(from, to, branch_type));
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct NanoId(Arc<str>);

impl Deref for NanoId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for NanoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for NanoId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for NanoId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(NanoId(Arc::from(s)))
    }
}

impl NanoId {
    pub fn new<S>(s: S) -> Self
    where
        S: AsRef<str>,
    {
        NanoId(Arc::from(s.as_ref()))
    }

    pub fn new_with_nanoid() -> Self {
        NanoId(Arc::from(nanoid!(8, &ALPHABET)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NanoId,
    pub activity: Activity,
    #[serde(with = "pos2_serde")]
    pub position: egui::Pos2,
    #[serde(default = "default_node_width")]
    pub width: f32,
    #[serde(default = "default_node_height")]
    pub height: f32,
}

fn default_node_width() -> f32 {
    UiConstants::NODE_WIDTH
}

fn default_node_height() -> f32 {
    UiConstants::NODE_HEIGHT
}

impl Node {
    pub fn get_rect(&self) -> egui::Rect {
        egui::Rect::from_min_size(self.position, egui::vec2(self.width, self.height))
    }

    pub fn is_routable(&self) -> bool {
        !matches!(self.activity, Activity::Note { .. })
    }

    pub fn snap_bounds(&self, grid_size: f32) -> (egui::Pos2, f32, f32) {
        if !self.is_routable() {
            return (self.position, self.width, self.height);
        }

        let right = self.position.x + self.width;
        let bottom = self.position.y + self.height;

        let (snapped_left, snapped_right) = enforce_minimum_cells(
            self.position.x,
            right,
            grid_size,
            UiConstants::MIN_NODE_CELLS,
        );
        let (snapped_top, snapped_bottom) = enforce_minimum_cells(
            self.position.y,
            bottom,
            grid_size,
            UiConstants::MIN_NODE_CELLS,
        );

        let snapped_pos = egui::Pos2::new(snapped_left, snapped_top);
        let snapped_width = snapped_right - snapped_left;
        let snapped_height = snapped_bottom - snapped_top;

        (snapped_pos, snapped_width, snapped_height)
    }

    pub fn get_routing_footprint(&self, grid_size: f32) -> egui::Rect {
        let (pos, w, h) = self.snap_bounds(grid_size);
        egui::Rect::from_min_size(pos, egui::vec2(w, h))
    }

    pub fn get_visual_bounds(&self) -> egui::Rect {
        self.get_rect()
    }

    pub fn get_input_pin_pos(&self) -> egui::Pos2 {
        self.position + egui::vec2(self.width / 2.0, 0.0)
    }

    pub fn get_output_pin_pos(&self) -> egui::Pos2 {
        self.position + egui::vec2(self.width / 2.0, self.height)
    }

    pub fn has_input_pin(&self) -> bool {
        !matches!(
            self.activity,
            Activity::Start { .. } | Activity::Note { .. }
        )
    }

    pub fn has_output_pin(&self) -> bool {
        !matches!(
            self.activity,
            Activity::End { .. } | Activity::Note { .. } | Activity::Continue | Activity::Break
        )
    }

    pub fn get_output_pin_count(&self) -> usize {
        match &self.activity {
            Activity::IfCondition { .. } => 2,
            Activity::Loop { .. } => 2,
            Activity::While { .. } => 2,
            Activity::TryCatch => 2,
            Activity::End { .. } => 0,
            _ => {
                if self.activity.can_have_error_output() {
                    2
                } else {
                    1
                }
            }
        }
    }

    pub fn get_preferred_output_direction(&self, branch_type: &BranchType) -> OutputDirection {
        match &self.activity {
            Activity::IfCondition { .. } => match branch_type {
                BranchType::TrueBranch => OutputDirection::Down,
                BranchType::FalseBranch => OutputDirection::Right,
                _ => OutputDirection::Down,
            },
            Activity::Loop { .. } => match branch_type {
                BranchType::Default => OutputDirection::Down,
                BranchType::LoopBody => OutputDirection::Right,
                _ => OutputDirection::Down,
            },
            Activity::While { .. } => match branch_type {
                BranchType::Default => OutputDirection::Down,
                BranchType::LoopBody => OutputDirection::Right,
                _ => OutputDirection::Down,
            },
            Activity::TryCatch => match branch_type {
                BranchType::TryBranch => OutputDirection::Down,
                BranchType::CatchBranch => OutputDirection::Right,
                _ => OutputDirection::Down,
            },
            _ => match branch_type {
                BranchType::ErrorBranch => OutputDirection::Right,
                _ => OutputDirection::Down,
            },
        }
    }

    pub fn get_output_pin_positions(&self) -> [Option<egui::Pos2>; 2] {
        let pin_count = self.get_output_pin_count();
        if pin_count == 0 {
            return [None, None];
        }

        match &self.activity {
            Activity::End { .. } | Activity::Note { .. } => [None, None],
            Activity::IfCondition { .. } => {
                let true_dir = self.get_preferred_output_direction(&BranchType::TrueBranch);
                let false_dir = self.get_preferred_output_direction(&BranchType::FalseBranch);
                [
                    Some(self.get_pin_pos_for_direction(true_dir)),
                    Some(self.get_pin_pos_for_direction(false_dir)),
                ]
            }
            Activity::Loop { .. } => {
                let default_dir = self.get_preferred_output_direction(&BranchType::Default);
                let loop_dir = self.get_preferred_output_direction(&BranchType::LoopBody);
                [
                    Some(self.get_pin_pos_for_direction(default_dir)),
                    Some(self.get_pin_pos_for_direction(loop_dir)),
                ]
            }
            Activity::While { .. } => {
                let default_dir = self.get_preferred_output_direction(&BranchType::Default);
                let loop_dir = self.get_preferred_output_direction(&BranchType::LoopBody);
                [
                    Some(self.get_pin_pos_for_direction(default_dir)),
                    Some(self.get_pin_pos_for_direction(loop_dir)),
                ]
            }
            Activity::TryCatch => {
                let try_dir = self.get_preferred_output_direction(&BranchType::TryBranch);
                let catch_dir = self.get_preferred_output_direction(&BranchType::CatchBranch);
                [
                    Some(self.get_pin_pos_for_direction(try_dir)),
                    Some(self.get_pin_pos_for_direction(catch_dir)),
                ]
            }
            Activity::CallScenario { .. } | Activity::RunPowershell { .. } => {
                let default_dir = self.get_preferred_output_direction(&BranchType::Default);
                let error_dir = self.get_preferred_output_direction(&BranchType::ErrorBranch);
                [
                    Some(self.get_pin_pos_for_direction(default_dir)),
                    Some(self.get_pin_pos_for_direction(error_dir)),
                ]
            }
            _ => {
                let default_dir = self.get_preferred_output_direction(&BranchType::Default);
                if self.activity.can_have_error_output() {
                    let error_dir = self.get_preferred_output_direction(&BranchType::ErrorBranch);
                    [
                        Some(self.get_pin_pos_for_direction(default_dir)),
                        Some(self.get_pin_pos_for_direction(error_dir)),
                    ]
                } else {
                    [Some(self.get_pin_pos_for_direction(default_dir)), None]
                }
            }
        }
    }

    fn get_pin_pos_for_direction(&self, direction: OutputDirection) -> egui::Pos2 {
        let center_x = self.width / 2.0;
        let center_y = self.height / 2.0;
        let bottom = self.height;
        let right = self.width;

        match direction {
            OutputDirection::Down => self.position + egui::vec2(center_x, bottom),
            OutputDirection::Right => self.position + egui::vec2(right, center_y),
            OutputDirection::Left => self.position + egui::vec2(0.0, center_y),
            OutputDirection::Up => self.position + egui::vec2(center_x, 0.0),
        }
    }

    #[allow(dead_code)]
    fn get_output_pin_positions_horizontal(&self) -> Vec<egui::Pos2> {
        let pin_count = self.get_output_pin_count();
        if pin_count == 0 {
            return vec![];
        }

        let right = self.width;
        let spacing = self.height / (pin_count as f32 + 1.0);

        (0..pin_count)
            .map(|i| {
                let y_offset = spacing * ((i as f32) + 1.0);
                self.position + egui::vec2(right, y_offset)
            })
            .collect()
    }

    pub fn get_output_pin_pos_by_index(&self, index: usize) -> egui::Pos2 {
        match self.get_output_pin_positions().get(index) {
            Some(Some(pos)) => *pos,
            _ => egui::Pos2::ZERO,
        }
    }

    pub fn get_pin_index_for_branch(&self, branch_type: &BranchType) -> usize {
        match &self.activity {
            Activity::IfCondition { .. } => match branch_type {
                BranchType::TrueBranch => 0,
                BranchType::FalseBranch => 1,
                _ => 0,
            },
            Activity::Loop { .. } => match branch_type {
                BranchType::Default => 0,
                BranchType::LoopBody => 1,
                _ => 0,
            },
            Activity::While { .. } => match branch_type {
                BranchType::Default => 0,
                BranchType::LoopBody => 1,
                _ => 0,
            },
            Activity::TryCatch => match branch_type {
                BranchType::TryBranch => 0,
                BranchType::CatchBranch => 1,
                _ => 0,
            },
            _ => {
                if self.activity.can_have_error_output() {
                    match branch_type {
                        BranchType::ErrorBranch => 1,
                        _ => 0,
                    }
                } else {
                    0
                }
            }
        }
    }

    pub fn get_branch_type_for_pin(&self, pin_index: usize) -> BranchType {
        match &self.activity {
            Activity::IfCondition { .. } => {
                if pin_index == 0 {
                    BranchType::TrueBranch
                } else {
                    BranchType::FalseBranch
                }
            }
            Activity::Loop { .. } | Activity::While { .. } => {
                if pin_index == 1 {
                    BranchType::LoopBody
                } else {
                    BranchType::Default
                }
            }
            Activity::TryCatch => {
                if pin_index == 0 {
                    BranchType::TryBranch
                } else {
                    BranchType::CatchBranch
                }
            }
            _ => {
                if self.activity.can_have_error_output() {
                    if pin_index == 0 {
                        BranchType::Default
                    } else {
                        BranchType::ErrorBranch
                    }
                } else {
                    BranchType::Default
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Activity {
    Start {
        scenario_id: NanoId,
    },
    End {
        scenario_id: NanoId,
    },
    Log {
        level: LogLevel,
        message: String,
    },
    Delay {
        milliseconds: u64,
    },
    SetVariable {
        name: String,
        value: String,
        var_type: VariableType,
        #[serde(default)]
        is_global: bool,
    },
    Evaluate {
        expression: String,
    },
    IfCondition {
        condition: String,
    },
    Loop {
        start: i64,
        end: i64,
        step: i64,
        index: String,
    },
    While {
        condition: String,
    },
    Continue,
    Break,
    CallScenario {
        scenario_id: NanoId,
        #[serde(default)]
        parameters: Vec<VariablesBinding>,
    },
    RunPowershell {
        code: String,
    },
    Note {
        text: String,
        width: f32,
        height: f32,
    },
    TryCatch,
}

impl Activity {
    pub fn can_have_error_output(&self) -> bool {
        matches!(
            self,
            Activity::CallScenario { .. } | Activity::RunPowershell { .. }
        )
    }

    pub fn iter_as_str() -> impl Iterator<Item = &'static str> {
        [
            "Start",
            "End",
            "Log",
            "Delay",
            "SetVariable",
            "Evaluate",
            "IfCondition",
            "Loop",
            "While",
            "Continue",
            "Break",
            "CallScenario",
            "RunPowershell",
            "Note",
            "TryCatch",
        ]
        .iter()
        .copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariableDirection {
    In,
    Out,
    InOut,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariablesBinding {
    pub target_var_name: String,
    pub source_var_name: String,
    pub direction: VariableDirection,
    #[serde(default)]
    pub source_scope: Option<VariableScope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioParameter {
    pub var_name: String,
    pub direction: VariableDirection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Connection {
    pub id: NanoId,
    pub from_node: NanoId,
    pub to_node: NanoId,
    #[serde(default)]
    pub branch_type: BranchType,
}

impl Connection {
    pub fn new(id: NanoId, from_node: NanoId, to_node: NanoId, branch_type: BranchType) -> Self {
        Self {
            id,
            from_node,
            to_node,
            branch_type,
        }
    }

    pub fn new_with_nanoid(from_node: NanoId, to_node: NanoId, branch_type: BranchType) -> Self {
        Self {
            id: NanoId::new_with_nanoid(),
            from_node,
            to_node,
            branch_type,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum BranchType {
    #[default]
    Default,
    TrueBranch,
    FalseBranch,
    LoopBody,
    ErrorBranch,
    TryBranch,
    CatchBranch,
}

mod pos2_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(pos: &egui::Pos2, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (pos.x, pos.y).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<egui::Pos2, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (x, y) = <(f32, f32)>::deserialize(deserializer)?;
        Ok(egui::pos2(x, y))
    }
}
