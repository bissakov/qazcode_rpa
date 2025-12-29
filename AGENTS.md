# Agent Guidelines for QazCode RPA Platform

Visual node-based RPA workflow editor built with Rust and egui. Create and execute automation workflows with a GUI designer (`rpa-studio`) and CLI runner (`rpa-cli`).

---

## Interaction Rules
- Minimize verbosity
- Keep summarizations brief
- Never announce the next task or subtask unless prompted

---

## Issue Tracking (GitHub Issues via `gh`)

This project uses **GitHub Issues** managed through the `gh` CLI.

### Issue Title Convention (MANDATORY)

All issues **must include the type prefix** in the title:

- `[BUG] ...`
- `[TASK] ...`
- `[FEATURE] ...`
- `[DOCS] ...`
- `[CHORE] ...`

Example:
```
[BUG] Crash when deleting connected node
```

Issues without a prefix are invalid.

---

### Finding Work
- `gh issue list` – List open issues
- `gh issue list --label bug` – List bugs
- `gh issue view <id>` – View issue details
- `gh issue list --assignee @me` – Your assigned issues

---

### Creating Issues
```

gh issue create 
--title "[BUG] ..." 
--body "Description" 
--label bug

```

Common labels:
- `bug`
- `task`
- `feature`
- `docs`
- `chore`
- `blocked`
- `priority:critical`
- `priority:high`
- `priority:medium`
- `priority:low`

---

### Claiming & Updating Work
- Assign yourself:
```
gh issue edit <id> --add-assignee @me
```
- Add / remove labels:
```
gh issue edit <id> --add-label blocked
gh issue edit <id> --remove-label blocked
```
- Close issue:
```
gh issue close <id>
```

Dependencies are tracked via:
- Linked issues (`blocked by #ID`)
- `blocked` label
- Explicit references in the issue body

---

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

## Build
- Build all:
```
cargo build --release
```
- Build specific:
```
cargo build --release --bin rpa-studio
```
- Lint:
```
cargo clippy
```
- Validate i18n:
```
cargo run --bin validate_locale
```

---

## Code Rules
- No comments unless strictly necessary
- No hardcoding unless unavoidable
- All constants go in `constants.rs`
- Extract duplicated logic
- Fix all errors and warnings immediately
- Run `cargo clippy` after each major task
- Update `CLAUDE.md` and/or `README.md` if architecture or features change
- Bump version in root `Cargo.toml` after every significant change

---

## After Changes
- Update version in root `Cargo.toml` after significant changes
- Update documentation if behavior or architecture changes

---

## Landing the Plane (Session Completion)

**Work is not complete until pushed.**

### SESSION CLOSE PROTOCOL
```
[ ] 1. cargo fmt
[ ] 2. git status
[ ] 3. git add <files>
[ ] 4. git commit -m "..."
[ ] 5. git push
```

### Critical Rules
- Never stop before `git push`
- Never say “ready to push”
- If push fails, fix and retry
- Track multi-session or discovered work as GitHub issues
- Use local TODOs only for trivial, single-session work

