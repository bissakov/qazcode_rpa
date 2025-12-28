mod activity_ext;
mod app;
mod colors;
mod custom;
mod dialogs;
mod events;
mod file_io;
mod loglevel_ext;
mod state;
mod ui;
mod undo_redo;

use eframe::egui;
use egui::IconData;
use rpa_core::{
    IrBuilder, LogEntry, LogLevel, ScenarioValidator, UiConstants, execute_project_with_typed_vars,
    get_timestamp,
};
use rust_i18n::t;
use state::RpaApp;
use std::sync::mpsc::channel;
use std::time::SystemTime;

rust_i18n::i18n!("locales", fallback = "en");

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    target_fps: usize,
    font_size: f32,
    show_minimap: bool,
    allow_node_resize: bool,
    language: String,
    current_max_entry_size: usize,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            target_fps: 60,
            font_size: UiConstants::DEFAULT_FONT_SIZE,
            show_minimap: true,
            allow_node_resize: false,
            language: "en".to_string(),
            current_max_entry_size: UiConstants::DEFAULT_LOG_ENTRIES,
        }
    }
}

fn load_icon() -> IconData {
    let bytes = include_bytes!("../../../resources/icon.ico");
    let img = image::load_from_memory(bytes).unwrap().to_rgba8();
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
        vsync: false,
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_visible(false)
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
            level: LogLevel::Info,
            activity: "SYSTEM".to_string(),
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
                level: LogLevel::Error,
                activity: "SYSTEM".to_string(),
                message: format!(
                    "Execution aborted: {} validation errors",
                    validation_result.errors.len()
                ),
            });
            self.project.execution_log.push(LogEntry {
                timestamp: "[00:00.00]".to_string(),
                level: LogLevel::Info,
                activity: "SYSTEM".to_string(),
                message: UiConstants::EXECUTION_COMPLETE_MARKER.to_string(),
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
                    level: LogLevel::Error,
                    activity: "SYSTEM".to_string(),
                    message: format!("IR compilation failed: {}", e),
                });
                self.project.execution_log.push(LogEntry {
                    timestamp: "[00:00.00]".to_string(),
                    level: LogLevel::Info,
                    activity: "SYSTEM".to_string(),
                    message: UiConstants::EXECUTION_COMPLETE_MARKER.to_string(),
                });
                self.is_executing = false;
                return;
            }
        };

        let stop_control = self.stop_control.clone();
        let variables = self.global_variables.clone();

        let project = std::sync::Arc::new(self.project.clone());

        std::thread::spawn(move || {
            execute_project_with_typed_vars(
                &project,
                &log_sender,
                start_time,
                &program,
                variables,
                stop_control,
            );
        });
    }
}
