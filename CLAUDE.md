# QazCode RPA Platform
Visual node-based workflow editor. Rust + egui.

## Interaction Rules
- Minimize verbosity
- Keep summarizations brief
- Never announce the next task or subtask unless prompted

## Workspace Structure
Rust workspace with four crates:

### rpa-core (library)
- **node_graph.rs** - Project, Scenario, Node, Activity, Connection, LogEntry, LogLevel, UiState, ProjectFile
- **execution.rs** - Execution engine, variable resolution `{varName}`, LogOutput trait, IrExecutor
- **validation.rs** - Pre-execution validation (structural, control flow, data flow)
- **ir.rs** - IR compilation (node graph → linear instructions)
- **constants.rs** - UiConstants
- **activity_metadata.rs** - ActivityMetadata with properties (PropertyDef, PropertyType), ActivityCategory, ColorCategory, PinConfig
- Activities DO NOT have GUI methods

### rpa-studio (binary)
- **main.rs** - App, panels, save/load, table UI for logs, i18n, metadata-driven activity buttons
- **ui.rs** - Canvas rendering, node/connection drawing, metadata-driven property panel
- **colors.rs** - ColorPalette (uses ActivityMetadata)
- **activity_ext.rs** - Extension trait: get_name() (via metadata), get_color()
- **loglevel_ext.rs** - Extension trait: get_color()
- **locales/** - en.yml, ru.yml
- **build.rs** - Locale loading config
- Uses rust-i18n

### rpa-cli (binary)
- **main.rs** - CLI with clap, execution logic

### validate_locale (binary)
- **main.rs** - Validates localization completeness

## Data Flow
ProjectFile → project + ui_state
Project → main_scenario + scenarios[] + execution_log: Vec<LogEntry> + initial_variables: IndexMap<String, VariableValue>
VariableValue → String(String) | Boolean(bool) | Number(f64)
UiState → current_scenario_index + pan_offset + zoom + font_size + show_minimap + language
Scenario → nodes[] + connections[]
Node → Activity + position + UUID
LogEntry → timestamp + level (Info/Warning/Error) + activity + message

## Save/Load
- Files save as ProjectFile (wrapper with Project + UiState)
- Backward compatible: can load old Project-only files
- UI state persists: zoom, pan, selected scenario, font size, minimap, language

## Logging
- LogEntry struct with timestamp, level, activity, message
- LogOutput trait sends LogEntry via channel to UI
- UI displays logs in striped table (Timestamp | Level | Activity | Message)
- Colored levels: Info=gray, Warning=yellow, Error=red

## Multi-Pin Output
- IfCondition: 0=True, 1=False
- Loop: 0=Body, 1=Next
- Error outputs: 0=Success, 1=Error

## Execution Pipeline
Three-phase execution system:

1. **Validation Phase** (`validation.rs`)
   - **Structural checks**: Start/End nodes required, connection integrity, dead-end detection, pin coverage
   - **Control flow checks**: Loop parameters (step ≠ 0), condition syntax, scenario references, recursion depth (max 100)
   - **Data flow checks**: Variable name validation, undefined variable tracking
   - **Caching**: Hash-based validation cache for performance
   - **Output**: Errors block execution, warnings allow execution

2. **IR Compilation Phase** (`ir.rs`)
   - Converts node graph to linear instruction sequence
   - Flattens control flow: If/While/Loop become conditional jumps
   - Eliminates dead nodes: Only reachable nodes are compiled
   - Single-pass compilation with node-to-instruction mapping
   - Instructions: Start, End, Log, Delay, SetVar, GetVar, Jump, JumpIf, LoopInit, LoopCheck, etc.

3. **Execution Phase** (`execution.rs`)
   - Single-pass instruction dispatch using IrExecutor
   - No node graph traversal during execution
   - Backward compatibility: Old executor kept for CallScenario nodes

## Validation Rules

**Errors (block execution):**
- Missing Start/End nodes
- Dead-end paths (nodes reachable from Start but don't lead to End)
- Loop step = 0 or invalid range (start >= end with positive step)
- Empty variable names
- Invalid scenario references (CallScenario pointing to non-existent scenario)
- Malformed conditions (empty or invalid syntax)
- Loop/While with no after-loop connection (dead-end)

**Warnings (allow execution):**
- If node missing True/False branches
- Try-Catch missing Try/Catch branches
- Loop/While with no body connection (loop will be skipped entirely)
- Undefined variables (used before set)
- Deep scenario recursion (depth > 100)

**Special Cases:**
- Dead nodes (no connections): Completely ignored (no errors/warnings)
- Error output pins: Can be disconnected (user choice)
- Empty loop bodies: Loop skipped (0 iterations) with WARNING

## Localization
- **Languages**: English (en), Russian (ru), Kazakh (kz)
- **Files**: `crates/rpa-studio/locales/{lang}.yml` (_version: 1 format, flat key-value)
- **Usage**: `t!("key.path").as_ref()` macro
- **Settings**: Edit → Settings → Language (saved in UiState)
- **Adding strings**: Add to en.yml, ru.yml, and kz.yml
- **Validation**: `cargo run --bin validate_locale` verifies all metadata keys exist

## Code Rules
- No comments unless necessary
- No hardcoding unless necessary
- All constants in constants.rs
- Extract duplicated logic into methods
- Fix all errors and warnings immediately
- Run `cargo clippy` after each big task
- Update CLAUDE.md and/or README.md if necessary otherwise finish
- Update version in the root's Cargo.toml after every significant change

## Variables
- **Variables**: Define typed variables before execution starts (saved with project)
  - Types: String, Boolean, Number (f64)
  - GUI: Right panel → "Variables" → "Variables" → "+ Add Variable"
    - Select type from dropdown
    - Boolean accepts: true/false, 1/0, yes/no
    - Number displays without decimals if whole number
  - CLI: Automatically loaded from project file, converted to strings during execution
  - CLI `--var` arguments override project variables (strings only)
  - Variables are passed to execution context at startup
  - **Display order**: First-in order (insertion order preserved via IndexMap)
- **Runtime Variables**: Show variables created/modified during execution
  - Display only, updated in real-time during workflow execution
  - All runtime variables are strings internally
  - Types are inferred for display: "true"/"false" → Boolean, numeric strings → Number, else → String
  - CLI verbose mode (`-v`) shows types in format: `name  [Type]  value`
  - **Display order**: First-in order (creation order preserved)

## Build & Run
```bash
# Build workspace
cargo build --release

# Run studio (GUI)
cargo run --bin rpa-studio --release

# Run CLI
cargo run --bin rpa-cli --release -- project.rpa
cargo run --bin rpa-cli --release -- project.rpa -v
cargo run --bin rpa-cli --release -- project.rpa -s "Scenario 1"
cargo run --bin rpa-cli --release -- project.rpa --var name=John --var count=5
cargo run --bin rpa-cli --release -- project.rpa -v -s "Login Flow" --var user=admin

# Validate localization
cargo run --bin validate_locale
```

## Adding Activity
1. Add to Activity enum (rpa-core/node_graph.rs)
2. Impl `can_have_error_output()` in Activity
3. Add metadata entry with properties (rpa-core/activity_metadata.rs) - defines button, category, properties
4. Add to `activities_by_category()` in ActivityMetadata
5. Add localization keys to en.yml and ru.yml (validate with `cargo run --bin validate_locale`)
6. Add `add_*_node()` to Scenario (rpa-core/node_graph.rs)
7. Handle in `execute_activity()` (rpa-core/execution.rs)

Note: Button generation and property rendering are now metadata-driven (no changes needed in main.rs or ui.rs)
