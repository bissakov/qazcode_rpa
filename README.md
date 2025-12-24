# QazCode RPA Platform

Visual node-based RPA workflow editor built with Rust and egui. Create and execute automation workflows with a GUI designer (rpa-studio) and CLI runner (rpa-cli).

## Project Structure

Rust workspace with 4 members (version 0.1.6, edition 2024):

- **rpa-core** (library) - Core shared library with data structures, execution engine, validation, IR compilation, and metadata system
- **rpa-studio** (binary) - Visual GUI application with drag-and-drop workflow designer
- **rpa-cli** (binary) - Command-line runner for headless execution
- **validate_locale** (tool) - Localization validator for multi-language support

## Features

### Studio (GUI)
- Metadata-driven UI with no GUI methods in Activity enum
- Visual node-based workflow designer with drag-and-drop interface
- Knife tool for cutting connections
- Node resizing for notes (8 resize handles)
- Minimap (200x150, configurable)
- Runtime variables panel (real-time monitoring during execution)
- Multi-language support: English, Russian, Kazakh

### CLI
- Headless execution with channel-based logging
- Threaded execution with stop flag
- Variable overrides via `--var` flag
- Verbose mode and scenario selection

### Core Architecture
- IR-based execution: Scenarios validated → compiled to IR → executed (single-pass, no recursion)
- Comprehensive validation with hash-based caching
- Expression parser/evaluator with arithmetic, comparison, boolean logic
- Variable system with ID-based indexing and interpolation

## Building

```bash
# Build all
cargo build --release

# Build specific
cargo build --release --bin rpa-studio
cargo build --release --bin rpa-cli
cargo build --release --bin validate_locale

# Development builds
cargo build
```

## Testing & Validation

```bash
# Run tests
cargo test

# Lint (run after changes)
cargo clippy

# Validate localization
cargo run --bin validate_locale
```

## Usage

### GUI Studio

Launch the visual workflow designer:

```bash
# Release build
cargo run --bin rpa-studio --release

# Development build
cargo run --bin rpa-studio
```

Features:
- Create and edit workflows visually
- Save/load `.rpa` project files
- Real-time execution with visual feedback
- Node properties panel
- Runtime variable monitoring

### CLI Runner

Execute workflows from command line:

```bash
# Run main scenario (development)
cargo run --bin rpa-cli -- project.rpa

# Run with release build
cargo run --bin rpa-cli --release -- project.rpa

# Run with verbose output
cargo run --bin rpa-cli --release -- project.rpa -v

# Run specific scenario
cargo run --bin rpa-cli --release -- project.rpa -s "Scenario 1"

# Set initial variables
cargo run --bin rpa-cli --release -- project.rpa --var name=John --var count=5

# Combined options
cargo run --bin rpa-cli --release -- project.rpa -v -s "Login Flow" --var user=admin
```

CLI Options:
- `-v, --verbose`: Print detailed execution logs
- `-s, --scenario <NAME>`: Run specific scenario by name
- `--var <NAME=VALUE>`: Set initial variable values

## Activities

### Flow Control
- **Start**: Entry point for workflow
- **End**: Exit point for workflow

### Basic Activities
- **Log Message**: Write message to execution log (supports variable interpolation)
- **Delay**: Pause execution for specified milliseconds
- **Set Variable**: Create or update a variable

### Control Flow
- **If Condition**: Conditional branching with True/False branches
- **Loop**: Repeat execution N times with body/next outputs, sets `loop_counter`
- **While**: Condition-based looping
- **Try-Catch**: Error handling with Try/Catch branches

### Scenarios
- **Call Scenario**: Execute another scenario, shares variable context

### Scripting
- **Run Powershell**: Execute PowerShell code (placeholder, not yet implemented)

## Variable System

### Typed Variables
- Pre-defined variables with types (String/Boolean/Number), saved with project
- Runtime variables created/modified during execution, displayed in real-time
- ID-based storage with efficient lookup

### Variable Interpolation
Use `{varName}` syntax in expressions and conditions:

```
Set Variable: name = "John"
Log Message: "Hello, {name}!"  → outputs "Hello, John!"
```

### Expression Support
- Arithmetic: `{count} + 1`, `{price} * {quantity}`
- Comparison: `{var} == "value"`, `{count} > 5`, `{count} >= 10`
- Boolean logic: `{condition} && {other}`, `{flag} || {fallback}`

## Execution Pipeline

1. **Validation** - Pre-execution checks for structural/control/data flow issues
   - Structural: Start/End nodes, connection integrity, dead-end detection
   - Control Flow: Loop parameters, condition syntax, scenario references
   - Data Flow: Variable name validation, undefined variable tracking
   - Hash-based caching for performance

2. **IR Compilation** - Convert node graph to linear instruction sequence
   - Flattens control flow (If/While/Loop → conditional jumps)
   - Eliminates dead nodes (only compiles reachable nodes)
   - Single-pass compilation with node-to-instruction mapping

3. **Execution** - Single-pass instruction dispatch with IrExecutor
   - Instruction-based execution (no node graph traversal)
   - Backward compatibility: Old executor kept for CallScenario nodes

## Error Handling

Activities with error outputs:
- Success path: Output pin 0
- Error path: Output pin 1
- Error message stored in `{last_error}` variable

### Validation Rules
**Errors (block execution):**
- Missing Start/End nodes, dead-end paths, disconnected after-loop pins
- Invalid loop parameters (step=0, invalid range)
- Empty variable names, invalid scenario references, malformed conditions

**Warnings (allow execution):**
- If/Try-Catch missing branches, empty loop bodies
- Undefined variables, deep scenario recursion (depth > 100)

## Project Format

Projects saved as `.rpa` files in JSON format:
- Main scenario and additional scenarios (reusable workflows)
- Node positions, properties, and connections
- Typed variables and UI state

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

## Exit Codes

CLI exit codes:
- `0`: Success
- `1`: Execution errors or project load failure
