# Agent Guidelines for QazCode RPA Platform

Visual node-based RPA workflow editor built with Rust and egui. Create and execute automation workflows with a GUI designer (`rpa-studio`) and CLI runner (`rpa-cli`).

---

## Interaction Rules
- Minimize verbosity
- Keep summarizations brief
- Never announce the next task or subtask unless prompted

## Project Structure

### **rpa-core** (library)
- `node_graph.rs` – Project, Scenario, Node, Activity, Connection, logs, UI state
- `execution.rs` – Execution engine, IR executor, variable resolution
- `validation.rs` – Pre-execution validation with hash caching
- `ir.rs` – Node graph → linear IR
- `evaluator.rs` – Expression parser and evaluator
- `variables.rs` – Variable storage (ID-based)
- `activity_metadata.rs` – Metadata driving UI generation
- `constants.rs` – UI constants and defaults
- `utils.rs` – Interruptible sleep

### **rpa-studio** (binary)
- `main.rs` – App state, panels, dialogs, execution control
- `ui.rs` – Canvas, nodes, connections, minimap, tools
- `activity_ext.rs` – Activity extensions (name, color)
- `colors.rs` – ColorPalette
- `loglevel_ext.rs` – LogLevel color mapping
- `locales/` – en.yml, ru.yml, kz.yml

### **rpa-cli** (binary)
- Headless execution of `.rpa` projects
- Flags:
- `-v` verbose
- `-s` scenario
- `--var` variable overrides
- Exit codes:
- `0` success
- `1` error

### **validate_locale** (tool)
- Validates locale completeness against `activity_metadata.rs`

---

## Code Rules
- No comments unless strictly necessary
- No hardcoding unless unavoidable
- All constants go in `constants.rs`
- Extract duplicated logic
- Fix all errors and warnings immediately
- Update `CLAUDE.md` and/or `README.md` if architecture or features change
- Bump version in root `Cargo.toml` after every significant change

---

## After Changes
- Update version in root `Cargo.toml` after significant changes
- Update documentation if behavior or architecture changes
