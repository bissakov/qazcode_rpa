use crate::properties::{ElementType, SelectedElement, WindowNode};
use eframe::egui;
use shared::NanoId;
use std::collections::HashSet;
use ui_automation::{
    automation::{Control, ControlId, WindowId, find_controls_in_window},
    win32::automation::Rect,
};

pub fn get_window_controls_by_hwnd(window_hwnd: isize) -> Result<Vec<Control>, String> {
    let window_id = WindowId(window_hwnd);
    find_controls_in_window(window_id.as_hwnd()).map_err(|e| e.to_string())
}

pub fn get_control_child_controls(control_hwnd: isize) -> Result<Vec<Control>, String> {
    let control_id = ControlId(control_hwnd);
    find_controls_in_window(control_id.as_hwnd()).map_err(|e| e.to_string())
}

pub fn build_control_node(control: Control) -> WindowNode {
    WindowNode::Control {
        id: NanoId::default(),
        class: control.class_name.clone(),
        text: control.text.clone(),
        control_hwnd: control.id.0,
        bounds: control.bounds,
        children: Vec::new(),
    }
}

pub fn render_tree(
    ui: &mut egui::Ui,
    root_node: &mut Option<WindowNode>,
    expanded_nodes: &mut HashSet<NanoId>,
    selected_node_id: Option<NanoId>,
) -> Option<(NanoId, SelectedElement)> {
    let Some(root) = root_node else {
        ui.label("No windows found. Click Refresh to enumerate windows.");
        return None;
    };

    let mut selected = None;

    if let WindowNode::Window {
        children,
        window_hwnd,
        ..
    } = root
    {
        for child in children {
            if let Some(sel) = render_node(
                ui,
                child,
                expanded_nodes,
                selected_node_id.clone(),
                "",
                "",
                *window_hwnd,
            ) {
                selected = Some(sel);
            }
        }
    }

    selected
}

fn render_node(
    ui: &mut egui::Ui,
    node: &mut WindowNode,
    expanded_nodes: &mut HashSet<NanoId>,
    selected_node_id: Option<NanoId>,
    parent_window_title: &str,
    parent_window_class: &str,
    parent_window_hwnd: isize,
) -> Option<(NanoId, SelectedElement)> {
    match node {
        WindowNode::Window {
            id,
            title,
            class,
            children,
            window_hwnd,
            bounds,
        } => {
            let title_str = title.as_str();

            let label = format!(
                "Window: {}",
                if title_str.is_empty() {
                    "[untitled]"
                } else {
                    title_str
                }
            );

            let is_selected = Some(id.clone()) == selected_node_id;
            let rich_label = if is_selected {
                egui::RichText::new(format!("🔹 {}", label)).strong()
            } else {
                egui::RichText::new(label)
            };

            let title_clone = title.clone();
            let class_clone = class.clone();
            let window_hwnd_copy = *window_hwnd;

            let response = egui::CollapsingHeader::new(rich_label)
                .id_salt(id.clone())
                .default_open(false)
                .show(ui, |ui| {
                    if children.is_empty() {
                        load_window_controls(window_hwnd, children);
                    }

                    let mut selected = None;
                    for child in children {
                        if let Some(sel) = render_node(
                            ui,
                            child,
                            expanded_nodes,
                            selected_node_id.clone(),
                            &title_clone,
                            &class_clone,
                            window_hwnd_copy,
                        ) {
                            selected = Some(sel);
                        }
                    }
                    selected
                });

            if response.header_response.clicked() {
                if expanded_nodes.contains(id) {
                    expanded_nodes.remove(id);
                } else {
                    expanded_nodes.insert(id.clone());
                }
                return Some((
                    id.clone(),
                    SelectedElement {
                        element_type: ElementType::Window,
                        window_title: title.clone(),
                        window_class: class.clone(),
                        window_hwnd: *window_hwnd,
                        window_bounds: *bounds,
                        control_class: None,
                        control_text: None,
                        control_hwnd: None,
                        control_bounds: None,
                    },
                ));
            }

            response.body_returned.flatten()
        }
        WindowNode::Control {
            id,
            class,
            text,
            control_hwnd,
            children,
            bounds,
        } => {
            let text_str = text.as_str();
            let class_str = class.as_str();

            let label = format!(
                "Control: {} [{}]",
                if text_str.is_empty() {
                    "[no text]"
                } else {
                    text_str
                },
                class_str
            );

            let is_selected = Some(id.clone()) == selected_node_id;
            let rich_label = if is_selected {
                egui::RichText::new(format!("🔹 {}", label)).strong()
            } else {
                egui::RichText::new(label)
            };

            let response = egui::CollapsingHeader::new(rich_label)
                .id_salt(id.clone())
                .default_open(false)
                .show(ui, |ui| {
                    if children.is_empty() {
                        load_control_children(control_hwnd, children);
                    }

                    let mut selected = None;
                    for child in children {
                        if let Some(sel) = render_node(
                            ui,
                            child,
                            expanded_nodes,
                            selected_node_id.clone(),
                            parent_window_title,
                            parent_window_class,
                            parent_window_hwnd,
                        ) {
                            selected = Some(sel);
                        }
                    }
                    selected
                });

            if response.header_response.clicked() {
                if expanded_nodes.contains(id) {
                    expanded_nodes.remove(id);
                } else {
                    expanded_nodes.insert(id.clone());
                }
                return Some((
                    id.clone(),
                    SelectedElement {
                        element_type: ElementType::Control,
                        window_title: parent_window_title.to_string(),
                        window_class: parent_window_class.to_string(),
                        window_hwnd: parent_window_hwnd,
                        window_bounds: Rect {
                            left: 0,
                            top: 0,
                            width: 0,
                            height: 0,
                        },
                        control_class: Some(class.clone()),
                        control_text: Some(text.clone()),
                        control_hwnd: Some(*control_hwnd),
                        control_bounds: Some(*bounds),
                    },
                ));
            }

            response.body_returned.flatten()
        }
    }
}

fn load_window_controls(window_hwnd: &isize, children: &mut Vec<WindowNode>) {
    if let Ok(ctrls) = get_window_controls_by_hwnd(*window_hwnd) {
        *children = ctrls.into_iter().map(build_control_node).collect();
    }
}

fn load_control_children(control_hwnd: &isize, children: &mut Vec<WindowNode>) {
    if let Ok(ctrls) = get_control_child_controls(*control_hwnd) {
        *children = ctrls.into_iter().map(build_control_node).collect();
    }
}
