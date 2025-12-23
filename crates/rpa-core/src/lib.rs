pub mod activity_metadata;
pub mod constants;
pub mod evaluator;
pub mod execution;
pub mod ir;
pub mod node_graph;
pub mod utils;
pub mod validation;
pub mod variables;

pub use activity_metadata::{
    ActivityCategory, ActivityMetadata, ColorCategory, PinConfig, PropertyDef, PropertyType,
};
pub use constants::{ActivityCategories, ActivityDefaults, UiConstants};
pub use evaluator::evaluate;
pub use execution::{
    execute_project_with_typed_vars, execute_project_with_vars, execute_scenario_with_vars,
};
pub use ir::{Instruction, IrProgram};
pub use node_graph::{
    Activity, BranchType, Connection, LogEntry, LogLevel, Node, Project, ProjectFile, Scenario,
    UiState, VariableType, VariableValue,
};
pub use validation::{ValidationCache, ValidationIssue, ValidationLevel, ValidationResult};
pub use variables::{VarEvent, VarId, Variables};
