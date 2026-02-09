use crate::log::LogEntry;
use arc_script::Value;
use serde::{Deserialize, Serialize};
use shared::NanoId;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSnapshot {
    pub timestamp: String,
    pub global_vars: HashMap<String, Value>,
    pub scenario_vars: HashMap<NanoId, HashMap<String, Value>>,
}

#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    StateSnapshot(ExecutionSnapshot),
    Log(LogEntry),
    Completed,
    Error(String),
}

#[derive(Debug, Clone)]
pub enum ExecutionCommand {
    Stop,
}
