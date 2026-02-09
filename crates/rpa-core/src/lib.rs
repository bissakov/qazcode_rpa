pub mod activity_metadata;
pub mod constants;
pub mod evaluator_adapter;
pub mod events;
pub mod execution;
pub mod ir;
pub mod log;
pub mod node_graph;
pub mod stop_control;
pub mod validation;
pub mod variables;

pub use activity_metadata::{
    ActivityCategory, ActivityMetadata, ColorCategory, PinConfig, PropertyDef, PropertyType,
};
pub use constants::{ActivityCategories, ActivityDefaults, CoreConstants};
pub use events::{ExecutionCommand, ExecutionEvent, ExecutionSnapshot};
pub use execution::{execute_project_with_typed_vars, get_timestamp};
pub use ir::{Instruction, IrBuilder, IrProgram};
pub use node_graph::{Activity, BranchType, Connection, Node, Project, ProjectFile, Scenario};
pub use stop_control::StopControl;
pub use validation::{
    ScenarioValidator, ValidationCache, ValidationIssue, ValidationLevel, ValidationResult,
};
pub use variables::Variables;
