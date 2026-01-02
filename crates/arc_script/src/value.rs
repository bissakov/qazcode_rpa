use crate::variable_type::VariableType;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Number(f64),
    Boolean(bool),
    String(String),
    Undefined,
}

pub trait ValueExt {
    fn to_bool(&self) -> Result<bool, String>;
    fn to_number(&self) -> Result<f64, String>;
}

impl ValueExt for Value {
    fn to_bool(&self) -> Result<bool, String> {
        match self {
            Self::Boolean(b) => Ok(*b),
            _ => Err("Expected boolean".into()),
        }
    }

    fn to_number(&self) -> Result<f64, String> {
        match self {
            Self::Number(n) => Ok(*n),
            Self::Boolean(b) => Ok(if *b { 1.0 } else { 0.0 }),
            _ => Err("Expected number".into()),
        }
    }
}

impl Value {
    #[must_use]
    pub const fn get_type(&self) -> VariableType {
        match self {
            Self::String(_) => VariableType::String,
            Self::Boolean(_) => VariableType::Boolean,
            Self::Number(_) | Self::Undefined => VariableType::Number,
        }
    }

    #[must_use]
    pub fn infer_type_from_string(s: &str) -> VariableType {
        if s.to_lowercase() == "true" || s.to_lowercase() == "false" {
            VariableType::Boolean
        } else if s.parse::<f64>().is_ok() {
            VariableType::Number
        } else {
            VariableType::String
        }
    }

    /// Converts a string to a Value of the specified type.
    ///
    /// # Errors
    ///
    /// Returns an error if the string cannot be parsed as the specified type.
    pub fn from_string(s: &str, var_type: &VariableType) -> Result<Self, String> {
        match var_type {
            VariableType::String => Ok(Self::String(s.to_string())),
            VariableType::Boolean => match s.to_lowercase().as_str() {
                "true" | "1" | "yes" => Ok(Self::Boolean(true)),
                "false" | "0" | "no" => Ok(Self::Boolean(false)),
                _ => Err(format!("Invalid boolean value: {s}")),
            },
            VariableType::Number => s
                .parse::<f64>()
                .map(Self::Number)
                .map_err(|_| format!("Invalid number value: {s}")),
        }
    }

    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        if let Self::Boolean(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    #[must_use]
    pub const fn as_number(&self) -> Option<f64> {
        if let Self::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(s) => write!(f, "{s}"),
            Self::Boolean(b) => write!(f, "{b}"),
            Self::Number(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{n:.0}")
                } else {
                    write!(f, "{n}")
                }
            }
            Self::Undefined => write!(f, ""),
        }
    }
}

pub trait VariableResolver {
    /// Resolves a variable by name.
    ///
    /// # Errors
    ///
    /// Returns an error if the variable is not found.
    fn resolve(&self, name: &str) -> Result<Value, String>;
}
