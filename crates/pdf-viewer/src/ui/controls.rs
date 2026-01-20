use crate::constants::*;
use crate::viewer::state::PdfViewerState;
use eframe::egui;
use std::path::PathBuf;

pub fn show_controls(
    ui: &mut egui::Ui,
    state: &mut PdfViewerState,
    pending_file: &mut Option<PathBuf>,
) {
    ui.horizontal(|ui| {
        if ui.button("⏮ Open File").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("PDF", &["pdf"])
                .pick_file()
            {
                *pending_file = Some(path);
            }
        }

        ui.separator();

        if ui.button("⬅ Prev").clicked() {
            state.prev_page();
        }

        if ui.button("Next ➡").clicked() {
            state.next_page();
        }

        ui.separator();

        let input_response = ui.text_edit_singleline(&mut state.page_input_text);
        if input_response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Ok(page_num) = state.page_input_text.parse::<usize>() {
                let _ = state.goto_page(page_num.saturating_sub(1));
            }
        }
        ui.label(format!("/ {}", state.total_pages));

        ui.separator();

        if ui.button("🔍-").clicked() {
            state.set_zoom(state.zoom / ZOOM_BUTTON_FACTOR);
        }

        let mut zoom_pct = (state.zoom * 100.0) as i32;
        if ui
            .add(
                egui::Slider::new(
                    &mut zoom_pct,
                    (MIN_ZOOM * 100.0) as i32..=(MAX_ZOOM * 100.0) as i32,
                )
                .text("%"),
            )
            .changed()
        {
            state.set_zoom(zoom_pct as f32 / 100.0);
        }

        if ui.button("🔍+").clicked() {
            state.set_zoom(state.zoom * ZOOM_BUTTON_FACTOR);
        }

        ui.separator();

        let button_show_words_text = if state.show_word_boundaries {
            "📝 Hide Words"
        } else {
            "📝 Show Words"
        };

        if ui.button(button_show_words_text).clicked() {
            state.toggle_word_boundaries();
        }

        let button_show_anchors_text = if state.show_anchors {
            "📝 Hide Anchors"
        } else {
            "📝 Show Anchors"
        };

        if ui.button(button_show_anchors_text).clicked() {
            state.toggle_anchors();
        }
    });
}
