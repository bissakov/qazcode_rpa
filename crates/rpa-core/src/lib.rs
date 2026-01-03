pub mod activity_metadata;
pub mod canvas_grid;
pub mod constants;
pub mod evaluator_adapter;
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
pub use canvas_grid::{CanvasObstacleGrid, CellState};
pub use constants::{
    ActivityCategories, ActivityDefaults, UiConstants, enforce_minimum_cells, snap_to_grid,
};
pub use execution::{execute_project_with_typed_vars, get_timestamp};
pub use ir::{Instruction, IrBuilder, IrProgram};
pub use node_graph::{
    Activity, BranchType, Connection, NanoId, Node, Project, ProjectFile, Scenario, UiState,
};
pub use stop_control::StopControl;
pub use validation::{
    ScenarioValidator, ValidationCache, ValidationIssue, ValidationLevel, ValidationResult,
};
pub use variables::Variables;
