use crate::VariableValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct VarId(u32);

impl VarId {
    #[inline]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variables {
    name_to_id: HashMap<String, VarId>,
    id_to_name: Vec<String>,
    values: Vec<VariableValue>,
}

impl Default for Variables {
    fn default() -> Self {
        Self::new()
    }
}

impl Variables {
    pub fn new() -> Self {
        Self {
            name_to_id: HashMap::new(),
            id_to_name: Vec::new(),
            values: Vec::new(),
        }
    }

    pub fn snapshot(&self) -> Vec<VariableValue> {
        self.values.clone()
    }

    pub fn values(&self) -> &[VariableValue] {
        &self.values
    }

    pub fn id(&mut self, name: &str) -> VarId {
        if let Some(&id) = self.name_to_id.get(name) {
            return id;
        }
        let id = VarId(self.id_to_name.len() as u32);
        self.name_to_id.insert(name.to_owned(), id);
        self.id_to_name.push(name.to_owned());
        self.values.push(VariableValue::Undefined);
        id
    }

    pub fn name(&self, id: VarId) -> &str {
        &self.id_to_name[id.index()]
    }

    pub fn set(&mut self, id: VarId, value: VariableValue) {
        self.values[id.index()] = value;
    }

    pub fn get(&self, id: VarId) -> &VariableValue {
        &self.values[id.index()]
    }

    pub fn remove(&mut self, id: VarId) {
        self.values[id.index()] = VariableValue::Undefined;
    }

    pub fn contains(&self, name: &str) -> bool {
        self.name_to_id.contains_key(name)
    }

    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.name_to_id.keys()
    }

    pub fn is_empty(&self) -> bool {
        self.id_to_name.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &VariableValue)> {
        self.name_to_id
            .iter()
            .map(move |(name, &id)| (name, &self.values[id.index()]))
    }

    pub fn clear(&mut self) {
        self.name_to_id.clear();
        self.id_to_name.clear();
        self.values.clear();
    }
}

pub enum VarEvent {
    Set { name: String, value: VariableValue },
    Remove { name: String },
    SetId { id: VarId, value: VariableValue },
    RemoveId { id: VarId },
}
