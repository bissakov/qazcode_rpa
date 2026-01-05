use eframe::egui;
use egui::IconData;
use ui_explorer::app::UiExplorerApp;

fn load_icon() -> IconData {
    let bytes = include_bytes!("../../../resources/icon.ico");

    let img = match image::load_from_memory(bytes) {
        Ok(img) => img.to_rgba8(),
        Err(_) => return IconData::default(),
    };

    let (w, h) = img.dimensions();

    IconData {
        rgba: img.into_raw(),
        width: w,
        height: h,
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        vsync: true,
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_maximized(false)
            .with_title("UI Explorer")
            .with_inner_size((1024.0, 768.0))
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "UI Explorer",
        options,
        Box::new(|_cc| {
            let app = UiExplorerApp::default();
            Ok(Box::new(app))
        }),
    )
}
