use crate::{constants::UiConstants, variables::Variables};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, fmt};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash)]
pub enum VariableType {
    String,
    Boolean,
    Number,
}

impl VariableType {
    pub fn as_str(&self) -> &str {
        match self {
            VariableType::String => "String",
            VariableType::Boolean => "Boolean",
            VariableType::Number => "Number",
        }
    }

    pub fn all() -> Vec<VariableType> {
        vec![
            VariableType::String,
            VariableType::Boolean,
            VariableType::Number,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VariableValue {
    String(String),
    Boolean(bool),
    Number(f64),
    Undefined,
}

impl VariableValue {
    pub fn get_type(&self) -> VariableType {
        match self {
            VariableValue::String(_) => VariableType::String,
            VariableValue::Boolean(_) => VariableType::Boolean,
            VariableValue::Number(_) => VariableType::Number,
            VariableValue::Undefined => VariableType::String,
        }
    }

    pub fn infer_type_from_string(s: &str) -> VariableType {
        if s.to_lowercase() == "true" || s.to_lowercase() == "false" {
            VariableType::Boolean
        } else if s.parse::<f64>().is_ok() {
            VariableType::Number
        } else {
            VariableType::String
        }
    }

    pub fn from_string(s: &str, var_type: &VariableType) -> Result<Self, String> {
        match var_type {
            VariableType::String => Ok(VariableValue::String(s.to_string())),
            VariableType::Boolean => match s.to_lowercase().as_str() {
                "true" | "1" | "yes" => Ok(VariableValue::Boolean(true)),
                "false" | "0" | "no" => Ok(VariableValue::Boolean(false)),
                _ => Err(format!("Invalid boolean value: {}", s)),
            },
            VariableType::Number => s
                .parse::<f64>()
                .map(VariableValue::Number)
                .map_err(|_| format!("Invalid number value: {}", s)),
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let VariableValue::Boolean(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        if let VariableValue::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            VariableValue::String(s) => Some(s),
            _ => None,
        }
    }
}

impl fmt::Display for VariableValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VariableValue::String(s) => write!(f, "{}", s),
            VariableValue::Boolean(b) => write!(f, "{}", b),
            VariableValue::Number(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{:.0}", n)
                } else {
                    write!(f, "{}", n)
                }
            }
            VariableValue::Undefined => write!(f, ""),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Debug,
}

impl LogLevel {
    pub fn as_str(&self) -> &str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Warning => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Debug => "DEBUG",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub activity: String,
    pub message: String,
}

const MAX_LOG_ENTRIES: usize = 100;

#[derive(Default, Debug, Clone)]
pub struct LogStorage {
    values: VecDeque<LogEntry>,
}

impl LogStorage {
    pub fn new() -> Self {
        Self {
            values: VecDeque::new(),
        }
    }

    pub fn default() -> Self {
        Self {
            values: VecDeque::new(),
        }
    }

    pub fn push(&mut self, entry: LogEntry) {
        if self.values.len() == MAX_LOG_ENTRIES {
            self.values.pop_front();
        }
        self.values.push_back(entry);
    }

    pub fn get(&self, idx: usize) -> Option<&LogEntry> {
        self.values.get(idx)
    }

    pub fn len(&mut self) -> usize {
        self.values.len()
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub main_scenario: Scenario,
    pub scenarios: Vec<Scenario>,
    #[serde(skip)]
    pub execution_log: LogStorage,
    pub variables: Variables,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiState {
    pub current_scenario_index: Option<usize>,
    #[serde(with = "vec2_serde")]
    pub pan_offset: egui::Vec2,
    pub zoom: f32,
    pub font_size: f32,
    pub show_minimap: bool,
    #[serde(default = "default_allow_node_resize")]
    pub allow_node_resize: bool,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
}

fn default_language() -> String {
    "en".to_string()
}

fn default_allow_node_resize() -> bool {
    true
}

fn default_max_iterations() -> usize {
    UiConstants::LOOP_MAX_ITERATIONS
}

impl UiState {
    pub fn normalize_max_iterations(value: usize) -> usize {
        value.clamp(
            UiConstants::LOOP_ITERATIONS_MIN,
            UiConstants::LOOP_ITERATIONS_MAX,
        )
    }

    pub fn is_unlimited(value: usize) -> bool {
        value == 0
    }
}

impl Default for UiState {
    fn default() -> Self {
        use crate::constants::UiConstants;
        Self {
            current_scenario_index: None,
            pan_offset: egui::Vec2::ZERO,
            zoom: 1.0,
            font_size: UiConstants::DEFAULT_FONT_SIZE,
            show_minimap: true,
            allow_node_resize: true,
            language: "en".to_string(),
            max_iterations: UiState::normalize_max_iterations(UiConstants::LOOP_MAX_ITERATIONS),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    pub project: Project,
}

impl Project {
    pub fn new(name: &str, variables: Variables) -> Self {
        Self {
            name: name.to_string(),
            main_scenario: Scenario::new("Main"),
            scenarios: Vec::new(),
            execution_log: LogStorage::new(),
            // initial_variables: indexmap::IndexMap::new(),
            variables,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub id: Uuid,
    pub name: String,
    pub nodes: Vec<Node>,
    pub connections: Vec<Connection>,
}

impl Scenario {
    pub fn new(name: &str) -> Self {
        let mut scenario = Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            nodes: Vec::new(),
            connections: Vec::new(),
        };

        scenario.add_node(
            Activity::Start {
                scenario_id: scenario.id,
            },
            egui::pos2(300.0, 250.0),
        );
        scenario.add_node(
            Activity::End {
                scenario_id: scenario.id,
            },
            egui::pos2(600.0, 250.0),
        );

        if scenario.nodes.len() >= 2 {
            let start_id = scenario.nodes[0].id;
            let end_id = scenario.nodes[1].id;
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
            id: Uuid::new_v4(),
            activity,
            position,
            width,
            height,
        };
        self.nodes.push(node);
    }

    pub fn get_node_mut(&mut self, id: Uuid) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn get_node(&self, id: Uuid) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn remove_node(&mut self, id: Uuid) {
        self.nodes.retain(|n| n.id != id);
        self.connections
            .retain(|c| c.from_node != id && c.to_node != id);
    }

    pub fn add_connection_with_branch(&mut self, from: Uuid, to: Uuid, branch_type: BranchType) {
        if self
            .connections
            .iter()
            .any(|c| c.from_node == from && c.to_node == to && c.branch_type == branch_type)
        {
            return;
        }

        self.connections.push(Connection {
            id: Uuid::new_v4(),
            from_node: from,
            to_node: to,
            branch_type,
        });
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: Uuid,
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

    pub fn get_input_pin_pos(&self) -> egui::Pos2 {
        self.position + egui::vec2(0.0, self.height / 2.0)
    }

    pub fn get_output_pin_pos(&self) -> egui::Pos2 {
        self.position + egui::vec2(self.width, self.height / 2.0)
    }

    pub fn has_input_pin(&self) -> bool {
        !matches!(
            self.activity,
            Activity::Start { .. } | Activity::Note { .. }
        )
    }

    pub fn has_output_pin(&self) -> bool {
        !matches!(self.activity, Activity::End { .. } | Activity::Note { .. })
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

    pub fn get_output_pin_pos_by_index(&self, index: usize) -> egui::Pos2 {
        let pin_offset_top = self.height / 4.0;
        let pin_offset_bottom = self.height * 3.0 / 4.0;
        let pin_offset_center = self.height / 2.0;

        match &self.activity {
            Activity::IfCondition { .. }
            | Activity::Loop { .. }
            | Activity::While { .. }
            | Activity::TryCatch => {
                if index == 0 {
                    self.position + egui::vec2(self.width, pin_offset_top)
                } else {
                    self.position + egui::vec2(self.width, pin_offset_bottom)
                }
            }
            _ => {
                if self.activity.can_have_error_output() {
                    if index == 0 {
                        self.position + egui::vec2(self.width, pin_offset_top)
                    } else {
                        self.position + egui::vec2(self.width, pin_offset_bottom)
                    }
                } else {
                    self.position + egui::vec2(self.width, pin_offset_center)
                }
            }
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
                BranchType::LoopBody => 0,
                BranchType::Default => 1,
                _ => 0,
            },
            Activity::While { .. } => match branch_type {
                BranchType::LoopBody => 0,
                BranchType::Default => 1,
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
                if pin_index == 0 {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Activity {
    Start {
        scenario_id: Uuid,
    },
    End {
        scenario_id: Uuid,
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
    CallScenario {
        scenario_id: Uuid,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: Uuid,
    pub from_node: Uuid,
    pub to_node: Uuid,
    #[serde(default)]
    pub branch_type: BranchType,
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

mod vec2_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(vec: &egui::Vec2, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (vec.x, vec.y).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<egui::Vec2, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (x, y) = <(f32, f32)>::deserialize(deserializer)?;
        Ok(egui::vec2(x, y))
    }
}
