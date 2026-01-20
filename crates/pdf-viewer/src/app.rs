use crate::widget::PdfViewerWidget;
use eframe::egui;

pub struct PdfViewerApp {
    widget: PdfViewerWidget,
}

impl PdfViewerApp {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for PdfViewerApp {
    fn default() -> Self {
        Self {
            widget: PdfViewerWidget::new(),
        }
    }
}

impl eframe::App for PdfViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.widget.show(ctx);
    }
}
