use crate::render::render_ui_explorer_content;
use crate::state::UiExplorerState;

#[derive(Default)]
pub struct UiExplorerApp {
    state: UiExplorerState,
}

impl UiExplorerApp {
    pub fn new() -> Self {
        Self::default()
    }
}

impl eframe::App for UiExplorerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.state.root_node.is_none() {
            self.state.refresh_windows();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("UI Explorer");
            ui.separator();
            render_ui_explorer_content(ui, &mut self.state);
        });
    }
}
