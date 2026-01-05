use crate::{properties::render_properties, state::UiExplorerState, window_tree::render_tree};

pub fn render_ui_explorer_content(ui: &mut egui::Ui, state: &mut UiExplorerState) {
    ui.horizontal(|ui| {
        if ui.button("🔄 Refresh").clicked() {
            state.refresh_windows();
        }

        let prev_show_hidden = state.show_hidden_windows;
        ui.checkbox(&mut state.show_hidden_windows, "Show hidden windows");

        // If checkbox state changed, refresh the tree
        if state.show_hidden_windows != prev_show_hidden {
            state.refresh_windows();
        }

        if let Some(err) = &state.error_message {
            ui.label(format!("❌ {}", err));
        }

        if state.is_refreshing {
            ui.label("⏳ Refreshing...");
        }
    });

    ui.separator();

    egui::SidePanel::left("ui_explorer_side_panel")
        .resizable(true)
        .default_width(ui.available_width() * 0.4)
        .show_inside(ui, |ui| {
            egui::ScrollArea::both()
                .max_width(ui.available_width())
                .show(ui, |ui| {
                    if let Some((selected_id, selected_element)) = render_tree(
                        ui,
                        &mut state.root_node,
                        &mut state.expanded_nodes,
                        state.selected_node_id.clone(),
                    ) {
                        state.selected_node_id = Some(selected_id);
                        state.selected_element = Some(selected_element);
                    }
                });
        });

    egui::CentralPanel::default().show_inside(ui, |ui| {
        if let Some(ref element) = state.selected_element {
            render_properties(ui, element);
        } else {
            ui.label("Select an element to see properties");
        }
    });
}
