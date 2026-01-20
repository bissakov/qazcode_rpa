use crate::error::Result;
use crate::ui::anchor_panel::AnchorPanel;
use crate::ui::viewer_panel::PdfViewerPanel;
use crate::viewer::state::PdfViewerState;
use eframe::egui;
use std::path::PathBuf;

pub struct PdfViewerWidget {
    viewer_state: PdfViewerState,
    viewer_panel: PdfViewerPanel,
    anchor_panel: AnchorPanel,
}

impl PdfViewerWidget {
    pub fn new() -> Self {
        Self {
            viewer_state: PdfViewerState::default(),
            viewer_panel: PdfViewerPanel::new(),
            anchor_panel: AnchorPanel::default(),
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        self.viewer_panel
            .show(ctx, &mut self.viewer_state, &mut self.anchor_panel);
    }

    pub fn state(&self) -> &PdfViewerState {
        &self.viewer_state
    }

    pub fn state_mut(&mut self) -> &mut PdfViewerState {
        &mut self.viewer_state
    }

    pub fn open_pdf(&mut self, path: PathBuf) -> Result<()> {
        self.viewer_state.open_pdf(path)
    }

    pub fn current_page(&self) -> usize {
        self.viewer_state.current_page
    }

    pub fn total_pages(&self) -> usize {
        self.viewer_state.total_pages
    }

    pub fn goto_page(&mut self, page: usize) -> Result<()> {
        self.viewer_state.goto_page(page)
    }

    pub fn anchor_panel(&self) -> &AnchorPanel {
        &self.anchor_panel
    }

    pub fn anchor_panel_mut(&mut self) -> &mut AnchorPanel {
        &mut self.anchor_panel
    }
}

impl Default for PdfViewerWidget {
    fn default() -> Self {
        Self::new()
    }
}
