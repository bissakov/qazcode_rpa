use crate::dialogs::{ElementType, SelectedElement, WindowNode};
use eframe::egui;
use egui_ltreeview::{NodeBuilder, TreeView, TreeViewState};
use uuid::Uuid;

pub fn render_tree(
    ui: &mut egui::Ui,
    root_node: &mut Option<WindowNode>,
    tree_state: &mut TreeViewState<Uuid>,
) -> Option<SelectedElement> {
    let Some(root) = root_node else {
        ui.label("No windows found. Click Refresh to enumerate windows.");
        return None;
    };

    let mut selected = None;

    let (_response, _actions) = TreeView::new(ui.make_persistent_id("ui_explorer_tree"))
        .allow_multi_selection(false)
        .allow_drag_and_drop(false)
        .show_state(ui, tree_state, |mut builder| {
            show_node_recursive(&mut builder, root);
            builder.close_dir();
        });

    if let Some(selected_id) = tree_state.selected().first() {
        selected = build_selected_element(root, *selected_id);
    }

    selected
}

fn show_node_recursive(
    builder: &mut egui_ltreeview::TreeViewBuilder<Uuid>,
    node: &WindowNode,
) {
    match node {
        WindowNode::Window {
            id,
            title,
            children,
            ..
        } => {
            let label = format!(
                "Window: {}",
                if title.is_empty() { "[untitled]" } else { title }
            );
            builder.node(NodeBuilder::dir(*id).label(label));

            for child in children {
                show_node_recursive(builder, child);
            }

            builder.close_dir();
        }
        WindowNode::Control {
            id,
            class,
            text,
            ..
        } => {
            let label = format!(
                "Control: {} [{}]",
                if text.is_empty() { "[no text]" } else { text },
                class
            );
            builder.node(NodeBuilder::leaf(*id).label(label));
        }
    }
}

fn build_selected_element(root: &WindowNode, selected_id: Uuid) -> Option<SelectedElement> {
    match root {
        WindowNode::Window { id, children, .. } => {
            if *id == selected_id {
                return Some(SelectedElement {
                    node_id: *id,
                    element_type: ElementType::Window,
                    window_title: match root {
                        WindowNode::Window { title, .. } => title.clone(),
                        _ => String::new(),
                    },
                    window_class: match root {
                        WindowNode::Window { class, .. } => class.clone(),
                        _ => String::new(),
                    },
                    control_class: None,
                    control_text: None,
                });
            }
            for child in children {
                if let Some(sel) = build_selected_element(child, selected_id) {
                    return Some(sel);
                }
            }
        }
        WindowNode::Control {
            id,
            class,
            text,
            parent_window_title,
            parent_window_class,
        } => {
            if *id == selected_id {
                return Some(SelectedElement {
                    node_id: *id,
                    element_type: ElementType::Control,
                    window_title: parent_window_title.clone(),
                    window_class: parent_window_class.clone(),
                    control_class: Some(class.clone()),
                    control_text: Some(text.clone()),
                });
            }
        }
    }

    None
}
