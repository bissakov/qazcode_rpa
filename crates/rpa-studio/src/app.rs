use crate::state::RpaApp;
use eframe::egui;
use rpa_core::UiConstants;
use rust_i18n::t;
use std::time::{Duration, Instant, SystemTime};

impl eframe::App for RpaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let frame_time = now - self.last_frame;
        self.last_frame = now;

        let alpha = 0.1;
        self.smoothed_frame_time = Duration::from_secs_f64(
            self.smoothed_frame_time.as_secs_f64() * (1.0 - alpha)
                + frame_time.as_secs_f64() * alpha,
        );

        self.title_timer += frame_time;
        if self.title_timer >= Duration::from_millis(250) {
            self.title_timer = Duration::ZERO;

            let ms = self.smoothed_frame_time.as_secs_f64() * 1000.0;
            let fps = 1.0 / self.smoothed_frame_time.as_secs_f64();

            ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                format!(
                    "{} — {:.1} FPS ({:.2} ms)",
                    t!("window.title").as_ref(),
                    fps,
                    ms
                )
                .into(),
            ));
        }

        // ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));

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
        self.render_ui_explorer(ctx);
        self.handle_keyboard_shortcuts(ctx);

        ctx.request_repaint();

        // let target_dt = Duration::from_secs_f64(1.0 / self.settings.target_fps as f64);
        // if now.duration_since(self.last_repaint) >= target_dt {
        //     self.last_repaint = now;
        //     ctx.request_repaint_after(target_dt);
        // }
        // ctx.request_repaint_after(target_frame_time);
        // if frame_time < target_frame_time {
        //     std::thread::sleep(target_frame_time - frame_time);
        // }
    }
}

impl RpaApp {
    pub fn process_execution_updates(&mut self, _ctx: &egui::Context) {
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
            // ctx.request_repaint();
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

            self.validate_scenario_indices();
            self.invalidate_current_scenario();
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

            self.validate_scenario_indices();
            self.invalidate_current_scenario();
        }
    }

    fn validate_scenario_indices(&mut self) {
        let scenario_count = self.project.scenarios.len();

        self.opened_scenarios.retain(|&idx| idx < scenario_count);

        if self
            .current_scenario_index
            .is_some_and(|idx| idx >= scenario_count)
        {
            self.current_scenario_index = None;
        }
    }

    pub fn invalidate_current_scenario(&mut self) {
        let scenario = match self.current_scenario_index {
            None => &mut self.project.main_scenario,
            Some(idx) => &mut self.project.scenarios[idx],
        };
        scenario.obstacle_grid.invalidate();
    }
}
