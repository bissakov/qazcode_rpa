use crate::ext::ScenarioExt;
use rpa_core::{Project, Scenario, Variables, log::LogStorage};

pub trait ProjectExt {
    fn new_empty(name: &str, variables: Variables) -> Self;
}

impl ProjectExt for Project {
    fn new_empty(name: &str, variables: Variables) -> Self {
        Self {
            name: name.to_string(),
            main_scenario: Scenario::new_empty("Main"),
            scenarios: Vec::new(),
            execution_log: LogStorage::new(),
            variables,
        }
    }
}
