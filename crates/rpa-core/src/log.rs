use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::constants::CoreConstants;
use shared::NanoId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogActivity {
    Start,
    End,
    Log,
    Delay,
    SetVariable,
    Evaluate,
    IfCondition,
    Loop,
    While,
    Continue,
    Break,
    CallScenario,
    RunPowershell,
    Note,
    TryCatch,
    Execution,
    System,
}

impl LogActivity {
    pub fn as_str(&self) -> &str {
        match self {
            LogActivity::Start => "START",
            LogActivity::End => "END",
            LogActivity::Log => "LOG",
            LogActivity::Delay => "DELAY",
            LogActivity::SetVariable => "SET VARIABLE",
            LogActivity::Evaluate => "EVALUATE",
            LogActivity::IfCondition => "IF CONDITION",
            LogActivity::Loop => "LOOP",
            LogActivity::While => "WHILE",
            LogActivity::Continue => "CONTINUE",
            LogActivity::Break => "BREAK",
            LogActivity::CallScenario => "CALL SCENARIO",
            LogActivity::RunPowershell => "RUN POWERSHELL",
            LogActivity::Note => "NOTE",
            LogActivity::TryCatch => "TRY CATCH",
            LogActivity::Execution => "EXECUTION",
            LogActivity::System => "SYSTEM",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub node_id: Option<NanoId>,
    pub level: LogLevel,
    pub activity: LogActivity,
    pub message: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct LogStorage {
    values: VecDeque<LogEntry>,
    pub max_entry_count: usize,
}

impl LogStorage {
    pub fn new() -> Self {
        Self {
            values: VecDeque::with_capacity(CoreConstants::DEFAULT_LOG_ENTRIES),
            max_entry_count: CoreConstants::DEFAULT_LOG_ENTRIES,
        }
    }

    pub fn push(&mut self, entry: LogEntry) {
        if self.values.len() == self.max_entry_count {
            self.values.pop_front();
        }
        self.values.push_back(entry);
    }

    pub fn get(&self, idx: usize) -> Option<&LogEntry> {
        self.values.get(idx)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }
}
