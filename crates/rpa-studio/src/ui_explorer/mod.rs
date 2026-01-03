pub mod properties;
pub mod window_tree;

use crate::dialogs::{UiExplorerDialog, WindowNode};
use ui_automation::{Control, Window, find_windows};
use uuid::Uuid;

impl UiExplorerDialog {
    pub fn refresh_windows(&mut self) {
        self.is_refreshing = true;
        self.error_message = None;
        self.tree_state = Default::default();

        match find_windows() {
            Ok(windows) => {
                let mut root_children = Vec::new();

                for window in windows {
                    let controls = match get_window_child_controls(&window) {
                        Ok(ctrls) => ctrls
                            .into_iter()
                            .map(|c| build_control_node(c, &window))
                            .collect(),
                        Err(_) => Vec::new(),
                    };

                    root_children.push(WindowNode::Window {
                        id: Uuid::new_v4(),
                        title: window.title.clone(),
                        class: window.class_name.clone(),
                        children: controls,
                    });
                }

                self.root_node = Some(WindowNode::Window {
                    id: Uuid::new_v4(),
                    title: "Windows".to_string(),
                    class: String::new(),
                    children: root_children,
                });
                self.selected_element = None;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to enumerate windows: {}", e));
            }
        }

        self.is_refreshing = false;
    }
}

pub fn get_window_child_controls(_window: &Window) -> Result<Vec<Control>, String> {
    Ok(Vec::new())
}

pub fn build_control_node(control: Control, parent_window: &Window) -> WindowNode {
    WindowNode::Control {
        id: Uuid::new_v4(),
        class: control.class_name.clone(),
        text: control.text.clone(),
        parent_window_title: parent_window.title.clone(),
        parent_window_class: parent_window.class_name.clone(),
    }
}
