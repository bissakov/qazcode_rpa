use crate::dialogs::{ElementType, SelectedElement};
use eframe::egui;

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
    ui.label("Note: Selector generation requires live Window object (refresh needed)");
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
    ui.label("Note: Selector generation requires live Control object (refresh needed)");
}
