use arc_script::{Value, VariableResolver};
use crate::variables::Variables;

impl VariableResolver for Variables {
    fn resolve(&self, name: &str) -> Result<Value, String> {
        let v = self.get(name);
        match v {
            Some(val) if !matches!(val, Value::Undefined) => Ok(val.clone()),
            _ => Err(format!("Undefined variable: {}", name)),
        }
    }
}
