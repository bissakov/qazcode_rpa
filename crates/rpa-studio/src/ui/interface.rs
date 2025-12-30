use crate::loglevel_ext::LogLevelExt;
use crate::state::RpaApp;
use crate::ui::canvas;
use eframe::egui;
use egui::{DragValue, Slider};
use egui_extras::{Column, TableBuilder};
use rpa_core::{
    Activity, LogEntry, LogLevel, Project, Scenario, UiConstants, VariableType, VariableValue,
    Variables, node_graph::VariableDirection, variables::VariableScope,
};
use rust_i18n::t;

use crate::custom::scenario_tab;

impl RpaApp {
    pub fn render_canvas_panel(
        &mut self,
        ctx: &egui::Context,
    ) -> (canvas::ContextMenuAction, egui::Vec2) {
        egui::CentralPanel::default()
            .show(ctx, |ui| {
                self.render_scenario_tab_bar(ui);
                ui.separator();

                let clipboard_empty = self.clipboard.nodes.is_empty();
                let show_minimap = self.settings.show_minimap;
                let allow_node_resize = self.settings.allow_node_resize;

                let current_scenario_index = self.current_scenario_index;
                let scenario = match current_scenario_index {
                    None => &mut self.project.main_scenario,
                    Some(i) => &mut self.project.scenarios[i],
                };

                let mut render_state = canvas::RenderState {
                    selected_nodes: &mut self.selected_nodes,
                    connection_from: &mut self.connection_from,
                    clipboard_empty,
                    show_minimap,
                    knife_tool_active: &mut self.knife_tool_active,
                    knife_path: &mut self.knife_path,
                    resizing_node: &mut self.resizing_node,
                    allow_node_resize,
                };

                let view = self.scenario_views.entry(scenario.id.clone()).or_default();

                let (canvas_result, dropped_activity) = ui
                    .dnd_drop_zone::<Activity, _>(egui::Frame::default(), |ui| {
                        canvas::render_node_graph(ui, scenario, &mut render_state, view)
                    });

                let (
                    context_action,
                    mouse_world_pos,
                    connection_created,
                    _drag_started,
                    drag_ended,
                    _resize_started,
                    resize_ended,
                ) = canvas_result.inner;

                if connection_created {
                    self.undo_redo.add_undo(&self.project);
                }

                if drag_ended && !self.is_executing {
                    self.undo_redo.add_undo(&self.project);
                }

                if resize_ended && !self.is_executing {
                    self.undo_redo.add_undo(&self.project);
                }

                if let Some(activity) = dropped_activity {
                    let pointer_pos = ctx.input(|i| i.pointer.interact_pos());
                    if let Some(pointer_pos) = pointer_pos {
                        let view = self.get_current_scenario_view_mut();
                        let world_pos =
                            ((pointer_pos.to_vec2() - view.pan_offset) / view.zoom).to_pos2();
                        self.get_current_scenario_mut()
                            .add_node((*activity).clone(), world_pos);
                        self.undo_redo.add_undo(&self.project);
                    }
                }

                (context_action, mouse_world_pos)
            })
            .inner
    }

    pub fn render_console_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("console")
            .resizable(true)
            .default_height(UiConstants::CONSOLE_HEIGHT)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(t!("panels.output").as_ref());

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(t!("bottom_bar.clear").as_ref()).clicked() {
                            self.project.execution_log.clear();
                            self.dialogs.selected_log_entry = None;
                        }
                    });
                });
                ui.separator();

                let row_count = self.project.execution_log.len();
                let text_height = egui::TextStyle::Body
                    .resolve(ui.style())
                    .size
                    .max(ui.spacing().interact_size.y);
                let available_height = ui.available_height();

                let table = TableBuilder::new(ui)
                    .auto_shrink(false)
                    .stick_to_bottom(true)
                    .striped(row_count > 0 && row_count < 500)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::initial(100.0).resizable(true))
                    .column(Column::initial(100.0).resizable(true))
                    .column(Column::initial(120.0).resizable(true))
                    .column(Column::remainder().resizable(true))
                    .min_scrolled_height(0.0)
                    .max_scroll_height(available_height);

                table
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.strong(t!("output_table.timestamp").as_ref());
                        });
                        header.col(|ui| {
                            ui.strong(t!("output_table.level").as_ref());
                        });
                        header.col(|ui| {
                            ui.strong(t!("output_table.activity").as_ref());
                        });
                        header.col(|ui| {
                            ui.strong(t!("output_table.message").as_ref());
                        });
                    })
                    .body(|body| {
                        if row_count == 0 {
                            body.rows(text_height, 5, |_row| {});
                            return;
                        }

                        body.rows(text_height * 1.4, row_count, |mut row| {
                            let row_index = row.index();
                            if let Some(log_entry) = &self.project.execution_log.get(row_index) {
                                row.col(|ui| {
                                    ui.label(&log_entry.timestamp);
                                });

                                row.col(|ui| {
                                    ui.colored_label(
                                        log_entry.level.get_color(),
                                        log_entry.level.as_str(),
                                    );
                                });

                                row.col(|ui| {
                                    ui.label(&log_entry.activity);
                                });

                                row.col(|ui| {
                                    let message = &log_entry.message;
                                    let has_newlines = message.contains('\n');

                                    if has_newlines {
                                        let mut lines = message.lines();
                                        let first_line = lines.next().unwrap_or("");
                                        let line_count = lines.count();
                                        let truncated =
                                            format!("{} [+{} lines]", first_line, line_count - 1);

                                        let response = ui.add(
                                            egui::Label::new(&truncated)
                                                .sense(egui::Sense::click()),
                                        );

                                        if response.clicked() {
                                            self.dialogs.selected_log_entry = Some(row_index);
                                        }

                                        if response.hovered() {
                                            ui.ctx()
                                                .set_cursor_icon(egui::CursorIcon::PointingHand);
                                        }
                                    } else {
                                        ui.label(message);
                                    }
                                });
                            }
                        });
                    });
            });
    }

    pub fn render_dialogs(&mut self, ctx: &egui::Context) {
        if let Some(log_index) = self.dialogs.selected_log_entry {
            let mut is_open = true;
            egui::Window::new("Log Message")
                .id(egui::Id::new("log_message_window"))
                .open(&mut is_open)
                .resizable(true)
                .default_width(600.0)
                .default_height(400.0)
                .show(ctx, |ui| {
                    if let Some(log_entry) = self.project.execution_log.get(log_index) {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.add(egui::Label::new(&log_entry.message).selectable(true).wrap());
                        });
                    }
                });

            if !is_open {
                self.dialogs.selected_log_entry = None;
            }
        }

        if self.dialogs.settings.show {
            if self.dialogs.settings.temp_settings.is_none() {
                self.dialogs.settings.temp_settings = Some(self.settings.clone());
            }

            let mut close_dialog = false;

            egui::Window::new(t!("settings_dialog.title").as_ref())
                .id(egui::Id::new("settings_window"))
                .open(&mut self.dialogs.settings.show)
                .show(ctx, |ui| {
                    if let Some(temp) = &mut self.dialogs.settings.temp_settings {
                        ui.heading(t!("settings_dialog.display_settings").as_ref());
                        ui.separator();

                        ui.label(t!("settings_dialog.max_fps").as_ref());
                        ui.add(egui::Slider::new(&mut temp.target_fps, 15..=540).text("FPS"));

                        ui.separator();

                        ui.label(t!("settings_dialog.font_size").as_ref());
                        ui.add(egui::Slider::new(&mut temp.font_size, 6.0..=32.0).text("pt"));

                        ui.separator();

                        ui.checkbox(
                            &mut temp.show_minimap,
                            t!("settings_dialog.show_minimap").as_ref(),
                        );

                        ui.checkbox(
                            &mut temp.allow_node_resize,
                            t!("settings_dialog.allow_node_resize").as_ref(),
                        );

                        ui.separator();

                        ui.label(t!("settings_dialog.log_entry_count").as_ref());
                        ui.add(Slider::new(
                            &mut temp.current_max_entry_size,
                            10..=UiConstants::MAX_LOG_ENTRIES,
                        ));

                        self.project.execution_log.max_entry_count = temp.current_max_entry_size;

                        ui.separator();

                        ui.label(t!("settings_dialog.language").as_ref());
                        let current_language = if temp.language == "ru" {
                            "Русский"
                        } else if temp.language == "kz" {
                            "Қазақша"
                        } else {
                            "English"
                        };
                        egui::ComboBox::from_id_salt("language_selector")
                            .selected_text(current_language)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut temp.language,
                                    "en".to_string(),
                                    "English",
                                );
                                ui.selectable_value(
                                    &mut temp.language,
                                    "kz".to_string(),
                                    "Қазақша",
                                );
                                ui.selectable_value(
                                    &mut temp.language,
                                    "ru".to_string(),
                                    "Русский",
                                );
                            });

                        ui.separator();
                        if ui.button(t!("settings_dialog.apply").as_ref()).clicked() {
                            let mut style = (*ctx.style()).clone();
                            style.text_styles.insert(
                                egui::TextStyle::Body,
                                egui::FontId::proportional(temp.font_size),
                            );
                            style.text_styles.insert(
                                egui::TextStyle::Button,
                                egui::FontId::proportional(temp.font_size),
                            );
                            style.text_styles.insert(
                                egui::TextStyle::Small,
                                egui::FontId::proportional(
                                    temp.font_size * UiConstants::SMALL_FONT_MULTIPLIER,
                                ),
                            );
                            ctx.set_style(style);

                            if temp.language != self.settings.language {
                                rust_i18n::set_locale(&temp.language);
                                ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                                    t!("window.title").to_string(),
                                ));
                                ctx.request_repaint();
                            }

                            self.settings = temp.clone();
                            close_dialog = true;
                        }
                    }
                });

            if !self.dialogs.settings.show || close_dialog {
                self.dialogs.settings.temp_settings = None;
            }
        }

        if let Some(index) = self.dialogs.rename_scenario.scenario_index {
            let mut close_window = false;

            if index < self.project.scenarios.len() {
                egui::Window::new(t!("rename_scenario_dialog.title").as_ref())
                    .id(egui::Id::new("rename_scenario_window"))
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(t!("rename_scenario_dialog.new_name").as_ref());

                        let text_id = ui.make_persistent_id("rename_scenario_text_edit");

                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.project.scenarios[index].name)
                                .id(text_id),
                        );

                        ui.memory_mut(|m| {
                            if m.focused().is_none() {
                                m.request_focus(text_id);
                            }
                        });

                        ui.horizontal(|ui| {
                            if ui
                                .button(t!("rename_scenario_dialog.ok").as_ref())
                                .clicked()
                                || (response.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                            {
                                close_window = true;
                            }
                            if ui
                                .button(t!("rename_scenario_dialog.cancel").as_ref())
                                .clicked()
                            {
                                close_window = true;
                            }
                        });
                    });
            } else {
                close_window = true;
            }

            if close_window {
                self.dialogs.rename_scenario.scenario_index = None;
            }
        }

        if self.dialogs.add_variable.show {
            let mut close_window = false;

            let screen_rect = ctx.content_rect();
            let sidebar_width = UiConstants::PROPERTIES_PANEL_WIDTH;
            let sidebar_left = screen_rect.right() - sidebar_width;
            let dialog_width = 300.0;
            let pos = egui::Pos2 {
                x: (sidebar_left - dialog_width - 10.0).max(10.0),
                y: (screen_rect.top() + 500.0).max(10.0),
            };

            egui::Window::new(t!("add_variable_dialog.title").as_ref())
                .id(egui::Id::new("add_variable_window"))
                .collapsible(false)
                .resizable(false)
                .default_pos(pos)
                .show(ctx, |ui| {
                    ui.label(t!("add_variable_dialog.variable_name").as_ref());

                    let name_response =
                        ui.text_edit_singleline(&mut self.dialogs.add_variable.name);

                    ui.label(t!("add_variable_dialog.type").as_ref());
                    egui::ComboBox::from_id_salt("var_type_combo")
                        .selected_text(self.dialogs.add_variable.var_type.as_str())
                        .show_ui(ui, |ui| {
                            for var_type in VariableType::all() {
                                ui.selectable_value(
                                    &mut self.dialogs.add_variable.var_type,
                                    var_type.clone(),
                                    var_type.as_str(),
                                );
                            }
                        });

                    ui.label(t!("add_variable_dialog.scope").as_ref());
                    let selected_scope = if self.dialogs.add_variable.is_global {
                        "Global"
                    } else {
                        "Scenario"
                    };
                    egui::ComboBox::from_id_salt("var_scope_combo")
                        .selected_text(selected_scope)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.dialogs.add_variable.is_global,
                                false,
                                "Scenario",
                            );
                            ui.selectable_value(
                                &mut self.dialogs.add_variable.is_global,
                                true,
                                "Global",
                            );
                        });

                    ui.label(t!("add_variable_dialog.value").as_ref());

                    let value_response = match self.dialogs.add_variable.var_type {
                        VariableType::String => {
                            ui.text_edit_singleline(&mut self.dialogs.add_variable.value)
                        }
                        VariableType::Boolean => {
                            let mut bool_val = self
                                .dialogs
                                .add_variable
                                .value
                                .parse::<bool>()
                                .unwrap_or(false);
                            let response = ui.checkbox(&mut bool_val, "");
                            self.dialogs.add_variable.value = bool_val.to_string();
                            response
                        }
                        VariableType::Number => {
                            let mut num_val = self
                                .dialogs
                                .add_variable
                                .value
                                .parse::<f64>()
                                .unwrap_or(0.0);
                            let response = ui.add(DragValue::new(&mut num_val));
                            self.dialogs.add_variable.value = num_val.to_string();
                            response
                        }
                    };

                    if self.dialogs.add_variable.name.is_empty() {
                        name_response.request_focus();
                    }

                    ui.horizontal(|ui| {
                        let can_add = !self.dialogs.add_variable.name.trim().is_empty();

                        if ui
                            .add_enabled(
                                can_add,
                                egui::Button::new(t!("add_variable_dialog.add").as_ref()),
                            )
                            .clicked()
                            || (can_add
                                && value_response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        {
                            match VariableValue::from_string(
                                &self.dialogs.add_variable.value,
                                &self.dialogs.add_variable.var_type,
                            ) {
                                Ok(value) => {
                                    let var_name = self.dialogs.add_variable.name.trim();
                                    if self.dialogs.add_variable.is_global {
                                        self.global_variables.set(
                                            var_name,
                                            value,
                                            VariableScope::Global,
                                        );
                                    } else {
                                        let scenario = match self.current_scenario_index {
                                            None => &mut self.project.main_scenario,
                                            Some(i) => &mut self.project.scenarios[i],
                                        };

                                        scenario.variables.set(
                                            var_name,
                                            value,
                                            VariableScope::Scenario,
                                        );
                                    };
                                    self.dialogs.add_variable.name.clear();
                                    self.dialogs.add_variable.value.clear();
                                    self.dialogs.add_variable.var_type = VariableType::String;
                                    self.dialogs.add_variable.is_global = false;
                                    close_window = true;
                                }
                                Err(err) => {
                                    self.project.execution_log.push(LogEntry {
                                        timestamp: "".to_string(),
                                        level: LogLevel::Error,
                                        activity: "SYSTEM".to_string(),
                                        message: t!(
                                            "system_messages.invalid_variable_value",
                                            error = err
                                        )
                                        .to_string(),
                                    });
                                }
                            }
                        }
                        if ui
                            .button(t!("add_variable_dialog.cancel").as_ref())
                            .clicked()
                        {
                            self.dialogs.add_variable.name.clear();
                            self.dialogs.add_variable.value.clear();
                            self.dialogs.add_variable.var_type = VariableType::String;
                            self.dialogs.add_variable.is_global = false;
                            close_window = true;
                        }
                    });
                });

            if close_window {
                self.dialogs.add_variable.show = false;
            }
        }

        if self.dialogs.debug.show_debug {
            egui::Window::new("Debug")
                .id(egui::Id::new("debug_window"))
                .open(&mut self.dialogs.debug.show_debug)
                .show(ctx, |ui| {
                    ctx.inspection_ui(ui);
                });
        }

        if let Some(index) = self.dialogs.rename_scenario.scenario_index {
            let mut close_window = false;

            if index < self.project.scenarios.len() {
                egui::Window::new(t!("rename_scenario_dialog.title").as_ref())
                    .id(egui::Id::new("rename_scenario_window"))
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(t!("rename_scenario_dialog.new_name").as_ref());
                        let response =
                            ui.text_edit_singleline(&mut self.project.scenarios[index].name);

                        response.request_focus();

                        ui.horizontal(|ui| {
                            if ui
                                .button(t!("rename_scenario_dialog.ok").as_ref())
                                .clicked()
                                || (response.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                            {
                                close_window = true;
                            }
                            if ui
                                .button(t!("rename_scenario_dialog.cancel").as_ref())
                                .clicked()
                            {
                                close_window = true;
                            }
                        });
                    });
            } else {
                close_window = true;
            }

            if close_window {
                self.dialogs.rename_scenario.scenario_index = None;
            }
        }

        self.render_variable_binding_dialog(ctx);
    }

    pub fn render_variable_binding_dialog(&mut self, ctx: &egui::Context) {
        if !self.dialogs.var_binding_dialog.show {
            return;
        }

        let mut close_window = false;

        let mut dialog_state = self.dialogs.var_binding_dialog.clone();
        let mut error_message = dialog_state.error_message.clone();

        egui::Window::new(t!("variable_binding.title").as_ref())
            .id(egui::Id::new("variable_binding_window"))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(t!("variable_binding.source_variable").as_ref());
                ui.text_edit_singleline(&mut dialog_state.source_var_name);

                ui.label(t!("variable_binding.target_parameter").as_ref());
                ui.text_edit_singleline(&mut dialog_state.target_var_name);

                ui.label(t!("variable_binding.direction").as_ref());
                egui::ComboBox::from_id_salt("param_direction_combo")
                    .selected_text(match dialog_state.direction {
                        rpa_core::node_graph::VariableDirection::In => "In",
                        rpa_core::node_graph::VariableDirection::Out => "Out",
                        rpa_core::node_graph::VariableDirection::InOut => "InOut",
                    })
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(
                                dialog_state.direction
                                    == rpa_core::node_graph::VariableDirection::In,
                                "In",
                            )
                            .clicked()
                        {
                            dialog_state.direction = rpa_core::node_graph::VariableDirection::In;
                        }
                        if ui
                            .selectable_label(
                                dialog_state.direction
                                    == rpa_core::node_graph::VariableDirection::Out,
                                "Out",
                            )
                            .clicked()
                        {
                            dialog_state.direction = rpa_core::node_graph::VariableDirection::Out;
                        }
                        if ui
                            .selectable_label(
                                dialog_state.direction
                                    == rpa_core::node_graph::VariableDirection::InOut,
                                "InOut",
                            )
                            .clicked()
                        {
                            dialog_state.direction = rpa_core::node_graph::VariableDirection::InOut;
                        }
                    });

                if let Some(ref error) = error_message {
                    ui.colored_label(egui::Color32::RED, error);
                }

                ui.horizontal(|ui| {
                    if ui.button(t!("variable_binding.ok").as_ref()).clicked() {
                        error_message = None;

                        if dialog_state.source_var_name.is_empty() {
                            error_message =
                                Some(t!("variable_binding.errors.source_var_required").to_string());
                        }

                        if error_message.is_none() && dialog_state.target_var_name.is_empty() {
                            error_message = Some(
                                t!("variable_binding.errors.target_param_required").to_string(),
                            );
                        }

                        if error_message.is_none() {
                            close_window = true;
                        }
                    }

                    if ui.button(t!("variable_binding.cancel").as_ref()).clicked() {
                        close_window = true;
                    }
                });
            });

        self.dialogs.var_binding_dialog = dialog_state.clone();
        self.dialogs.var_binding_dialog.error_message = error_message;

        if close_window
            && self.dialogs.var_binding_dialog.error_message.is_none()
            && let Some(current_idx) = self.current_scenario_index
            && let Some(node_id) = self.selected_nodes.iter().next().cloned()
        {
            let scenario_id_and_params = {
                if let Some(node) =
                    self.project.scenarios[current_idx].get_node_mut(node_id.clone())
                {
                    if let rpa_core::Activity::CallScenario {
                        scenario_id,
                        parameters,
                    } = &mut node.activity
                    {
                        let _source_var_type = self
                            .project
                            .variables
                            .get(&dialog_state.source_var_name)
                            .map(|v| v.get_type());

                        Some((scenario_id.clone(), parameters.clone()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some((scenario_id, mut parameters)) = scenario_id_and_params {
                // Find the called scenario to add the parameter to it
                if let Some(called_scenario) = self
                    .project
                    .scenarios
                    .iter_mut()
                    .find(|s| s.id == scenario_id)
                {
                    // Create or find the parameter in the called scenario
                    let target_var_exists = called_scenario
                        .parameters
                        .iter()
                        .any(|p| p.var_name == dialog_state.target_var_name);

                    if !target_var_exists {
                        // Parameter doesn't exist, create it
                        called_scenario
                            .parameters
                            .push(rpa_core::node_graph::ScenarioParameter {
                                var_name: dialog_state.target_var_name.clone(),
                                direction: dialog_state.direction,
                            });
                    }

                    let binding = rpa_core::node_graph::VariablesBinding {
                        target_var_name: dialog_state.target_var_name.clone(),
                        source_var_name: dialog_state.source_var_name.clone(),
                        direction: dialog_state.direction,
                        source_scope: None,
                    };

                    if let Some(idx) = dialog_state.editing_index {
                        if idx < parameters.len() {
                            parameters[idx] = binding;
                        }
                    } else {
                        parameters.push(binding);
                    }

                    // Now update the node activity with the modified parameters
                    if let Some(node) =
                        self.project.scenarios[current_idx].get_node_mut(node_id.clone())
                        && let rpa_core::Activity::CallScenario {
                            parameters: node_params,
                            ..
                        } = &mut node.activity
                    {
                        *node_params = parameters;
                    }
                }
            }
        }

        if close_window {
            self.dialogs.var_binding_dialog.show = false;
        }
    }

    pub fn render_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button(t!("menu.file").as_ref(), |ui| {
                    if ui.button(t!("menu.new_project").as_ref()).clicked() {
                        self.project = Project::new(
                            t!("default_values.new_project_name").as_ref(),
                            Variables::new(),
                        );
                        self.current_file = None;
                        self.current_scenario_index = None;
                        self.selected_nodes.clear();
                        self.init_current_scenario_view();
                        ui.close();
                    }
                    if ui.button(t!("menu.open").as_ref()).clicked() {
                        self.open_project();
                        ui.close();
                    }
                    if ui.button(t!("menu.save").as_ref()).clicked() {
                        self.save_project();
                        ui.close();
                    }
                    if ui.button(t!("menu.save_as").as_ref()).clicked() {
                        self.save_project_as();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(t!("menu.exit").as_ref()).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button(t!("menu.edit").as_ref(), |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    if ui.button(t!("menu.settings").as_ref()).clicked() {
                        self.dialogs.settings.show = true;
                        ui.close();
                    }
                    if ui.button("Debug").clicked() {
                        self.dialogs.debug.show_debug = true;
                        ui.close();
                    }
                });

                ui.separator();

                if self.is_executing {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Wait);
                    if ui.button(t!("toolbar.stop").as_ref()).clicked() {
                        self.stop_control.request_stop();
                    }
                } else {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
                    if ui.button(t!("toolbar.run").as_ref()).clicked() {
                        self.is_executing = true;
                        self.execute_project();
                    }
                }

                if ui
                    .add_enabled(
                        !self.is_executing && self.undo_redo.has_undo(&self.project),
                        egui::Button::new("⟲ Undo"),
                    )
                    .clicked()
                {
                    self.undo();
                }

                if ui
                    .add_enabled(
                        !self.is_executing && self.undo_redo.has_redo(&self.project),
                        egui::Button::new("⟳ Redo"),
                    )
                    .clicked()
                {
                    self.redo();
                }

                let fps = ctx.input(|i| 1.0 / i.stable_dt);
                ui.label(format!("FPS: {:.2}", fps));
            });
        });
    }

    pub fn render_left_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left_panel")
            .default_width(UiConstants::LEFT_PANEL_WIDTH)
            .show(ctx, |ui| {
                self.render_scenario_list(ui);
                ui.separator();
                ui.add_space(10.0);
                self.render_activity_buttons(ui, ctx);
            });
    }

    pub fn render_scenario_tab_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.set_min_height(40.0);

            let style = ui.style_mut();
            style.spacing.scroll.bar_width = 4.0;

            egui::ScrollArea::horizontal()
                .id_salt("scenario_tabs")
                .stick_to_right(true)
                .max_width(ui.available_width() - 50.0)
                .vscroll(false)
                .show(ui, |ui| {
                    let mut closed_tab_idx: Option<usize> = None;

                    ui.push_id("main_tab", |ui| {
                        let tab = scenario_tab(
                            ui,
                            ui.id(), // unique because of push_id
                            &self.project.main_scenario.name,
                            self.current_scenario_index.is_none(),
                        );

                        if tab.clicked {
                            self.current_scenario_index = None;
                            self.selected_nodes.clear();
                        }
                    });

                    ui.separator();

                    for &tab_idx in &self.opened_scenarios {
                        ui.push_id(tab_idx, |ui| {
                            let scenario = &self.project.scenarios[tab_idx];

                            let tab = scenario_tab(
                                ui,
                                ui.id(), // scoped ID
                                &scenario.name,
                                self.current_scenario_index == Some(tab_idx),
                            );

                            if tab.close_clicked {
                                closed_tab_idx = Some(tab_idx);
                            } else if tab.clicked {
                                self.current_scenario_index = Some(tab_idx);
                                self.selected_nodes.clear();
                            }
                        });
                    }

                    if let Some(i) = closed_tab_idx {
                        self.opened_scenarios.retain(|&idx| idx != i);

                        if self.current_scenario_index == Some(i) {
                            self.current_scenario_index = None;
                            self.selected_nodes.clear();
                        }
                    }
                });

            ui.add_space(ui.available_width() - 30.0);

            if ui.button(t!("sidebar.new_scenario").as_ref()).clicked() {
                let new_tab_idx = self.project.scenarios.len();
                let name = t!(
                    "default_values.scenario_name",
                    number = self.project.scenarios.len() + 1
                )
                .to_string();

                self.project.scenarios.push(Scenario::new(&name));
                self.undo_redo.add_undo(&self.project);

                self.open_scenario(new_tab_idx)
            }
        });
    }

    pub fn render_scenario_list(&mut self, ui: &mut egui::Ui) {
        ui.heading(t!("sidebar.scenarios").as_ref());
        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt("scenarios_scroll")
            .max_height(UiConstants::LEFT_PANEL_WIDTH)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                if ui
                    .selectable_label(
                        self.current_scenario_index.is_none(),
                        format!("📋 {}", self.project.main_scenario.name),
                    )
                    .clicked()
                {
                    self.current_scenario_index = None;
                    self.selected_nodes.clear();
                }

                let mut to_open: Option<usize> = None;
                let mut to_remove: Option<usize> = None;

                for (i, scenario) in self.project.scenarios.iter().enumerate() {
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(
                                self.current_scenario_index == Some(i),
                                format!("📁 {}", scenario.name),
                            )
                            .clicked()
                        {
                            to_open = Some(i);
                        }

                        if ui.small_button("✏").clicked() {
                            self.dialogs.rename_scenario.scenario_index = Some(i);
                        }

                        if ui.small_button("🗑").clicked() {
                            to_remove = Some(i);
                        }
                    });
                }

                if let Some(i) = to_open {
                    self.open_scenario(i);
                }

                if let Some(i) = to_remove {
                    self.project.scenarios.remove(i);
                    self.undo_redo.add_undo(&self.project);

                    self.opened_scenarios.retain(|&idx| idx != i);

                    for idx in &mut self.opened_scenarios {
                        if *idx > i {
                            *idx -= 1;
                        }
                    }

                    if self.current_scenario_index == Some(i) {
                        self.current_scenario_index = None;
                    } else if let Some(current) = self.current_scenario_index
                        && current > i
                    {
                        self.current_scenario_index = Some(current - 1);
                    }
                }
            });

        if ui.button(t!("sidebar.new_scenario").as_ref()).clicked() {
            let name = t!(
                "default_values.scenario_name",
                number = self.project.scenarios.len() + 1
            )
            .to_string();
            self.project.scenarios.push(Scenario::new(&name));
            self.undo_redo.add_undo(&self.project);
        }
    }

    pub fn render_activity_buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading(t!("sidebar.activities").as_ref());
        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt("activities_scroll")
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                use rpa_core::ActivityMetadata;

                let mut node_to_add: Option<Activity> = None;

                for (category, activities, default_open) in
                    ActivityMetadata::activities_by_category()
                {
                    egui::CollapsingHeader::new(t!(category.translation_key()).as_ref())
                        .default_open(default_open)
                        .show(ui, |ui| {
                            for (metadata, activity) in activities {
                                let response = ui.dnd_drag_source(
                                    egui::Id::new(format!(
                                        "activity_{:?}",
                                        std::mem::discriminant(&activity)
                                    )),
                                    activity.clone(),
                                    |ui| {
                                        let _ = ui.button(t!(metadata.button_key).as_ref());
                                    },
                                );

                                if response.response.clicked() {
                                    if matches!(activity, Activity::CallScenario { .. }) {
                                        if !self.project.scenarios.is_empty() {
                                            let scenario_id = self.project.scenarios[0].id.clone();
                                            node_to_add = Some(Activity::CallScenario {
                                                scenario_id,
                                                parameters: Vec::new(),
                                            });
                                        } else {
                                            self.project.execution_log.push(LogEntry {
                                                timestamp: "".to_string(),
                                                level: LogLevel::Warning,
                                                activity: "SYSTEM".to_string(),
                                                message: t!("system_messages.no_scenarios_warning")
                                                    .to_string(),
                                            });
                                        }
                                    } else {
                                        node_to_add = Some(activity.clone());
                                    }
                                }
                            }
                        });
                }

                if let Some(activity) = node_to_add {
                    let viewport_center = ctx.content_rect().center();
                    let view = self.get_current_scenario_view_mut();
                    let world_pos =
                        ((viewport_center.to_vec2() - view.pan_offset) / view.zoom).to_pos2();
                    let offset = self.get_current_scenario().nodes.len() as f32
                        * UiConstants::NEW_NODE_OFFSET_INCREMENT;
                    let new_node_pos = world_pos + egui::vec2(offset, offset);
                    self.get_current_scenario_mut()
                        .add_node(activity, new_node_pos);
                }
            });
    }

    pub fn render_right_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("properties")
            .default_width(UiConstants::PROPERTIES_PANEL_WIDTH)
            .show(ctx, |ui| {
                let total_height = ui.available_height();
                let properties_height = total_height * 0.5;
                let variables_height = total_height * 0.5;

                self.render_node_properties_section(ui, properties_height);
                ui.add_space(10.0);
                self.render_variables_section(ui, variables_height);
            });
    }

    pub fn render_node_properties_section(&mut self, ui: &mut egui::Ui, max_height: f32) {
        egui::TopBottomPanel::top("node_properties_panel")
            .resizable(true)
            .default_height(max_height)
            .show_inside(ui, |ui| {
                ui.heading(t!("panels.properties").as_ref());
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("properties_scroll")
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        if let Some(node_id) = self.selected_nodes.iter().next().cloned() {
                            let scenarios = self.project.scenarios.clone();
                            let (changed, param_action, activity) = {
                                let scenario = self.get_current_scenario_mut();
                                if let Some(node) = scenario.get_node_mut(node_id.clone()) {
                                    let (changed, param_action) =
                                        canvas::render_node_properties(ui, node, &scenarios);
                                    (changed, param_action, Some(node.activity.clone()))
                                } else {
                                    (false, canvas::ParameterBindingAction::None, None)
                                }
                            };

                            if let Some(activity) = activity {
                                if changed {
                                    self.property_edit_debounce = 0.0;
                                }

                                // Handle parameter binding actions
                                match param_action {
                                    canvas::ParameterBindingAction::Add => {
                                        if let Activity::CallScenario { scenario_id, .. } =
                                            &activity
                                        {
                                            self.dialogs.var_binding_dialog.show = true;
                                            self.dialogs.var_binding_dialog.scenario_id =
                                                scenario_id.clone();
                                            self.dialogs.var_binding_dialog.source_var_name =
                                                String::new();
                                            self.dialogs.var_binding_dialog.target_var_name =
                                                String::new();
                                            self.dialogs.var_binding_dialog.direction =
                                                VariableDirection::In;
                                            self.dialogs.var_binding_dialog.editing_index = None;
                                            self.dialogs.var_binding_dialog.error_message = None;
                                        }
                                    }
                                    canvas::ParameterBindingAction::Edit(idx) => {
                                        if let Activity::CallScenario {
                                            scenario_id,
                                            parameters,
                                        } = &activity
                                            && let Some(binding) = parameters.get(idx)
                                        {
                                            self.dialogs.var_binding_dialog.show = true;
                                            self.dialogs.var_binding_dialog.scenario_id =
                                                scenario_id.clone();
                                            self.dialogs.var_binding_dialog.source_var_name =
                                                binding.source_var_name.clone();
                                            self.dialogs.var_binding_dialog.target_var_name =
                                                binding.target_var_name.clone();
                                            self.dialogs.var_binding_dialog.direction =
                                                binding.direction;
                                            self.dialogs.var_binding_dialog.editing_index =
                                                Some(idx);
                                            self.dialogs.var_binding_dialog.error_message = None;
                                        }
                                    }
                                    canvas::ParameterBindingAction::None => {}
                                }
                            }

                            if self.selected_nodes.len() > 1 {
                                ui.separator();
                                ui.label(
                                    t!("status.nodes_selected", count = self.selected_nodes.len())
                                        .as_ref(),
                                );
                            }
                        } else {
                            ui.vertical_centered(|ui| {
                                ui.add_space(20.0);
                                ui.label(t!("status.no_node_selected").as_ref());
                            });
                        }
                    });
            });
    }

    pub fn render_variables_section(&mut self, ui: &mut egui::Ui, max_height: f32) {
        ui.heading(t!("panels.variables").as_ref());
        ui.separator();

        if ui.button(t!("variables.add_variable").as_ref()).clicked() {
            self.dialogs.add_variable.show = true;
        }

        egui::ScrollArea::vertical()
            .id_salt(format!(
                "variables_scroll_{}",
                self.get_current_scenario_id()
            ))
            .max_height(max_height - 40.0)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add_space(5.0);

                if self.is_executing {
                    if let Some(ref context) = self.execution_context {
                        let ctx = context.read().unwrap();
                        self.render_variables(
                            ui,
                            t!("panels.global_variables").as_ref(),
                            &ctx.global_variables,
                        );
                        let scenario = self.get_current_scenario();
                        let scenario_id = self.get_current_scenario_id();
                        if let Some(runtime_vars) = ctx.find_scenario_variables(scenario_id) {
                            let merged = scenario.variables.merge(runtime_vars);
                            self.render_variables(
                                ui,
                                t!("panels.local_variables").as_ref(),
                                &merged,
                            );
                        } else {
                            self.render_variables(
                                ui,
                                t!("panels.local_variables").as_ref(),
                                &scenario.variables,
                            );
                        }
                    } else {
                        self.render_variables(
                            ui,
                            t!("panels.global_variables").as_ref(),
                            &self.global_variables,
                        );
                        let scenario = self.get_current_scenario();
                        self.render_variables(
                            ui,
                            t!("panels.local_variables").as_ref(),
                            &scenario.variables,
                        );
                    }
                } else {
                    self.render_variables(
                        ui,
                        t!("panels.global_variables").as_ref(),
                        &self.global_variables,
                    );
                    let scenario = self.get_current_scenario();
                    self.render_variables(
                        ui,
                        t!("panels.local_variables").as_ref(),
                        &scenario.variables,
                    );
                }
            });
    }

    pub fn render_variables(
        &self,
        ui: &mut egui::Ui,
        collapsing_header_id: &str,
        variables: &Variables,
    ) {
        egui::CollapsingHeader::new(collapsing_header_id)
            .default_open(true)
            .show(ui, |ui| {
                if self.is_executing || !variables.is_empty() {
                    if !variables.is_empty() {
                        egui::Grid::new(format!(
                            "runtime_vars_grid_{}",
                            self.get_current_scenario_id()
                        ))
                        .striped(true)
                        .spacing([10.0, 4.0])
                        .min_col_width(60.0)
                        .show(ui, |ui| {
                            ui.strong(t!("variables.scope").as_ref());
                            ui.strong(t!("variables.name").as_ref());
                            ui.strong(t!("variables.type").as_ref());
                            ui.strong(t!("variables.value").as_ref());
                            ui.end_row();

                            for (name, value, scope) in variables.iter() {
                                ui.label(if *scope == VariableScope::Global {
                                    "Global"
                                } else {
                                    "Local"
                                });
                                ui.label(name);
                                ui.label(value.get_type().as_str());
                                let value_str = value.to_string();
                                let display_value = if value_str.len() > 20 {
                                    format!("{}...", &value_str[..20])
                                } else {
                                    value_str
                                };
                                ui.label(display_value);
                                ui.end_row();
                            }
                        });
                    } else if self.is_executing {
                        ui.vertical_centered(|ui| {
                            ui.add_space(5.0);
                            ui.label(t!("variables.runtime_waiting").as_ref());
                        });
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(5.0);
                        ui.label(t!("variables.runtime_run").as_ref());
                    });
                }
            });
    }
}
