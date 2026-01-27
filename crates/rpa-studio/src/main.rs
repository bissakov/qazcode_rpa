mod app;
mod colors;
mod custom;
mod dialogs;
mod events;
mod ext;
mod file_io;
mod settings;
mod state;
mod ui;
mod ui_constants;
mod undo_redo;

use crate::ui_constants::UiConstants;
use eframe::egui;
use egui::IconData;
use rpa_core::execution::{ExecutionContext, ScopeFrame};
use rpa_core::log::{LogActivity, LogEntry, LogLevel};
use rpa_core::{CoreConstants, IrBuilder, ScenarioValidator, get_timestamp};
use rust_i18n::t;
use state::RpaApp;
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

rust_i18n::i18n!("locales", fallback = "en");

fn load_icon() -> IconData {
    let bytes = include_bytes!("../../../resources/icon.ico");

    let img = match image::load_from_memory(bytes) {
        Ok(img) => img.to_rgba8(),
        Err(_) => return IconData::default(),
    };

    let (w, h) = img.dimensions();

    IconData {
        rgba: img.into_raw(),
        width: w,
        height: h,
    }
}

fn main() -> eframe::Result<()> {
    rust_i18n::set_locale("en");

    let options = eframe::NativeOptions {
        vsync: true,
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            // .with_visible(false)
            .with_maximized(true)
            .with_title(t!("window.title").as_ref())
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        t!("window.title").as_ref(),
        options,
        Box::new(|cc| {
            let app = RpaApp::default().with_initial_snapshot();
            rust_i18n::set_locale(&app.settings.language);
            let mut style = (*cc.egui_ctx.style()).clone();
            style.text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::proportional(app.settings.font_size),
            );
            style.text_styles.insert(
                egui::TextStyle::Button,
                egui::FontId::proportional(app.settings.font_size),
            );
            style.text_styles.insert(
                egui::TextStyle::Small,
                egui::FontId::proportional(
                    app.settings.font_size * UiConstants::SMALL_FONT_MULTIPLIER,
                ),
            );
            cc.egui_ctx.set_style(style);

            Ok(Box::new(app))
        }),
    )
}

impl RpaApp {
    fn execute_project(&mut self) {
        self.project.execution_log.clear();
        self.project.execution_log.push(LogEntry {
            timestamp: "[00:00.00]".to_string(),
            node_id: None,
            level: LogLevel::Info,
            activity: LogActivity::System,
            message: t!("system_messages.execution_start").to_string(),
        });

        self.stop_control.reset();

        let (log_sender, log_receiver) = channel();

        self.log_receiver = Some(log_receiver);

        let start_time = SystemTime::now();
        let validator = ScenarioValidator::new(&self.project.main_scenario, &self.project);
        let validation_result = validator.validate();

        if !validation_result.is_valid() {
            self.project.execution_log.push(LogEntry {
                timestamp: get_timestamp(start_time),
                node_id: None,
                level: LogLevel::Error,
                activity: LogActivity::System,
                message: format!(
                    "Execution aborted: {} validation errors",
                    validation_result.errors.len()
                ),
            });
            for error in validation_result.errors {
                self.project.execution_log.push(LogEntry {
                    timestamp: get_timestamp(start_time),
                    node_id: None,
                    level: LogLevel::Error,
                    activity: LogActivity::System,
                    message: error.message,
                });
            }

            for warning in validation_result.warnings {
                self.project.execution_log.push(LogEntry {
                    timestamp: get_timestamp(start_time),
                    node_id: None,
                    level: LogLevel::Warning,
                    activity: LogActivity::System,
                    message: warning.message,
                });
            }

            self.project.execution_log.push(LogEntry {
                timestamp: "[00:00.00]".to_string(),
                node_id: None,
                level: LogLevel::Info,
                activity: LogActivity::System,
                message: CoreConstants::EXECUTION_COMPLETE_MARKER.to_string(),
            });
            self.is_executing = false;
            return;
        }

        let ir_builder = IrBuilder::new(
            &self.project.main_scenario,
            &self.project,
            &validation_result.reachable_nodes,
            &mut self.global_variables,
        );
        let program = match ir_builder.build() {
            Ok(prog) => prog,
            Err(e) => {
                self.project.execution_log.push(LogEntry {
                    timestamp: get_timestamp(start_time),
                    node_id: None,
                    level: LogLevel::Error,
                    activity: LogActivity::System,
                    message: format!("IR compilation failed: {}", e),
                });
                self.project.execution_log.push(LogEntry {
                    timestamp: "[00:00.00]".to_string(),
                    node_id: None,
                    level: LogLevel::Info,
                    activity: LogActivity::System,
                    message: CoreConstants::EXECUTION_COMPLETE_MARKER.to_string(),
                });
                self.is_executing = false;
                return;
            }
        };

        let stop_control = self.stop_control.clone();
        let variables = self.global_variables.clone();

        let scope_stack = vec![ScopeFrame {
            scenario_id: self.project.main_scenario.id.clone(),
            variables: self.project.main_scenario.variables.clone(),
        }];

        let context = Arc::new(RwLock::new(ExecutionContext::new_without_sender(
            start_time,
            scope_stack,
            variables,
            stop_control,
        )));

        self.execution_context = Some(context.clone());

        let project = std::sync::Arc::new(self.project.clone());

        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut log = log_sender.clone();
                let mut executor = rpa_core::execution::IrExecutor::new(
                    &program,
                    &project,
                    context.clone(),
                    &mut log,
                );

                if let Err(e) = executor.execute()
                    && e != "Execution stopped by user"
                {
                    let _ = log_sender.send(LogEntry {
                        timestamp: get_timestamp(context.read().unwrap().start_time),
                        node_id: None,
                        level: LogLevel::Error,
                        activity: LogActivity::System,
                        message: format!("Execution error: {e}"),
                    });
                }
            }));

            if let Err(panic) = result {
                let panic_msg = if let Some(s) = panic.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "Unknown panic occurred".to_string()
                };

                let _ = log_sender.send(LogEntry {
                    timestamp: get_timestamp(start_time),
                    node_id: None,
                    level: LogLevel::Error,
                    activity: LogActivity::System,
                    message: format!("Execution interrupted: {panic_msg}"),
                });
            }

            let _ = log_sender.send(LogEntry {
                timestamp: get_timestamp(start_time),
                node_id: None,
                level: LogLevel::Info,
                activity: LogActivity::System,
                message: "Execution completed.".to_string(),
            });
            let _ = log_sender.send(LogEntry {
                timestamp: get_timestamp(start_time),
                node_id: None,
                level: LogLevel::Info,
                activity: LogActivity::System,
                message: CoreConstants::EXECUTION_COMPLETE_MARKER.to_string(),
            });
        });
    }

    fn compile_ir_for_debug(&mut self) {
        let scenario = self.get_current_scenario();
        let validator = ScenarioValidator::new(scenario, &self.project);
        let validation_result = validator.validate();

        if !validation_result.is_valid() {
            let error_msg = validation_result
                .errors
                .iter()
                .map(|e| format!("- {}", e.message))
                .collect::<Vec<_>>()
                .join("\n");
            self.dialogs.debug.compilation_error = Some(error_msg);
            self.dialogs.debug.compiled_ir_program = None;
            return;
        }

        let mut vars = self.global_variables.clone();
        let ir_builder = IrBuilder::new(
            scenario,
            &self.project,
            &validation_result.reachable_nodes,
            &mut vars,
        );

        match ir_builder.build() {
            Ok(program) => {
                self.dialogs.debug.compiled_ir_program = Some(program);
                self.dialogs.debug.compilation_error = None;
            }
            Err(e) => {
                self.dialogs.debug.compilation_error = Some(format!("IR compilation error: {}", e));
                self.dialogs.debug.compiled_ir_program = None;
            }
        }
    }
}
