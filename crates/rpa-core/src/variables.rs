use arc_script::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VariableScope {
    Global,
    Scenario,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variable {
    value: Value,
    scope: VariableScope,
}

impl Variable {
    pub fn new(value: Value, scope: VariableScope) -> Self {
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
        self.values
            .insert(name.to_owned(), Variable::new(Value::Undefined, scope));
    }

    pub fn get_scope(&self, name: &str) -> Option<&VariableScope> {
        self.values.get(name).map(|var| &var.scope)
    }

    pub fn set(&mut self, name: &str, value: Value, scope: VariableScope) {
        self.values
            .insert(name.to_owned(), Variable::new(value, scope));
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
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

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value, &VariableScope)> {
        self.values
            .iter()
            .map(|(name, var)| (name.as_str(), &var.value, &var.scope))
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn merge(&self, other: &Variables) -> Variables {
        let mut merged = self.clone();
        for (name, var) in &other.values {
            merged.values.insert(name.clone(), var.clone());
        }
        merged
    }
}
