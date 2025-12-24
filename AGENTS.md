# Agent Guidelines for QazCode RPA Platform

Visual node-based RPA workflow editor built with Rust and egui. Create and execute automation workflows with a GUI designer (rpa-studio) and CLI runner (rpa-cli).

## Project Structure

Rust workspace with 4 members (version 0.1.6, edition 2024):

**rpa-core** (library) - Core shared library:
- `node_graph.rs` (596 lines) - Data structures: Project, Scenario, Node, Activity, Connection, LogEntry, LogLevel, UiState, VariableValue
- `execution.rs` (806 lines) - Execution engine, IrExecutor, ExecutionContext, variable resolution, LogOutput trait
- `validation.rs` (1,014 lines) - Pre-execution validation with hash-based caching
- `ir.rs` (610 lines) - IR compilation (node graph → linear instructions)
- `evaluator.rs` (803 lines) - Expression parser/evaluator with arithmetic, comparison, boolean logic
- `variables.rs` (103 lines) - Variable storage with ID-based indexing
- `activity_metadata.rs` (539 lines) - Metadata system driving UI generation
- `constants.rs` (80 lines) - UI constants and defaults
- `utils.rs` (18 lines) - Interruptible sleep function

**rpa-studio** (binary) - Visual GUI application:
- `main.rs` (1,546 lines) - App state, menu bars, panels, execution management, dialogs
- `ui.rs` (1,797 lines) - Canvas rendering, node/connection drawing, property panels, context menus, knife tool, minimap
- `activity_ext.rs` (19 lines) - Extension trait for Activity (name, color)
- `colors.rs` (46 lines) - ColorPalette for activity and connection colors
- `loglevel_ext.rs` (17 lines) - Extension trait for LogLevel colors
- `locales/` - en.yml, ru.yml, kz.yml (English, Russian, Kazakh)

**rpa-cli** (binary) - Command-line runner:
- Executes `.rpa` projects headless
- Supports verbose mode (`-v`), scenario selection (`-s`), variable overrides (`--var`)
- Exit codes: 0 (success), 1 (error)

**validate_locale** (tool) - Localization validator:
- Scans activity_metadata.rs for required keys
- Validates all locale files for completeness
- Reports missing keys per language

## Build & Test
- Build all: `cargo build --release`
- Build specific: `cargo build --release --bin rpa-studio` (or `rpa-cli`, `validate_locale`)
- Lint: `cargo clippy` (run after each big task, fix all warnings)
- Test: `cargo test` (currently 9 tests in evaluator.rs only)
- Validate i18n: `cargo run --bin validate_locale`

## Code Style
- **No comments** unless necessary
- **No hardcoding** - all constants in `crates/rpa-core/src/constants.rs`
- Extract duplicated logic into methods
- Fix all errors and warnings immediately
- Imports: `use crate::*` for internal, standard/external libs first
- Types: Explicit types on struct fields, use `impl Trait` sparingly
- Naming: `snake_case` functions/vars, `PascalCase` types, `SCREAMING_SNAKE_CASE` constants
- Error handling: Use `Result<T, String>` for errors with descriptive messages
- Formatting: Standard rustfmt (4-space indent, 100 char line length)

## Architecture
- Activities have NO GUI methods (metadata-driven in `activity_metadata.rs`)
- IndexMap for ordered collections (variables, scenarios)
- Extension traits for UI concerns (`activity_ext.rs`, `loglevel_ext.rs`)
- **IR-based execution**: Scenarios are validated, compiled to IR, then executed (single-pass, no recursion)

## Execution Pipeline
1. **Validation** (`validation.rs`) - Pre-execution checks for structural/control/data flow issues
   - Structural: Start/End nodes, connection integrity, dead-end detection, pin coverage
   - Control Flow: Loop parameters, condition syntax, scenario references, recursion depth
   - Data Flow: Variable name validation, undefined variable tracking
   - Hash-based caching for performance
2. **IR Compilation** (`ir.rs`) - Convert node graph to linear instruction sequence
   - Flattens control flow (If/While/Loop → conditional jumps)
   - Eliminates dead nodes (only compiles reachable nodes)
   - Single-pass compilation with node-to-instruction mapping
3. **Execution** (`execution.rs`) - Single-pass instruction dispatch with IrExecutor
   - Instruction-based execution (no node graph traversal)
   - Backward compatibility: Old executor kept for CallScenario nodes

## Validation Rules
**Errors (block execution):**
- Missing Start/End nodes, dead-end paths, disconnected after-loop pins
- Invalid loop parameters (step=0, invalid range)
- Empty variable names, invalid scenario references, malformed conditions

**Warnings (allow execution):**
- If/Try-Catch missing branches, empty loop bodies (loop skipped)
- Undefined variables, deep scenario recursion (depth > 100)

**Special Cases:**
- Dead nodes (no connections): Completely ignored
- Error output pins: Can be disconnected
- Loop/While with no body connection: WARNING + loop skipped (0 iterations)
- Loop/While with no after-loop connection: ERROR (dead-end)

## Dependencies

**Workspace:**
- serde 1.0 (derive), serde_json 1.0
- uuid 1.0 (v4, serde)
- indexmap 2.0 (serde) - Ordered collections
- rust-i18n 3 - Internationalization

**rpa-core:**
- egui 0.33.3

**rpa-cli:**
- clap 4.5 (derive) - CLI argument parsing
- dhat 0.3.3 (optional) - Heap profiling
- embed-resource 2.5.2 (build) - Windows icon embedding

**rpa-studio:**
- eframe 0.33.3, egui 0.33.3, egui_extras 0.33.3
- egui_code_editor 0.2.20 - Code editing widget
- rfd 0.15 - Native file dialogs
- syntect 5.3.0 - Syntax highlighting
- once_cell 1.21.3, image 0.25.9
- embed-resource 2.5.2 (build)

## Testing

**Current Coverage:**
- Only 9 tests in `evaluator.rs` (rpa-core):
  - Arithmetic, parentheses, comparison, boolean logic
  - Variable interpolation, complex expressions
  - Error cases (empty, division by zero, undefined variables)
- No tests in: rpa-cli, rpa-studio, validate_locale, or other rpa-core modules

## Key Features

**Studio:**
- Metadata-driven UI (activities defined in activity_metadata.rs, no GUI methods in Activity enum)
- Knife tool for cutting connections
- Node resizing for notes (8 resize handles)
- Minimap (200x150, configurable)
- Runtime variables panel (real-time monitoring during execution)
- Debug IR output (`show_debug_ir` flag)
- Multi-language support: English, Russian, Kazakh

**CLI:**
- Headless execution with channel-based logging
- ASCII art banner
- Threaded execution with stop flag
- Variable overrides via `--var` flag

**Variable System:**
- Typed Variables: Pre-defined with types (String/Boolean/Number), saved with project
- Runtime Variables: Created/modified during execution, displayed in real-time
- ID-based storage with efficient lookup
- Variable interpolation: `{varName}` syntax

## Activities

**Flow Control:**
- Start, End

**Basic:**
- Log Message, Delay, Set Variable, Evaluate (expression parser with error output)

**Control Flow:**
- If Condition (True/False branches)
- Loop (body/next outputs, sets `loop_counter`)
- While (condition-based looping)
- Try-Catch (error handling with Try/Catch branches)

**Scenarios:**
- Call Scenario (executes another scenario, shares variable context)

**Scripting:**
- Run Powershell (placeholder, not yet implemented)

## Adding New Activity

1. Add to Activity enum (rpa-core/node_graph.rs)
2. Implement `can_have_error_output()` in Activity
3. Add metadata entry with properties (rpa-core/activity_metadata.rs)
4. Add to `activities_by_category()` in ActivityMetadata
5. Add localization keys to en.yml, ru.yml, kz.yml
6. Validate with `cargo run --bin validate_locale`
7. Add `add_*_node()` to Scenario (rpa-core/node_graph.rs)
8. Handle in `execute_activity()` (rpa-core/execution.rs)

Note: Button generation and property rendering are metadata-driven (no UI code changes needed)

## After Changes
- Update version in root `Cargo.toml` after significant changes
- Update `CLAUDE.md` and/or `README.md` if architecture/features change
