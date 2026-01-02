use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum VariableType {
    String,
    Boolean,
    Number,
}

impl VariableType {
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::String => "String",
            Self::Boolean => "Boolean",
            Self::Number => "Number",
        }
    }

    #[must_use]
    pub fn all() -> Vec<Self> {
        vec![Self::String, Self::Boolean, Self::Number]
    }
}
