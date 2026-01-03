use crate::state::RpaApp;
use rpa_core::log::{LogActivity, LogEntry, LogLevel};
use rpa_core::{Project, ProjectFile};
use rust_i18n::t;

impl RpaApp {
    pub fn save_project(&mut self) {
        if let Some(path) = &self.current_file {
            self.save_to_file(path.clone());
        } else {
            self.save_project_as();
        }
    }

    pub fn save_project_as(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("RPA Project", &["rpa"])
            .save_file()
        {
            self.save_to_file(path);
        }
    }

    pub fn save_to_file(&mut self, path: std::path::PathBuf) {
        let project_file = ProjectFile {
            project: self.project.clone(),
        };

        match serde_json::to_string(&project_file) {
            Ok(json) => match std::fs::write(&path, json) {
                Ok(_) => {
                    self.current_file = Some(path.clone());
                    // TODO: Decide undo history behavior on save:
                    // - Option 1: Clear history on file save (cleaner UX, current recommendation)
                    // - Option 2: Persist history to temporary state file (more powerful, uses disk space)
                    // - Option 3: Keep history independent of save (current implementation)
                    // When decided, call: self.undo_redo.clear_undo_history();
                    self.project.execution_log.push(LogEntry {
                        timestamp: "[00:00.00]".to_string(),
                        node_id: None,
                        level: LogLevel::Info,
                        activity: LogActivity::System,
                        message: t!("system_messages.project_saved", path = path.display())
                            .to_string(),
                    });
                }
                Err(e) => {
                    self.project.execution_log.push(LogEntry {
                        timestamp: "[00:00.00]".to_string(),
                        node_id: None,
                        level: LogLevel::Error,
                        activity: LogActivity::System,
                        message: t!("system_messages.failed_save", error = e).to_string(),
                    });
                }
            },
            Err(e) => {
                self.project.execution_log.push(LogEntry {
                    timestamp: "[00:00.00]".to_string(),
                    node_id: None,
                    level: LogLevel::Error,
                    activity: LogActivity::System,
                    message: t!("system_messages.failed_serialize", error = e).to_string(),
                });
            }
        }
    }

    pub fn open_project(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("RPA Project", &["rpa"])
            .pick_file()
        {
            match std::fs::read_to_string(&path) {
                Ok(contents) => {
                    let result = serde_json::from_str::<ProjectFile>(&contents).or_else(|_| {
                        serde_json::from_str::<Project>(&contents)
                            .map(|project| ProjectFile { project })
                    });

                    match result {
                        Ok(mut project_file) => {
                            project_file.project.execution_log.clear();
                            project_file.project.execution_log.push(LogEntry {
                                timestamp: "".to_string(),
                                node_id: None,
                                level: LogLevel::Info,
                                activity: LogActivity::System,
                                message: t!(
                                    "system_messages.project_loaded",
                                    path = path.display()
                                )
                                .to_string(),
                            });

                            self.project = project_file.project;
                            self.current_scenario_index = None;
                            self.init_current_scenario_view();

                            rust_i18n::set_locale(&self.settings.language);
                            self.current_file = Some(path);

                            self.selected_nodes.clear();
                        }
                        Err(e) => {
                            self.project.execution_log.push(LogEntry {
                                timestamp: "".to_string(),
                                node_id: None,
                                level: LogLevel::Error,
                                activity: LogActivity::System,
                                message: t!("system_messages.failed_parse", error = e).to_string(),
                            });
                        }
                    }
                }
                Err(e) => {
                    self.project.execution_log.push(LogEntry {
                        timestamp: "[00:00.00]".to_string(),
                        node_id: None,
                        level: LogLevel::Error,
                        activity: LogActivity::System,
                        message: t!("system_messages.failed_read", error = e).to_string(),
                    });
                }
            }
        }
    }
}
