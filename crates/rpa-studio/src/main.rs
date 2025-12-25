mod activity_ext;
mod colors;
mod dialogs;
mod loglevel_ext;
mod ui;

use egui::{DragValue, IconData, Slider};
use egui_extras::{Column, TableBuilder};
use loglevel_ext::LogLevelExt;

rust_i18n::i18n!("locales", fallback = "en");

use eframe::egui;

use dialogs::DialogState;
use rpa_core::{
    Activity, IrBuilder, LogEntry, LogLevel, Node, Project, ProjectFile, Scenario,
    ScenarioValidator, UiConstants, VarEvent, VariableType, VariableValue, Variables,
    execute_project_with_typed_vars, get_timestamp,
};
use rust_i18n::t;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    font_size: f32,
    show_minimap: bool,
    allow_node_resize: bool,
    language: String,
    current_max_entry_size: usize,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
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
        viewport: egui::ViewportBuilder::default()
            .with_maximized(true)
            .with_title(t!("window.title").as_ref())
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        t!("window.title").as_ref(),
        options,
        Box::new(|cc| {
            let app = RpaApp::default();
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

struct RpaApp {
    project: Project,
    is_executing: bool,
    selected_nodes: HashSet<Uuid>,
    current_file: Option<std::path::PathBuf>,
    current_scenario_index: Option<usize>,
    connection_from: Option<(Uuid, usize)>,
    pan_offset: egui::Vec2,
    zoom: f32,
    settings: AppSettings,
    log_receiver: Option<std::sync::mpsc::Receiver<LogEntry>>,
    clipboard: Vec<Node>,
    variables: Variables,
    variable_receiver: Option<std::sync::mpsc::Receiver<VarEvent>>,
    knife_tool_active: bool,
    knife_path: Vec<egui::Pos2>,
    resizing_node: Option<(Uuid, ui::ResizeHandle)>,
    stop_flag: Arc<AtomicBool>,
    dialogs: DialogState,
}

impl Default for RpaApp {
    fn default() -> Self {
        Self {
            project: Project::new("New Project", Variables::new()),
            is_executing: false,
            selected_nodes: std::collections::HashSet::new(),
            current_file: None,
            current_scenario_index: None,
            connection_from: None,
            pan_offset: egui::Vec2::ZERO,
            zoom: 1.0,
            settings: AppSettings::default(),
            log_receiver: None,
            clipboard: Vec::new(),
            variables: Variables::new(),
            variable_receiver: None,
            knife_tool_active: false,
            knife_path: Vec::new(),
            resizing_node: None,
            stop_flag: Arc::new(AtomicBool::new(false)),
            dialogs: DialogState::default(),
        }
    }
}

impl eframe::App for RpaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_execution_updates(ctx);
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
    fn process_execution_updates(&mut self, ctx: &egui::Context) {
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

        if let Some(var_receiver) = self.variable_receiver.as_ref() {
            for event in var_receiver.try_iter() {
                match event {
                    VarEvent::Set { name, value } => {
                        let id = self.variables.id(&name);
                        self.variables.set(id, value);
                    }
                    VarEvent::Remove { name } => {
                        let id = self.variables.id(&name);
                        self.variables.remove(id);
                    }
                    VarEvent::SetId { id, value } => {
                        self.variables.set(id, value);
                    }
                    VarEvent::RemoveId { id } => {
                        self.variables.remove(id);
                    }
                }
            }
            ctx.request_repaint();
        }

        if execution_complete {
            self.is_executing = false;
            self.log_receiver = None;
            self.variable_receiver = None;
        }
    }

    fn render_canvas_panel(&mut self, ctx: &egui::Context) -> (ui::ContextMenuAction, egui::Vec2) {
        egui::CentralPanel::default()
            .show(ctx, |ui| {
                let scenario_name = self.get_current_scenario().name.clone();
                ui.heading(&scenario_name);
                ui.separator();

                let clipboard_empty = self.clipboard.is_empty();
                let show_minimap = self.settings.show_minimap;
                let allow_node_resize = self.settings.allow_node_resize;

                let current_scenario_index = self.current_scenario_index;
                let scenario = match current_scenario_index {
                    None => &mut self.project.main_scenario,
                    Some(i) => &mut self.project.scenarios[i],
                };

                let mut render_state = ui::RenderState {
                    selected_nodes: &mut self.selected_nodes,
                    connection_from: &mut self.connection_from,
                    pan_offset: &mut self.pan_offset,
                    zoom: &mut self.zoom,
                    clipboard_empty,
                    show_minimap,
                    knife_tool_active: &mut self.knife_tool_active,
                    knife_path: &mut self.knife_path,
                    resizing_node: &mut self.resizing_node,
                    allow_node_resize,
                };
                ui::render_node_graph(ui, scenario, &mut render_state)
            })
            .inner
    }

    fn handle_context_menu_action(
        &mut self,
        action: ui::ContextMenuAction,
        mouse_world_pos: egui::Vec2,
    ) {
        match action {
            ui::ContextMenuAction::Copy => {
                self.copy_selected_nodes();
            }
            ui::ContextMenuAction::Paste => {
                self.paste_clipboard_nodes(mouse_world_pos);
            }
            ui::ContextMenuAction::Delete => {
                if !self.selected_nodes.is_empty() {
                    let nodes_to_remove: Vec<_> = self.selected_nodes.iter().copied().collect();
                    let scenario = self.get_current_scenario_mut();
                    for node_id in nodes_to_remove {
                        scenario.remove_node(node_id);
                    }
                    self.selected_nodes.clear();
                }
            }
            ui::ContextMenuAction::SelectAll => {
                let node_ids: Vec<_> = self
                    .get_current_scenario()
                    .nodes
                    .iter()
                    .map(|n| n.id)
                    .collect();
                self.selected_nodes.clear();
                for node_id in node_ids {
                    self.selected_nodes.insert(node_id);
                }
            }
            ui::ContextMenuAction::None => {}
        }
    }

    fn render_console_panel(&mut self, ctx: &egui::Context) {
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

    fn render_dialogs(&mut self, ctx: &egui::Context) {
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
                                    let id = self.project.variables.id(var_name);
                                    self.project.variables.set(id, value);
                                    self.dialogs.add_variable.name.clear();
                                    self.dialogs.add_variable.value.clear();
                                    self.dialogs.add_variable.var_type = VariableType::String;
                                    close_window = true;
                                }
                                Err(err) => {
                                    self.project.execution_log.push(LogEntry {
                                        timestamp: "[00:00.00]".to_string(),
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

        if self.dialogs.debug.show_debug_ir {
            // egui::Window::new("Debug Instructions")
            //     .id(egui::Id::new("debug_instructions_window"))
            //     .open(&mut self.show_debug_ir)
            //     .show(ctx, |ui| {
            //         // let ir_builder = IrBuilder::new(
            //         //     &self.project.main_scenario,
            //         //     &self.project,
            //         //     &validation_result.reachable_nodes,
            //         // );
            //         ScrollArea::vertical()
            //             .auto_shrink([false; 2])
            //             .show(ui, |ui| {
            //                 ui.monospace(format_ir(program));
            //             });
            //     });
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
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let mut handled = false;

        let copy_event = ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy)));
        let paste_event =
            ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Paste(_))));
        let has_selected = !self.selected_nodes.is_empty();
        let no_settings = !self.dialogs.settings.show;
        let no_rename = self.dialogs.rename_scenario.scenario_index.is_none();

        if copy_event && has_selected && no_settings && no_rename {
            self.copy_selected_nodes();
            handled = true;
        }

        let has_clipboard = !self.clipboard.is_empty();

        if paste_event && has_clipboard && no_settings && no_rename {
            let mouse_world_pos = ctx
                .pointer_hover_pos()
                .map(|pos| (pos.to_vec2() - self.pan_offset) / self.zoom)
                .unwrap_or_else(|| {
                    let viewport_center = ctx.content_rect().center();
                    (viewport_center.to_vec2() - self.pan_offset) / self.zoom
                });

            self.paste_clipboard_nodes(mouse_world_pos);
            handled = true;
        }

        if !ctx.wants_keyboard_input() {
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::A)) {
                let node_ids: Vec<_> = self
                    .get_current_scenario()
                    .nodes
                    .iter()
                    .map(|n| n.id)
                    .collect();
                self.selected_nodes.clear();
                for node_id in node_ids {
                    self.selected_nodes.insert(node_id);
                }
                handled = true;
            }

            if ctx.input(|i| i.key_pressed(egui::Key::Delete)) && !self.selected_nodes.is_empty() {
                let nodes_to_remove: Vec<_> = self.selected_nodes.iter().copied().collect();
                let scenario = self.get_current_scenario_mut();
                for node_id in nodes_to_remove {
                    scenario.remove_node(node_id);
                }
                self.selected_nodes.clear();
                handled = true;
            }
        }

        if handled {
            ctx.request_repaint();
        }
    }

    fn render_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button(t!("menu.file").as_ref(), |ui| {
                    if ui.button(t!("menu.new_project").as_ref()).clicked() {
                        self.project = Project::new(
                            t!("default_values.new_project_name").as_ref(),
                            Variables::new(),
                        );
                        self.current_file = None;
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
                        self.stop_flag.store(true, Ordering::Relaxed);
                    }
                } else {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
                    if ui.button(t!("toolbar.run").as_ref()).clicked() {
                        self.is_executing = true;
                        self.execute_project();
                    }
                }
            });
        });
    }

    fn render_left_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left_panel")
            .default_width(UiConstants::LEFT_PANEL_WIDTH)
            .show(ctx, |ui| {
                self.render_scenario_list(ui);
                ui.separator();
                ui.add_space(10.0);
                self.render_activity_buttons(ui, ctx);
            });
    }

    fn render_scenario_list(&mut self, ui: &mut egui::Ui) {
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
                            self.current_scenario_index = Some(i);
                            self.selected_nodes.clear();
                        }

                        if ui.small_button("✏").clicked() {
                            self.dialogs.rename_scenario.scenario_index = Some(i);
                        }

                        if ui.small_button("🗑").clicked() {
                            to_remove = Some(i);
                        }
                    });
                }

                if let Some(i) = to_remove {
                    self.project.scenarios.remove(i);
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
        }
    }

    fn render_activity_buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
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
                                if ui.button(t!(metadata.button_key).as_ref()).clicked() {
                                    if matches!(activity, Activity::CallScenario { .. }) {
                                        if !self.project.scenarios.is_empty() {
                                            let scenario_id = self.project.scenarios[0].id;
                                            node_to_add =
                                                Some(Activity::CallScenario { scenario_id });
                                        } else {
                                            self.project.execution_log.push(LogEntry {
                                                timestamp: "[00:00.00]".to_string(),
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
                    let world_pos =
                        ((viewport_center.to_vec2() - self.pan_offset) / self.zoom).to_pos2();
                    let offset = self.get_current_scenario().nodes.len() as f32
                        * UiConstants::NEW_NODE_OFFSET_INCREMENT;
                    let new_node_pos = world_pos + egui::vec2(offset, offset);
                    self.get_current_scenario_mut()
                        .add_node(activity, new_node_pos);
                }
            });
    }

    fn render_right_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("properties")
            .default_width(UiConstants::PROPERTIES_PANEL_WIDTH)
            .show(ctx, |ui| {
                let total_height = ui.available_height();
                let properties_height = total_height * 0.5;
                let variables_height = total_height * 0.5;

                self.render_node_properties_section(ui, properties_height);
                ui.add_space(10.0);
                ui.separator();
                self.render_variables_section(ui, variables_height);
            });
    }

    fn render_node_properties_section(&mut self, ui: &mut egui::Ui, max_height: f32) {
        ui.heading(t!("panels.properties").as_ref());
        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt("properties_scroll")
            .max_height(max_height - 40.0)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                if let Some(&node_id) = self.selected_nodes.iter().next() {
                    let scenarios: Vec<_> = self.project.scenarios.to_vec();
                    let scenario = self.get_current_scenario_mut();
                    if let Some(node) = scenario.get_node_mut(node_id) {
                        ui::render_node_properties(ui, node, &scenarios);
                    }
                    if self.selected_nodes.len() > 1 {
                        ui.separator();
                        ui.label(
                            t!("status.nodes_selected", count = self.selected_nodes.len()).as_ref(),
                        );
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(t!("status.no_node_selected").as_ref());
                    });
                }
            });
    }

    fn render_variables_section(&mut self, ui: &mut egui::Ui, max_height: f32) {
        ui.heading(t!("panels.variables").as_ref());
        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt("variables_scroll")
            .max_height(max_height - 40.0)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                self.render_initial_variables(ui);
                ui.add_space(5.0);
                self.render_runtime_variables(ui);
            });
    }

    fn render_initial_variables(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new(t!("variables.title").as_ref())
            .default_open(true)
            .show(ui, |ui| {
                let all_vars: Vec<(String, VariableValue)> = self
                    .project
                    .variables
                    .iter()
                    .map(|(name, value)| (name.clone(), value.clone()))
                    .collect();

                if !all_vars.is_empty() {
                    let mut to_remove: Option<String> = None;

                    egui::Grid::new("user_defined_vars_grid")
                        .striped(true)
                        .spacing([10.0, 4.0])
                        .min_col_width(60.0)
                        .show(ui, |ui| {
                            ui.strong(t!("variables.name").as_ref());
                            ui.strong(t!("variables.type").as_ref());
                            ui.strong(t!("variables.value").as_ref());
                            ui.strong("");
                            ui.end_row();

                            for (name, value) in &all_vars {
                                if !matches!(value, VariableValue::Undefined) {
                                    ui.label(name);
                                    ui.label(value.get_type().as_str());
                                    let value_str = format!("{}", value);
                                    let display_value = if value_str.len() > 15 {
                                        format!("{}...", &value_str[..15])
                                    } else {
                                        value_str
                                    };
                                    ui.label(display_value);

                                    if ui.small_button("🗑").clicked() {
                                        to_remove = Some(name.clone());
                                    }
                                    ui.end_row();
                                }
                            }
                        });

                    if let Some(name) = to_remove {
                        let id = self.project.variables.id(&name);
                        self.project.variables.remove(id);
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(5.0);
                        ui.label(t!("variables.no_variables").as_ref());
                    });
                }

                ui.add_space(5.0);
                if ui.button(t!("variables.add_variable").as_ref()).clicked() {
                    self.dialogs.add_variable.show = true;
                }
            });
    }

    fn render_runtime_variables(&self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new(t!("panels.runtime_variables").as_ref())
            .default_open(true)
            .show(ui, |ui| {
                if self.is_executing || !self.variables.is_empty() {
                    if !self.variables.is_empty() {
                        egui::Grid::new("runtime_vars_grid")
                            .striped(true)
                            .spacing([10.0, 4.0])
                            .min_col_width(60.0)
                            .show(ui, |ui| {
                                ui.strong(t!("variables.name").as_ref());
                                ui.strong(t!("variables.type").as_ref());
                                ui.strong(t!("variables.value").as_ref());
                                ui.end_row();

                                for (name, value) in self.variables.iter() {
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
                            ui.spinner();
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

    fn get_current_scenario(&self) -> &Scenario {
        match self.current_scenario_index {
            None => &self.project.main_scenario,
            Some(i) => &self.project.scenarios[i],
        }
    }

    fn get_current_scenario_mut(&mut self) -> &mut Scenario {
        match self.current_scenario_index {
            None => &mut self.project.main_scenario,
            Some(i) => &mut self.project.scenarios[i],
        }
    }

    fn copy_selected_nodes(&mut self) {
        let nodes_to_copy: Vec<_> = self
            .get_current_scenario()
            .nodes
            .iter()
            .filter(|n| self.selected_nodes.contains(&n.id))
            .cloned()
            .collect();
        self.clipboard.clear();
        self.clipboard.extend(nodes_to_copy);
    }

    fn paste_clipboard_nodes(&mut self, mouse_world_pos: egui::Vec2) {
        if self.clipboard.is_empty() {
            return;
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for node in &self.clipboard {
            min_x = min_x.min(node.position.x);
            min_y = min_y.min(node.position.y);
            max_x = max_x.max(node.position.x);
            max_y = max_y.max(node.position.y);
        }

        let clipboard_center = egui::pos2((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);
        let offset = mouse_world_pos - clipboard_center.to_vec2();

        let mut nodes_to_paste = Vec::new();
        let mut new_node_ids = Vec::new();
        let mut old_to_new_id: std::collections::HashMap<uuid::Uuid, uuid::Uuid> =
            std::collections::HashMap::new();

        let clipboard_node_ids: std::collections::HashSet<uuid::Uuid> =
            self.clipboard.iter().map(|n| n.id).collect();

        for node in &self.clipboard {
            let mut new_node = node.clone();
            let new_id = uuid::Uuid::new_v4();
            old_to_new_id.insert(new_node.id, new_id);
            new_node.id = new_id;
            new_node.position = (new_node.position.to_vec2() + offset).to_pos2();
            new_node_ids.push(new_node.id);
            nodes_to_paste.push(new_node);
        }

        self.selected_nodes.clear();
        for node_id in new_node_ids {
            self.selected_nodes.insert(node_id);
        }

        let scenario = self.get_current_scenario_mut();
        scenario.nodes.extend(nodes_to_paste);

        let connections_to_copy: Vec<_> = scenario
            .connections
            .iter()
            .filter(|conn| {
                clipboard_node_ids.contains(&conn.from_node)
                    && clipboard_node_ids.contains(&conn.to_node)
            })
            .cloned()
            .collect();

        for conn in connections_to_copy {
            if let (Some(&new_from), Some(&new_to)) = (
                old_to_new_id.get(&conn.from_node),
                old_to_new_id.get(&conn.to_node),
            ) {
                scenario.add_connection_with_branch(new_from, new_to, conn.branch_type);
            }
        }
    }

    fn execute_project(&mut self) {
        self.project.execution_log.clear();
        self.project.execution_log.push(LogEntry {
            timestamp: "[00:00.00]".to_string(),
            level: LogLevel::Info,
            activity: "SYSTEM".to_string(),
            message: t!("system_messages.execution_start").to_string(),
        });

        self.stop_flag.store(false, Ordering::Relaxed);

        let project = self.project.clone();
        // let initial_vars: indexmap::IndexMap<String, VariableValue> = self
        //     .project
        //     .variables
        //     .iter()
        //     .filter_map(|(name, value)| {
        //         if !matches!(value, VariableValue::Undefined) {
        //             Some((name.clone(), value.clone()))
        //         } else {
        //             None
        //         }
        //     })
        //     .collect();

        let (log_sender, log_receiver) = channel();
        let (var_sender, var_receiver) = channel::<VarEvent>();

        self.log_receiver = Some(log_receiver);
        self.variable_receiver = Some(var_receiver);
        // self.variables.clear();

        let start_time = SystemTime::now();
        let validator = ScenarioValidator::new(&project.main_scenario, &project);
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
            &project.main_scenario,
            &project,
            &validation_result.reachable_nodes,
            &mut self.variables,
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

        let variables = self.variables.clone();

        let stop_flag = Arc::clone(&self.stop_flag);
        std::thread::spawn(move || {
            execute_project_with_typed_vars(
                &project, &log_sender, &var_sender, start_time, &program, variables, stop_flag,
            );
        });
    }

    fn save_project(&mut self) {
        if let Some(path) = &self.current_file {
            self.save_to_file(path.clone());
        } else {
            self.save_project_as();
        }
    }

    fn save_project_as(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("RPA Project", &["rpa"])
            .save_file()
        {
            self.save_to_file(path);
        }
    }

    fn save_to_file(&mut self, path: std::path::PathBuf) {
        let project_file = ProjectFile {
            project: self.project.clone(),
        };

        match serde_json::to_string(&project_file) {
            Ok(json) => match std::fs::write(&path, json) {
                Ok(_) => {
                    self.current_file = Some(path.clone());
                    self.project.execution_log.push(LogEntry {
                        timestamp: "[00:00.00]".to_string(),
                        level: LogLevel::Info,
                        activity: "SYSTEM".to_string(),
                        message: t!("system_messages.project_saved", path = path.display())
                            .to_string(),
                    });
                }
                Err(e) => {
                    self.project.execution_log.push(LogEntry {
                        timestamp: "[00:00.00]".to_string(),
                        level: LogLevel::Error,
                        activity: "SYSTEM".to_string(),
                        message: t!("system_messages.failed_save", error = e).to_string(),
                    });
                }
            },
            Err(e) => {
                self.project.execution_log.push(LogEntry {
                    timestamp: "[00:00.00]".to_string(),
                    level: LogLevel::Error,
                    activity: "SYSTEM".to_string(),
                    message: t!("system_messages.failed_serialize", error = e).to_string(),
                });
            }
        }
    }

    fn open_project(&mut self) {
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
                                timestamp: "[00:00.00]".to_string(),
                                level: LogLevel::Info,
                                activity: "SYSTEM".to_string(),
                                message: t!(
                                    "system_messages.project_loaded",
                                    path = path.display()
                                )
                                .to_string(),
                            });

                            self.project = project_file.project;
                            self.current_scenario_index = None;
                            self.pan_offset = egui::Vec2::ZERO;
                            self.zoom = 1.0;

                            rust_i18n::set_locale(&self.settings.language);
                            self.current_file = Some(path);

                            self.selected_nodes.clear();
                        }
                        Err(e) => {
                            self.project.execution_log.push(LogEntry {
                                timestamp: "[00:00.00]".to_string(),
                                level: LogLevel::Error,
                                activity: "SYSTEM".to_string(),
                                message: t!("system_messages.failed_parse", error = e).to_string(),
                            });
                        }
                    }
                }
                Err(e) => {
                    self.project.execution_log.push(LogEntry {
                        timestamp: "[00:00.00]".to_string(),
                        level: LogLevel::Error,
                        activity: "SYSTEM".to_string(),
                        message: t!("system_messages.failed_read", error = e).to_string(),
                    });
                }
            }
        }
    }
}
