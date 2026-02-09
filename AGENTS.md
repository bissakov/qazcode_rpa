# Agent Guidelines for QazCode RPA Platform

Visual node-based RPA workflow editor built with Rust and egui. Create and execute automation workflows with a GUI designer (`rpa-studio`) and CLI runner (`rpa-cli`).

---

## Interaction Rules
- Minimize verbosity
- Keep summarizations brief
- Never announce the next task or subtask unless prompted

## Code Rules
- Run `cargo check` and `cargo fmt` after major changes
- No comments unless strictly necessary
- No hardcoding unless unavoidable
- Extract duplicated logic
- Fix all errors and warnings immediately

---

## After Changes
- Update version in root `Cargo.toml` after significant changes
- Update documentation if behavior or architecture changes
