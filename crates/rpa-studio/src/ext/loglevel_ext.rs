use rpa_core::log::LogLevel;

pub trait LogLevelExt {
    fn get_color(&self) -> egui::Color32;
}

impl LogLevelExt for LogLevel {
    fn get_color(&self) -> egui::Color32 {
        match self {
            LogLevel::Info => egui::Color32::from_rgb(200, 200, 200),
            LogLevel::Warning => egui::Color32::from_rgb(255, 200, 0),
            LogLevel::Error => egui::Color32::from_rgb(255, 100, 100),
            LogLevel::Debug => egui::Color32::from_rgb(200, 255, 0),
        }
    }
}
