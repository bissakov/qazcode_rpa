use crate::VariableValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VariableScope {
    Global,
    Scenario,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variable {
    value: VariableValue,
    scope: VariableScope,
}

impl Variable {
    pub fn new(value: VariableValue, scope: VariableScope) -> Self {
        Self { value, scope }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variables {
    values: HashMap<String, Variable>,
}

impl Default for Variables {
    fn default() -> Self {
        Self::new()
    }
}

impl Variables {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn create_variable(&mut self, name: &str, scope: VariableScope) {
        self.values.insert(
            name.to_owned(),
            Variable::new(VariableValue::Undefined, scope),
        );
    }

    pub fn get_scope(&self, name: &str) -> Option<&VariableScope> {
        self.values.get(name).map(|var| &var.scope)
    }

    pub fn set_scope(&mut self, name: &str, scope: VariableScope) {
        if let Some(var) = self.values.get_mut(name) {
            var.scope = scope;
        }
    }

    pub fn set(&mut self, name: &str, value: VariableValue) {
        if let Some(var) = self.values.get_mut(name) {
            var.value = value;
        }
    }

    pub fn get(&self, name: &str) -> Option<&VariableValue> {
        self.values.get(name).map(|var| &var.value)
    }

    pub fn remove(&mut self, name: &str) {
        self.values.remove(name);
    }

    pub fn contains(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }

    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.values.keys()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &VariableValue, &VariableScope)> {
        self.values
            .iter()
            .map(|(name, var)| (name.as_str(), &var.value, &var.scope))
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }
}

pub enum VarEvent {
    Set { name: String, value: VariableValue },
    Remove { name: String },
}
