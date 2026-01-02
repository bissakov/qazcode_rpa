#[derive(Debug, Clone, PartialEq)]
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
            Value::Boolean(b) => Ok(*b),
            _ => Err("Expected boolean".into()),
        }
    }

    fn to_number(&self) -> Result<f64, String> {
        match self {
            Value::Number(n) => Ok(*n),
            Value::Boolean(b) => Ok(if *b { 1.0 } else { 0.0 }),
            _ => Err("Expected number".into()),
        }
    }
}
