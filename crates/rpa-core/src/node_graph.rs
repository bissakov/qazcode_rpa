use crate::log::LogLevel;
use crate::log::LogStorage;
use crate::variables::{VariableScope, Variables};
use arc_script::VariableType;
use serde::{Deserialize, Serialize};
use shared::NanoId;

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
        Self {
            id: NanoId::default(),
            name: name.to_string(),
            nodes: Vec::new(),
            connections: Vec::new(),
            parameters: Vec::new(),
            variables: Variables::new(),
        }
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NanoId,
    pub activity: Activity,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Node {
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
            id: NanoId::default(),
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
