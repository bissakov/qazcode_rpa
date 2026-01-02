use eframe::egui;
use egui_extras::{Column, TableBuilder};
use rpa_core::ir::Instruction;
use rust_i18n::t;

use crate::state::RpaApp;

impl RpaApp {
    pub fn render_ir_debug_window(&mut self, ctx: &egui::Context) {
        let mut is_open = self.dialogs.debug.show_ir_view;

        egui::Window::new(t!("ir_view.title").as_ref())
            .open(&mut is_open)
            .resizable(true)
            .default_width(900.0)
            .default_height(600.0)
            .show(ctx, |ui| {
                if ui.button(t!("ir_view.recompile").as_ref()).clicked() {
                    self.compile_ir_for_debug();
                }

                ui.separator();

                if let Some(error) = &self.dialogs.debug.compilation_error {
                    ui.colored_label(egui::Color32::RED, t!("ir_view.error").as_ref());
                    ui.label(error.as_str());
                    ui.separator();
                } else if let Some(program) = &self.dialogs.debug.compiled_ir_program {
                    ui.label(format!(
                        "{}: {}",
                        t!("ir_view.entry_point").as_ref(),
                        program.entry_point
                    ));
                    ui.label(format!(
                        "{}: {}",
                        t!("ir_view.total_instructions").as_ref(),
                        program.instructions.len()
                    ));

                    if !program.scenario_start_index.is_empty() {
                        ui.separator();
                        ui.label(t!("ir_view.scenario_map").as_ref());
                        let mut entries: Vec<_> = program.scenario_start_index.iter().collect();
                        entries.sort_by_key(|e| e.1);
                        for (scenario_id, idx) in entries {
                            ui.label(format!("  {} → {}", scenario_id, idx));
                        }
                    }

                    ui.separator();

                    let table = TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .column(Column::auto().resizable(true).range(40.0..=80.0))
                        .column(Column::remainder().resizable(true).range(150.0..=400.0))
                        .column(Column::remainder().resizable(true));

                    table
                        .header(18.0, |mut header| {
                            header.col(|ui| {
                                ui.strong(t!("ir_view.index").as_ref());
                            });
                            header.col(|ui| {
                                ui.strong(t!("ir_view.instruction").as_ref());
                            });
                            header.col(|ui| {
                                ui.strong(t!("ir_view.details").as_ref());
                            });
                        })
                        .body(|body| {
                            body.rows(18.0, program.instructions.len(), |mut row| {
                                let row_index = row.index();
                                let instruction = &program.instructions[row_index];
                                let (instr_str, details) = format_instruction(instruction);

                                row.col(|ui| {
                                    ui.label(row_index.to_string());
                                });
                                row.col(|ui| {
                                    ui.label(instr_str);
                                });
                                row.col(|ui| {
                                    ui.label(details);
                                });
                            });
                        });
                } else {
                    ui.label("Waiting for compilation...");
                }
            });

        self.dialogs.debug.show_ir_view = is_open;
        if !is_open {
            self.dialogs.debug.compiled_ir_program = None;
            self.dialogs.debug.compilation_error = None;
        }
    }
}

fn format_instruction(instruction: &Instruction) -> (String, String) {
    match instruction {
        Instruction::Start { scenario_id } => ("Start".to_string(), format!("{}", scenario_id)),
        Instruction::End { scenario_id } => ("End".to_string(), format!("{}", scenario_id)),
        Instruction::Log { level, message } => (
            "Log".to_string(),
            format!("{:?}: {}", level, truncate(message, 60)),
        ),
        Instruction::Delay { milliseconds } => ("Delay".to_string(), format!("{}ms", milliseconds)),
        Instruction::SetVar { var, value, scope } => (
            "SetVar".to_string(),
            format!("{} = {:?} ({:?})", var, value, scope),
        ),
        Instruction::Evaluate { expr } => ("Evaluate".to_string(), format!("{:?}", expr)),
        Instruction::Jump { target } => ("Jump".to_string(), format!("→ {}", target)),
        Instruction::JumpIf { condition, target } => (
            "JumpIf".to_string(),
            format!("cond: {:?} → {}", condition, target),
        ),
        Instruction::JumpIfNot { condition, target } => (
            "JumpIfNot".to_string(),
            format!("cond: {:?} → {}", condition, target),
        ),
        Instruction::LoopInit { index, start } => {
            ("LoopInit".to_string(), format!("{} = {}", index, start))
        }
        Instruction::LoopLog {
            index,
            start,
            end,
            step,
        } => (
            "LoopLog".to_string(),
            format!("{} from {} to {} step {}", index, start, end, step),
        ),
        Instruction::LoopCheck {
            index,
            end,
            step,
            body_target,
            end_target,
        } => (
            "LoopCheck".to_string(),
            format!(
                "{} <= {} step {}: body→{} end→{}",
                index, end, step, body_target, end_target
            ),
        ),
        Instruction::LoopNext {
            index,
            step,
            check_target,
        } => (
            "LoopNext".to_string(),
            format!("{} += {} → {}", index, step, check_target),
        ),
        Instruction::WhileCheck {
            condition,
            body_target,
            end_target,
        } => (
            "WhileCheck".to_string(),
            format!("{:?}: body→{} end→{}", condition, body_target, end_target),
        ),
        Instruction::LoopContinue { check_target } => {
            ("LoopContinue".to_string(), format!("→ {}", check_target))
        }
        Instruction::LoopBreak { end_target } => {
            ("LoopBreak".to_string(), format!("→ {}", end_target))
        }
        Instruction::PushErrorHandler { catch_target } => (
            "PushErrorHandler".to_string(),
            format!("→ {}", catch_target),
        ),
        Instruction::PopErrorHandler => ("PopErrorHandler".to_string(), String::new()),
        Instruction::CallScenario {
            scenario_id,
            parameters,
        } => (
            "CallScenario".to_string(),
            format!("{} with {} params", scenario_id, parameters.len()),
        ),
        Instruction::RunPowershell { code } => ("RunPowershell".to_string(), truncate(code, 60)),
        Instruction::DebugMarker {
            node_id,
            description,
        } => (
            "★ DebugMarker".to_string(),
            format!("{}: {}", node_id, description),
        ),
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s.to_string()
    }
}
