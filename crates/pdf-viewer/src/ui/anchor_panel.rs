use crate::anchor::types::*;
use crate::anchor::{resolver, template};
use crate::viewer::state::PdfViewerState;
use egui;

pub struct AnchorPanel {
    pub template: Option<AnchorTemplate>,
    pub selected_anchor_id: Option<String>,
    pub selected_region_id: Option<String>,
    pub show_create_anchor: bool,
    pub show_create_region: bool,
    pub anchor_form: AnchorForm,
    pub region_form: RegionForm,
    pub last_resolve_results: Option<std::collections::HashMap<String, crate::error::Result<()>>>,
    pub auto_resolve: bool,
    needs_resolve: bool,
    last_page: Option<usize>,
}

pub struct AnchorForm {
    pub name: String,
    pub page: usize,
    pub anchor_type_index: usize,
    pub is_critical: bool,
    pub pos_x: String,
    pub pos_y: String,
    pub content_text: String,
    pub content_position: ContentAnchorPosition,
    pub regex_pattern: String,
    pub regex_position: ContentAnchorPosition,
    pub relative_base_id: Option<String>,
    pub relative_offset_x: String,
    pub relative_offset_y: String,
}

pub struct RegionForm {
    pub name: String,
    pub selected_anchor_ids: Vec<String>,
}

impl Default for AnchorPanel {
    fn default() -> Self {
        Self {
            template: None,
            selected_anchor_id: None,
            selected_region_id: None,
            show_create_anchor: false,
            show_create_region: false,
            anchor_form: AnchorForm::default(),
            region_form: RegionForm::default(),
            last_resolve_results: None,
            auto_resolve: true,
            needs_resolve: false,
            last_page: None,
        }
    }
}

impl Default for AnchorForm {
    fn default() -> Self {
        Self {
            name: String::new(),
            page: 0,
            anchor_type_index: 0,
            is_critical: true,
            pos_x: "0.0".to_string(),
            pos_y: "0.0".to_string(),
            content_text: String::new(),
            content_position: ContentAnchorPosition::TopLeft,
            regex_pattern: String::new(),
            regex_position: ContentAnchorPosition::TopLeft,
            relative_base_id: None,
            relative_offset_x: "0.0".to_string(),
            relative_offset_y: "0.0".to_string(),
        }
    }
}

impl Default for RegionForm {
    fn default() -> Self {
        Self {
            name: String::new(),
            selected_anchor_ids: Vec::new(),
        }
    }
}

pub fn show_anchor_panel(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    state: &mut PdfViewerState,
    panel: &mut AnchorPanel,
) {
    ui.heading("Anchor Template");

    show_template_section(ui, state, panel);

    ui.separator();

    if panel.template.is_some() {
        show_anchors_section(ui, state, panel);

        ui.separator();

        show_regions_section(ui, panel);

        ui.separator();

        show_actions_section(ui, state, panel);

        if panel.last_page != Some(state.current_page) {
            if let (Some(tmpl), Some(doc)) = (&mut panel.template, state.document()) {
                let _ = template::inject_document_anchors(tmpl, doc, state.current_page);
            }
            panel.needs_resolve = true;
            panel.last_page = Some(state.current_page);
        }

        if panel.auto_resolve && panel.needs_resolve {
            if let (Some(tmpl), Some(doc)) = (&mut panel.template, state.document()) {
                let _ = resolver::resolve_all_anchors(tmpl, doc);
                panel.needs_resolve = false;
            }
        }
    }

    show_create_anchor_dialog(ctx, state, panel);
    show_create_region_dialog(ctx, state, panel);
}

fn show_template_section(ui: &mut egui::Ui, state: &PdfViewerState, panel: &mut AnchorPanel) {
    ui.horizontal(|ui| {
        if ui.button("📁 New").clicked() {
            let mut tmpl = template::new_template("New Template".to_string());
            if let Some(doc) = state.document() {
                let _ = template::inject_document_anchors(&mut tmpl, doc, state.current_page);
            }
            panel.template = Some(tmpl);
            panel.needs_resolve = true;
        }

        if ui.button("📂 Load").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .pick_file()
            {
                match template::load_template(&path) {
                    Ok(mut t) => {
                        if let Some(doc) = state.document() {
                            let _ =
                                template::inject_document_anchors(&mut t, doc, state.current_page);
                        }
                        panel.template = Some(t);
                        panel.needs_resolve = true;
                    }
                    Err(_e) => {}
                }
            }
        }

        if panel.template.is_some() && ui.button("💾 Save").clicked() {
            if let Some(tmpl) = &panel.template {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .save_file()
                {
                    let _ = template::save_template(tmpl, &path);
                }
            }
        }
    });

    if let Some(tmpl) = &mut panel.template {
        ui.label(format!("Template: {}", tmpl.name));
        ui.label(format!(
            "Anchors: {} | Regions: {}",
            tmpl.anchors.len(),
            tmpl.regions.len()
        ));

        ui.horizontal(|ui| {
            ui.label("Overlap Threshold:");
            ui.add(egui::Slider::new(&mut tmpl.overlap_threshold, 0.0..=1.0).step_by(0.05));
        });
    }
}

fn show_anchors_section(ui: &mut egui::Ui, state: &PdfViewerState, panel: &mut AnchorPanel) {
    ui.heading("Anchors");

    if ui.button("➕ New Anchor").clicked() {
        panel.show_create_anchor = true;
        panel.anchor_form = AnchorForm {
            page: state.current_page,
            ..Default::default()
        };
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        if let Some(tmpl) = &mut panel.template {
            let mut to_delete = None;
            let current_page = state.current_page;

            let system_anchors: Vec<_> = tmpl
                .anchors
                .iter()
                .filter(|a| a.id.starts_with("doc-") && a.page == current_page)
                .collect();
            let user_anchors: Vec<_> = tmpl
                .anchors
                .iter()
                .filter(|a| !a.id.starts_with("doc-"))
                .collect();

            egui::CollapsingHeader::new(format!("System Anchors (Page {})", current_page + 1))
                .id_salt("system_anchors")
                .default_open(false)
                .show(ui, |ui| {
                    if system_anchors.is_empty() {
                        ui.label("No system anchors on this page");
                    }
                    for anchor in system_anchors {
                        ui.horizontal(|ui| {
                            let (icon, color) = match anchor.status {
                                AnchorStatus::Resolved => ("✔", egui::Color32::GREEN),
                                AnchorStatus::Failed { .. } => ("❌", egui::Color32::RED),
                                AnchorStatus::Unresolved => ("⭕", egui::Color32::GRAY),
                            };
                            ui.colored_label(color, icon);
                            ui.label(&anchor.name);
                        });
                    }
                });

            ui.add_space(4.0);

            egui::CollapsingHeader::new("User Anchors")
                .id_salt("user_anchors")
                .default_open(true)
                .show(ui, |ui| {
                    if user_anchors.is_empty() {
                        ui.label("No user anchors");
                    }
                    for anchor in user_anchors {
                        ui.horizontal(|ui| {
                            let (icon, color) = match anchor.status {
                                AnchorStatus::Resolved => ("✔", egui::Color32::GREEN),
                                AnchorStatus::Failed { .. } => ("❌", egui::Color32::RED),
                                AnchorStatus::Unresolved => ("⭕", egui::Color32::GRAY),
                            };
                            ui.colored_label(color, icon);

                            ui.label(&anchor.name);
                            ui.label(format!("(p{})", anchor.page + 1));

                            let delete_btn = ui.add(egui::Button::new("🗑").small());
                            if delete_btn.clicked() {
                                to_delete = Some(anchor.id.clone());
                            }
                        });
                    }
                });

            if let Some(id) = to_delete {
                tmpl.anchors.retain(|a| a.id != id);
                panel.needs_resolve = true;
            }
        }
    });
}

fn show_regions_section(ui: &mut egui::Ui, panel: &mut AnchorPanel) {
    ui.heading("Regions");

    if ui.button("➕ New Region").clicked() {
        panel.show_create_region = true;
        panel.region_form = RegionForm::default();
    }

    egui::ScrollArea::vertical()
        .max_height(200.0)
        .show(ui, |ui| {
            if let Some(tmpl) = &mut panel.template {
                let mut to_delete = None;

                for (idx, region) in tmpl.regions.iter().enumerate() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.strong(&region.name);

                            if ui.small_button("📋").clicked() {
                                if let Some(text) = &region.extracted_text {
                                    ui.ctx().copy_text(text.clone());
                                }
                            }

                            if ui.small_button("🗑").clicked() {
                                to_delete = Some(idx);
                            }
                        });

                        ui.label(format!("Anchors: {}", region.anchor_ids.len()));

                        if let Some(text) = &region.extracted_text {
                            ui.label(format!(
                                "Text: {}...",
                                text.chars().take(30).collect::<String>()
                            ));
                        }
                    });
                }

                if let Some(idx) = to_delete {
                    tmpl.regions.remove(idx);
                }
            }
        });
}

fn show_actions_section(ui: &mut egui::Ui, state: &PdfViewerState, panel: &mut AnchorPanel) {
    ui.heading("Actions");

    ui.checkbox(&mut panel.auto_resolve, "Auto-resolve anchors");

    if ui.button("🔍 Resolve All Anchors").clicked() {
        if let (Some(tmpl), Some(doc)) = (&mut panel.template, state.document()) {
            let results = resolver::resolve_all_anchors(tmpl, doc);
            panel.last_resolve_results = Some(results);
            panel.needs_resolve = false;
        }
    }

    if ui.button("📤 Extract All Regions").clicked() {
        if let (Some(tmpl), Some(doc)) = (&mut panel.template, state.document()) {
            let _ = resolver::extract_all_regions(tmpl, doc);
        }
    }

    if ui.button("💾 Export Results").clicked() {
        if let Some(tmpl) = &panel.template {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .set_file_name("results.json")
                .save_file()
            {
                let _ = template::export_results(tmpl, &path);
            }
        }
    }
}

fn show_create_anchor_dialog(
    ctx: &egui::Context,
    _state: &PdfViewerState,
    panel: &mut AnchorPanel,
) {
    if !panel.show_create_anchor {
        return;
    }

    egui::Window::new("Create Anchor")
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            let form = &mut panel.anchor_form;

            ui.label("Name:");
            ui.text_edit_singleline(&mut form.name);

            ui.label("Page:");
            ui.add(egui::DragValue::new(&mut form.page).speed(1));

            ui.checkbox(&mut form.is_critical, "Critical (hard error if fails)");

            ui.separator();

            ui.label("Anchor Type:");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut form.anchor_type_index, 0, "Position");
                ui.selectable_value(&mut form.anchor_type_index, 1, "Content");
                ui.selectable_value(&mut form.anchor_type_index, 2, "Regex");
                ui.selectable_value(&mut form.anchor_type_index, 3, "Relative");
            });

            ui.separator();

            match form.anchor_type_index {
                0 => show_position_fields(ui, form),
                1 => show_content_fields(ui, form),
                2 => show_regex_fields(ui, form),
                3 => show_relative_fields(ui, form, panel.template.as_ref()),
                _ => {}
            }

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Create").clicked() {
                    if let Some(anchor) = create_anchor_from_form(form) {
                        if let Some(tmpl) = &mut panel.template {
                            tmpl.anchors.push(anchor);
                        }
                        panel.show_create_anchor = false;
                        panel.needs_resolve = true;
                    }
                }

                if ui.button("Cancel").clicked() {
                    panel.show_create_anchor = false;
                }
            });
        });
}

fn show_position_fields(ui: &mut egui::Ui, form: &mut AnchorForm) {
    ui.label("X:");
    ui.text_edit_singleline(&mut form.pos_x);
    ui.label("Y:");
    ui.text_edit_singleline(&mut form.pos_y);
}

fn show_content_fields(ui: &mut egui::Ui, form: &mut AnchorForm) {
    ui.label("Search Text:");
    ui.text_edit_singleline(&mut form.content_text);
    show_anchor_position_selector(ui, &mut form.content_position);
}

fn show_regex_fields(ui: &mut egui::Ui, form: &mut AnchorForm) {
    ui.label("Regex Pattern:");
    ui.text_edit_singleline(&mut form.regex_pattern);
    show_anchor_position_selector(ui, &mut form.regex_position);
}

fn show_anchor_position_selector(ui: &mut egui::Ui, position: &mut ContentAnchorPosition) {
    ui.label("Anchor Position:");
    egui::ComboBox::from_label("")
        .selected_text(format!("{:?}", position))
        .show_ui(ui, |ui| {
            ui.selectable_value(position, ContentAnchorPosition::TopLeft, "TopLeft");
            ui.selectable_value(position, ContentAnchorPosition::TopRight, "TopRight");
            ui.selectable_value(position, ContentAnchorPosition::BottomLeft, "BottomLeft");
            ui.selectable_value(position, ContentAnchorPosition::BottomRight, "BottomRight");
            ui.selectable_value(position, ContentAnchorPosition::Center, "Center");
        });
}

fn show_relative_fields(
    ui: &mut egui::Ui,
    form: &mut AnchorForm,
    template: Option<&AnchorTemplate>,
) {
    ui.label("Base Anchor:");

    if let Some(t) = template {
        egui::ComboBox::from_label("")
            .selected_text(
                form.relative_base_id
                    .as_ref()
                    .and_then(|id| t.anchors.iter().find(|a| a.id == *id))
                    .map(|a| a.name.as_str())
                    .unwrap_or("Select..."),
            )
            .show_ui(ui, |ui| {
                for anchor in &t.anchors {
                    ui.selectable_value(
                        &mut form.relative_base_id,
                        Some(anchor.id.clone()),
                        &anchor.name,
                    );
                }
            });
    }

    ui.label("Offset X:");
    ui.text_edit_singleline(&mut form.relative_offset_x);
    ui.label("Offset Y:");
    ui.text_edit_singleline(&mut form.relative_offset_y);
}

fn create_anchor_from_form(form: &AnchorForm) -> Option<AnchorPoint> {
    let anchor_type = match form.anchor_type_index {
        0 => {
            let x = form.pos_x.parse().ok()?;
            let y = form.pos_y.parse().ok()?;
            AnchorType::Position { x, y }
        }
        1 => AnchorType::Content {
            search_text: form.content_text.clone(),
            anchor_at: form.content_position,
        },
        2 => AnchorType::Regex {
            pattern: form.regex_pattern.clone(),
            anchor_at: form.regex_position,
        },
        3 => AnchorType::Relative {
            base_anchor_id: form.relative_base_id.clone()?,
            offset_x: form.relative_offset_x.parse().ok()?,
            offset_y: form.relative_offset_y.parse().ok()?,
        },
        _ => return None,
    };

    Some(AnchorPoint {
        id: uuid::Uuid::new_v4().to_string(),
        name: form.name.clone(),
        anchor_type,
        page: form.page,
        resolved_position: None,
        status: AnchorStatus::Unresolved,
        is_critical: form.is_critical,
    })
}

fn show_create_region_dialog(ctx: &egui::Context, state: &PdfViewerState, panel: &mut AnchorPanel) {
    if !panel.show_create_region {
        return;
    }

    egui::Window::new("Create Region")
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            let form = &mut panel.region_form;

            ui.label("Name:");
            ui.text_edit_singleline(&mut form.name);

            ui.separator();

            ui.label("Select Anchors (in order):");
            let current_page = state.current_page;
            ui.label(format!("Current Page: {}", current_page + 1));

            if let Some(tmpl) = &panel.template {
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        let current_page_anchors: Vec<_> = tmpl
                            .anchors
                            .iter()
                            .filter(|a| a.page == current_page)
                            .collect();

                        let system_anchors: Vec<_> = current_page_anchors
                            .iter()
                            .filter(|a| a.id.starts_with("doc-"))
                            .collect();
                        let user_anchors: Vec<_> = current_page_anchors
                            .iter()
                            .filter(|a| !a.id.starts_with("doc-"))
                            .collect();

                        for anchor in &system_anchors {
                            ui.horizontal(|ui| {
                                let selected = form.selected_anchor_ids.contains(&anchor.id);
                                if ui.selectable_label(selected, &anchor.name).clicked() {
                                    if selected {
                                        form.selected_anchor_ids.retain(|id| id != &anchor.id);
                                    } else {
                                        form.selected_anchor_ids.push(anchor.id.clone());
                                    }
                                }
                            });
                        }

                        if !system_anchors.is_empty() && !user_anchors.is_empty() {
                            ui.separator();
                            ui.label("User Anchors:");
                        }

                        for anchor in &user_anchors {
                            ui.horizontal(|ui| {
                                let selected = form.selected_anchor_ids.contains(&anchor.id);
                                if ui.selectable_label(selected, &anchor.name).clicked() {
                                    if selected {
                                        form.selected_anchor_ids.retain(|id| id != &anchor.id);
                                    } else {
                                        form.selected_anchor_ids.push(anchor.id.clone());
                                    }
                                }
                            });
                        }
                    });
            }

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Create").clicked() && form.selected_anchor_ids.len() >= 3 {
                    if let Some(tmpl) = &mut panel.template {
                        tmpl.regions.push(AnchorRegion {
                            id: uuid::Uuid::new_v4().to_string(),
                            name: form.name.clone(),
                            anchor_ids: form.selected_anchor_ids.clone(),
                            extracted_text: None,
                            status: RegionStatus::Failed,
                        });
                    }
                    panel.show_create_region = false;
                }

                if ui.button("Cancel").clicked() {
                    panel.show_create_region = false;
                }
            });
        });
}
