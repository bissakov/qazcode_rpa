use super::{canvas, controls};
use crate::ui::anchor_panel::AnchorPanel;
use crate::viewer::state::PdfViewerState;
use eframe::egui;
use std::path::PathBuf;

pub struct PdfViewerPanel {
    show_controls: bool,
    pending_file: Option<PathBuf>,
}

impl PdfViewerPanel {
    pub fn new() -> Self {
        Self {
            show_controls: true,
            pending_file: None,
        }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        mut state: &mut PdfViewerState,
        mut anchor_panel: &mut AnchorPanel,
    ) {
        egui::SidePanel::left("anchor_panel")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                crate::ui::anchor_panel::show_anchor_panel(ctx, ui, &mut state, &mut anchor_panel);
            });

        if let Some(path) = self.pending_file.take() {
            match state.open_pdf(path.clone()) {
                Ok(_) => {
                    eprintln!("Loaded PDF: {:?}", path);
                    if let (Some(tmpl), Some(doc)) = (&mut anchor_panel.template, state.document())
                    {
                        let _ = crate::anchor::template::inject_document_anchors(
                            tmpl,
                            doc,
                            state.current_page,
                        );
                    }
                }
                Err(e) => eprintln!("Failed to load PDF: {}", e),
            }
        }

        egui::TopBottomPanel::bottom("stats_panel").show(ctx, |ui| {
            let stats = state.get_cache_stats();
            canvas::display_cache_stats(ui, stats);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.show_controls {
                controls::show_controls(ui, state, &mut self.pending_file);
                ui.separator();
            }

            canvas::show_canvas(ui, state, anchor_panel);
        });
    }

    pub fn toggle_controls(&mut self) {
        self.show_controls = !self.show_controls;
    }
}

impl Default for PdfViewerPanel {
    fn default() -> Self {
        Self::new()
    }
}
