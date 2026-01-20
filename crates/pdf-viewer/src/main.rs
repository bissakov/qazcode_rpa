use eframe::egui;
use pdf_viewer::app::PdfViewerApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        vsync: true,
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_maximized(true)
            .with_title("PDF Viewer"),
        ..Default::default()
    };

    eframe::run_native(
        "PDF Viewer",
        options,
        Box::new(|_cc| Ok(Box::new(PdfViewerApp::default()))),
    )
}
