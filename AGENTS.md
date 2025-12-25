# Agent Guidelines for QazCode RPA Platform

Visual node-based RPA workflow editor built with Rust and egui. Create and execute automation workflows with a GUI designer (rpa-studio) and CLI runner (rpa-cli).

## Interaction Rules
- Minimize verbosity
- Keep summarizations brief
- Never announce the next task or subtask unless prompted

## Issue Tracking

This project uses **bd (beads)** for issue tracking. Run `bd prime` for workflow context, or install hooks (`bd hooks install`) for auto-injection.

### Finding Work
- `bd ready` - Show issues ready to work (no blockers)
- `bd list --status=open` - All open issues
- `bd list --status=in_progress` - Active work
- `bd show <id>` - Detailed issue view with dependencies

### Creating & Updating Issues
- `bd create --title="..." --type=task|bug|feature --priority=2` - New issue
  - Priority: 0-4 (0=critical, 2=medium, 4=backlog)
- `bd update <id> --status=in_progress` - Claim work
- `bd update <id> --assignee=username` - Assign to someone
- `bd close <id>` - Mark complete
- `bd close <id1> <id2> ...` - Close multiple issues at once

### Dependencies & Blocking
- `bd dep add <issue> <depends-on>` - Add dependency
- `bd blocked` - Show all blocked issues

### Session Workflow
- `bd sync` - Sync with git remote (run at session end)
- `bd sync --status` - Check sync status without syncing
- `bd stats` - Project statistics
- `bd doctor` - Check for issues

## Project Structure

**rpa-core** (library) - Core shared library:
- `node_graph.rs` - Data structures: Project, Scenario, Node, Activity, Connection, LogEntry, LogLevel, UiState, VariableValue
- `execution.rs` - Execution engine, IrExecutor, ExecutionContext, variable resolution, LogOutput trait
- `validation.rs` - Pre-execution validation with hash-based caching
- `ir.rs` - IR compilation (node graph → linear instructions)
- `evaluator.rs` - Expression parser/evaluator with arithmetic, comparison, boolean logic
- `variables.rs` - Variable storage with ID-based indexing
- `activity_metadata.rs` - Metadata system driving UI generation
- `constants.rs` - UI constants and defaults
- `utils.rs` - Interruptible sleep function

**rpa-studio** (binary) - Visual GUI application:
- `main.rs` - App state, menu bars, panels, execution management, dialogs
- `ui.rs` - Canvas rendering, node/connection drawing, property panels, context menus, knife tool, minimap
- `activity_ext.rs` - Extension trait for Activity (name, color)
- `colors.rs` - ColorPalette for activity and connection colors
- `loglevel_ext.rs` - Extension trait for LogLevel colors
- `locales/` - en.yml, ru.yml, kz.yml (English, Russian, Kazakh)

**rpa-cli** (binary) - Command-line runner:
- Executes `.rpa` projects headless
- Supports verbose mode (`-v`), scenario selection (`-s`), variable overrides (`--var`)
- Exit codes: 0 (success), 1 (error)

**validate_locale** (tool) - Localization validator:
- Scans activity_metadata.rs for required keys
- Validates all locale files for completeness
- Reports missing keys per language

## Build
- Build all: `cargo build --release`
- Build specific: `cargo build --release --bin rpa-studio` (or `rpa-cli`, `validate_locale`)
- Lint: `cargo clippy` (run after each big task, fix all warnings)
- Validate i18n: `cargo run --bin validate_locale`

## Code Rules
- No comments unless necessary
- No hardcoding unless necessary
- All constants in constants.rs
- Extract duplicated logic into methods
- Fix all errors and warnings immediately
- Run `cargo clippy` after each big task
- Update CLAUDE.md and/or README.md if necessary otherwise finish
- Update version in the root's Cargo.toml after every significant change

## After Changes
- Update version in root `Cargo.toml` after significant changes
- Update `CLAUDE.md` and/or `README.md` if architecture/features change

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**SESSION CLOSE PROTOCOL:**

```
[ ] 1. git status              (check what changed)
[ ] 2. git add <files>         (stage code changes)
[ ] 3. bd sync                 (commit beads changes)
[ ] 4. git commit -m "..."     (commit code)
[ ] 5. bd sync                 (commit any new beads changes)
[ ] 6. git push                (push to remote)
```

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
- Track strategic work in beads (multi-session, dependencies, discovered work)
- Use TodoWrite for simple single-session execution tasks

### Core Workflow Principles
- **When in doubt**, prefer bd—persistence you don't need beats lost context
- **Git workflow**: Hooks auto-sync, run `bd sync` at session end
- **Session management**: Check `bd ready` for available work
- Track dependencies explicitly with `bd dep add` when work blocks other work

