use eframe::egui;
use ui_automation::win32::automation::{
    Control, ControlId, Rect, Window, WindowId, control_to_selector, window_to_selector,
};

use shared::NanoId;

#[derive(Clone, Debug)]
pub enum WindowNode {
    Window {
        id: NanoId,
        title: String,
        class: String,
        children: Vec<WindowNode>,
        window_hwnd: isize,
        bounds: Rect,
    },
    Control {
        id: NanoId,
        class: String,
        text: String,
        control_hwnd: isize,
        children: Vec<WindowNode>,
        bounds: Rect,
    },
}

#[derive(Clone)]
pub struct SelectedElement {
    pub element_type: ElementType,
    pub window_title: String,
    pub window_class: String,
    pub window_hwnd: isize,
    pub window_bounds: Rect,
    pub control_class: Option<String>,
    pub control_text: Option<String>,
    pub control_hwnd: Option<isize>,
    pub control_bounds: Option<Rect>,
}

#[derive(Clone, Copy, Debug)]
pub enum ElementType {
    Window,
    Control,
}

pub fn render_properties(ui: &mut egui::Ui, element: &SelectedElement) {
    match element.element_type {
        ElementType::Window => {
            render_window_properties(ui, element);
        }
        ElementType::Control => {
            render_control_properties(ui, element);
        }
    }
}

fn render_window_properties(ui: &mut egui::Ui, element: &SelectedElement) {
    ui.heading("WINDOW PROPERTIES");
    ui.separator();

    egui::Grid::new("window_properties")
        .striped(true)
        .spacing([20.0, 8.0])
        .show(ui, |ui| {
            ui.label("Title:");
            ui.label(&element.window_title);
            ui.end_row();

            ui.label("Class:");
            ui.label(&element.window_class);
            ui.end_row();
        });

    ui.separator();

    let window = Window {
        id: WindowId(element.window_hwnd),
        title: element.window_title.clone(),
        class_name: element.window_class.clone(),
        bounds: element.window_bounds,
        visible: true,
    };

    match window_to_selector(&window) {
        Ok(selector) => {
            ui.label("Selector:");
            ui.label(egui::RichText::new(&selector).monospace());
        }
        Err(e) => {
            ui.label(format!("Selector Error: {}", e));
        }
    }

    ui.separator();

    ui.horizontal(|ui| {
        if ui.button("Outline").clicked() {
            show_element_outline(element, false);
        }
        if ui.button("Focus & Outline").clicked() {
            show_element_outline(element, true);
        }
    });
}

fn render_control_properties(ui: &mut egui::Ui, element: &SelectedElement) {
    ui.heading("CONTROL PROPERTIES");
    ui.separator();

    egui::Grid::new("control_properties")
        .striped(true)
        .spacing([20.0, 8.0])
        .show(ui, |ui| {
            ui.label("Class:");
            ui.label(
                element
                    .control_class
                    .as_ref()
                    .unwrap_or(&"[none]".to_string()),
            );
            ui.end_row();

            ui.label("Text:");
            ui.label(
                element
                    .control_text
                    .as_ref()
                    .unwrap_or(&"[none]".to_string()),
            );
            ui.end_row();
        });

    ui.heading("PARENT WINDOW");
    ui.separator();

    egui::Grid::new("parent_window_properties")
        .striped(true)
        .spacing([20.0, 8.0])
        .show(ui, |ui| {
            ui.label("Title:");
            ui.label(&element.window_title);
            ui.end_row();

            ui.label("Class:");
            ui.label(&element.window_class);
            ui.end_row();
        });

    ui.separator();

    let window = Window {
        id: WindowId(element.window_hwnd),
        title: element.window_title.clone(),
        class_name: element.window_class.clone(),
        bounds: element.window_bounds,
        visible: true,
    };

    if let Some(control_hwnd) = element.control_hwnd {
        let control = Control {
            id: ControlId(control_hwnd),
            class_name: element.control_class.clone().unwrap_or_default(),
            text: element.control_text.clone().unwrap_or_default(),
            bounds: element.control_bounds.unwrap_or(Rect::empty()),
            visible: true,
            enabled: true,
        };

        match control_to_selector(&control, &window) {
            Ok(selector) => {
                ui.label("Selector:");
                ui.label(egui::RichText::new(&selector).monospace());
            }
            Err(e) => {
                ui.label(format!("Selector Error: {}", e));
            }
        }
    } else {
        ui.label("Selector Error: No control hwnd available");
    }

    ui.separator();

    ui.horizontal(|ui| {
        if ui.button("Outline").clicked() {
            show_element_outline(element, false);
        }
        if ui.button("Focus & Outline").clicked() {
            show_element_outline(element, true);
        }
    });
}

fn show_element_outline(element: &SelectedElement, focus: bool) {
    match element.element_type {
        ElementType::Window => {
            let mut window = Window {
                id: WindowId(element.window_hwnd),
                title: element.window_title.clone(),
                class_name: element.window_class.clone(),
                bounds: element.window_bounds,
                visible: true,
            };

            // Refresh to get CURRENT bounds from the actual window
            let _ = window.refresh();

            if focus {
                let _ = window.activate();
            }

            let _ = window.show_overlay();
        }
        ElementType::Control => {
            if let Some(control_hwnd) = element.control_hwnd {
                let mut control = Control {
                    id: ControlId(control_hwnd),
                    class_name: element.control_class.clone().unwrap_or_default(),
                    text: element.control_text.clone().unwrap_or_default(),
                    bounds: element.control_bounds.unwrap_or(Rect::empty()),
                    visible: true,
                    enabled: true,
                };

                // Refresh to get CURRENT bounds from the actual control
                let _ = control.refresh();

                if focus {
                    if !control.is_focused() {
                        let _ = control.focus();
                    }
                }

                let _ = control.show_overlay();
            }
        }
    }
}
