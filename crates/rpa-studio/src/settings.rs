use rpa_core::UiConstants;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    pub target_fps: usize,
    pub font_size: f32,
    pub show_minimap: bool,
    pub allow_node_resize: bool,
    pub language: String,
    pub current_max_entry_size: usize,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            target_fps: UiConstants::DEFAULT_FPS,
            font_size: UiConstants::DEFAULT_FONT_SIZE,
            show_minimap: true,
            allow_node_resize: false,
            language: "en".to_string(),
            current_max_entry_size: UiConstants::DEFAULT_LOG_ENTRIES,
        }
    }
}

#[derive(Default)]
pub struct SettingsDialog {
    pub show: bool,
    pub temp_settings: Option<AppSettings>,
}
