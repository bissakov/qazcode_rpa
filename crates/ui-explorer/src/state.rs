use std::collections::HashSet;

use shared::NanoId;
use ui_automation::automation::{Element, Rect, find_windows};

use crate::properties::{SelectedElement, WindowNode};

fn is_real_window(window: &Element) -> bool {
    window.visible
        && window.class_name != "ThumbnailDeviceHelperWnd"
        && window.class_name != "PseudoConsoleWindow"
        && window.class_name != "Winit Thread Event Target"
        && window.class_name != "ApplicationFrameWindow"
        && window.class_name != "Windows.UI.Core.CoreWindow"
        && window.class_name != "EdgeUiInputTopWndClass"
        && window.class_name != "DummyDWMListenerWindow"
}

#[derive(Clone, Default)]
pub struct UiExplorerState {
    pub show: bool,
    pub root_node: Option<WindowNode>,
    pub selected_element: Option<SelectedElement>,
    pub is_refreshing: bool,
    pub error_message: Option<String>,
    pub show_hidden_windows: bool,
    pub expanded_nodes: HashSet<NanoId>,
    pub selected_node_id: Option<NanoId>,
}

impl UiExplorerState {
    pub fn new_shown() -> Self {
        Self {
            show: true,
            ..Self::default()
        }
    }
}

impl UiExplorerState {
    pub fn refresh_windows(&mut self) {
        self.is_refreshing = true;
        self.error_message = None;
        self.expanded_nodes.clear();

        match find_windows() {
            Ok(windows) => {
                let mut root_children = Vec::new();

                for window in windows {
                    if !self.show_hidden_windows && !is_real_window(&window) {
                        continue;
                    }

                    root_children.push(WindowNode::Window {
                        id: NanoId::default(),
                        title: window.text.clone(),
                        class: window.class_name.clone(),
                        children: Vec::new(),
                        window_hwnd: window.id.0,
                        bounds: window.bounds,
                    });
                }

                self.root_node = Some(WindowNode::Window {
                    id: NanoId::default(),
                    title: "Windows".to_string(),
                    class: String::new(),
                    children: root_children,
                    window_hwnd: 0,
                    bounds: Rect {
                        left: 0,
                        top: 0,
                        width: 0,
                        height: 0,
                    },
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
