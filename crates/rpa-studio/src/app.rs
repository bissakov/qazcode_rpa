use crate::state::RpaApp;
use eframe::egui;
use rpa_core::UiConstants;
use std::time::SystemTime;

impl eframe::App for RpaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_execution_updates(ctx);

        let old_debounce = self.property_edit_debounce;
        self.property_edit_debounce -= ctx.input(|i| i.stable_dt);
        if old_debounce > 0.0 && self.property_edit_debounce <= 0.0 {
            self.snapshot_undo_state();
        }

        self.render_menu_bar(ctx);
        self.render_left_sidebar(ctx);
        self.render_right_panel(ctx);

        self.render_console_panel(ctx);
        let (context_action, mouse_world_pos) = self.render_canvas_panel(ctx);
        self.handle_context_menu_action(context_action, mouse_world_pos);
        self.render_dialogs(ctx);
        self.handle_keyboard_shortcuts(ctx);
    }
}

impl RpaApp {
    pub fn process_execution_updates(&mut self, ctx: &egui::Context) {
        let mut execution_complete = false;
        if let Some(receiver) = self.log_receiver.as_ref() {
            for log_entry in receiver.try_iter() {
                if log_entry.message == UiConstants::EXECUTION_COMPLETE_MARKER {
                    execution_complete = true;
                    break;
                } else {
                    self.project.execution_log.push(log_entry);
                }
            }
            ctx.request_repaint();
        }

        if execution_complete {
            self.is_executing = false;
            self.log_receiver = None;
        }
    }

    pub fn snapshot_undo_state(&mut self) {
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        self.undo_redo.feed_state(time, &self.project);
    }

    pub fn undo(&mut self) {
        if let Some(restored_project) = self.undo_redo.undo(&self.project) {
            self.project = restored_project;
            self.selected_nodes.clear();
            self.connection_from = None;
            self.knife_tool_active = false;
            self.knife_path.clear();
            self.resizing_node = None;

            if self
                .current_scenario_index
                .is_some_and(|idx| idx >= self.project.scenarios.len())
            {
                self.current_scenario_index = None;
            }
        }
    }

    pub fn redo(&mut self) {
        if let Some(restored_project) = self.undo_redo.redo(&self.project) {
            self.project = restored_project;
            self.selected_nodes.clear();
            self.connection_from = None;
            self.knife_tool_active = false;
            self.knife_path.clear();
            self.resizing_node = None;

            if self
                .current_scenario_index
                .is_some_and(|idx| idx >= self.project.scenarios.len())
            {
                self.current_scenario_index = None;
            }
        }
    }
}
