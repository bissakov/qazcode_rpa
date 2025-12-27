use egui::StrokeKind;

pub struct ScenarioTabEvent {
    pub clicked: bool,
    pub close_clicked: bool,
}

pub fn scenario_tab(
    ui: &mut egui::Ui,
    id: egui::Id,
    title: &str,
    selected: bool,
) -> ScenarioTabEvent {
    let padding = egui::vec2(10.0, 6.0);
    let close_size = 14.0;

    let font_id = egui::TextStyle::Button.resolve(ui.style());
    let galley =
        ui.fonts_mut(|f| f.layout_no_wrap(title.to_owned(), font_id, ui.visuals().text_color()));

    let desired_size = galley.size() + padding * 2.0 + egui::vec2(close_size + 6.0, 0.0);

    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    let visuals = ui.style().interact(&response);

    let bg = if selected {
        ui.visuals().selection.bg_fill
    } else {
        visuals.bg_fill
    };

    ui.painter().rect(
        rect,
        egui::CornerRadius::same(6),
        bg,
        visuals.bg_stroke,
        StrokeKind::Middle,
    );

    ui.painter()
        .galley(rect.min + padding, galley, visuals.text_color());

    let close_rect = egui::Rect::from_center_size(
        egui::pos2(rect.max.x - padding.x - close_size * 0.5, rect.center().y),
        egui::vec2(close_size, close_size),
    );

    let close_resp = ui.interact(close_rect, id.with("close"), egui::Sense::click());

    ui.painter().text(
        close_rect.center(),
        egui::Align2::CENTER_CENTER,
        "✖",
        egui::TextStyle::Small.resolve(ui.style()),
        if close_resp.hovered() {
            ui.visuals().error_fg_color
        } else {
            visuals.text_color()
        },
    );

    ScenarioTabEvent {
        clicked: response.clicked(),
        close_clicked: close_resp.clicked(),
    }
}
