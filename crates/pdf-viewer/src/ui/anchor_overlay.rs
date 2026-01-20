use crate::anchor::types::*;
use crate::ui::anchor_panel::AnchorPanel;
use crate::viewer::state::PdfViewerState;
use egui;
use image::RgbImage;

pub fn render_anchor_overlay(
    ui: &mut egui::Ui,
    state: &mut PdfViewerState,
    panel: &AnchorPanel,
    viewport_rect: egui::Rect,
    rendered_image: &RgbImage,
) {
    let Some(template) = &panel.template else {
        return;
    };

    let current_page = state.current_page;
    let render_width = rendered_image.width() as f32;
    let render_height = rendered_image.height() as f32;
    let zoom = state.zoom;
    let pan_offset = state.pan_offset;

    let word_tokens = match state.get_word_tokens_for_current_page() {
        Some(tokens) => tokens,
        None => return,
    };

    let page_width = word_tokens.page_width;
    let page_height = word_tokens.page_height;

    let painter = ui.painter();

    for region in &template.regions {
        if let Some(polygon_vertices) = get_region_screen_vertices(
            region,
            template,
            current_page,
            page_width,
            page_height,
            render_width,
            render_height,
            zoom,
            pan_offset,
            viewport_rect.min,
        ) {
            render_polygon(painter, &polygon_vertices, region.status);
            render_region_label(painter, &polygon_vertices, &region.name);
        }
    }

    for anchor in &template.anchors {
        if anchor.page != current_page {
            continue;
        }

        if let Some(pos) = anchor.resolved_position {
            let screen_pos = pdf_to_screen_pos(
                pos,
                page_width,
                page_height,
                render_width,
                render_height,
                zoom,
                pan_offset,
                viewport_rect.min,
            );

            render_anchor_marker(painter, screen_pos, anchor);
        }
    }
}

fn get_region_screen_vertices(
    region: &AnchorRegion,
    template: &AnchorTemplate,
    current_page: usize,
    page_width: f32,
    page_height: f32,
    render_width: f32,
    render_height: f32,
    zoom: f32,
    pan_offset: egui::Vec2,
    viewport_min: egui::Pos2,
) -> Option<Vec<egui::Pos2>> {
    let mut vertices = Vec::new();

    for anchor_id in &region.anchor_ids {
        let anchor = template.anchors.iter().find(|a| a.id == *anchor_id)?;
        if anchor.page != current_page {
            return None;
        }
        let pos = anchor.resolved_position?;
        let screen_pos = pdf_to_screen_pos(
            pos,
            page_width,
            page_height,
            render_width,
            render_height,
            zoom,
            pan_offset,
            viewport_min,
        );
        vertices.push(screen_pos);
    }

    Some(vertices)
}

fn pdf_to_screen_pos(
    (x, y): (f32, f32),
    page_width: f32,
    page_height: f32,
    render_width: f32,
    render_height: f32,
    zoom: f32,
    pan_offset: egui::Vec2,
    viewport_min: egui::Pos2,
) -> egui::Pos2 {
    let norm_x = x / page_width;
    let norm_y = 1.0 - (y / page_height);

    let render_x = norm_x * render_width;
    let render_y = norm_y * render_height;

    let zoomed_x = render_x * zoom;
    let zoomed_y = render_y * zoom;

    let screen_x = viewport_min.x + pan_offset.x + zoomed_x;
    let screen_y = viewport_min.y + pan_offset.y + zoomed_y;

    egui::pos2(screen_x, screen_y)
}

fn render_polygon(painter: &egui::Painter, vertices: &[egui::Pos2], status: RegionStatus) {
    let fill_color = match status {
        RegionStatus::Resolved => egui::Color32::from_rgba_premultiplied(0, 255, 0, 5),
        RegionStatus::Failed => egui::Color32::from_rgba_premultiplied(255, 0, 0, 5),
    };

    let stroke_color = match status {
        RegionStatus::Resolved => egui::Color32::GREEN,
        RegionStatus::Failed => egui::Color32::RED,
    };

    painter.add(egui::Shape::convex_polygon(
        vertices.to_vec(),
        fill_color,
        egui::Stroke::new(2.0, stroke_color),
    ));
}

fn render_region_label(painter: &egui::Painter, vertices: &[egui::Pos2], name: &str) {
    let n = vertices.len() as f32;
    let centroid = egui::pos2(
        vertices.iter().map(|p| p.x).sum::<f32>() / n,
        vertices.iter().map(|p| p.y).sum::<f32>() / n,
    );

    painter.text(
        centroid,
        egui::Align2::CENTER_CENTER,
        name,
        egui::FontId::proportional(12.0),
        egui::Color32::WHITE,
    );
}

fn render_anchor_marker(painter: &egui::Painter, pos: egui::Pos2, anchor: &AnchorPoint) {
    let radius = 6.0;

    let color = match anchor.status {
        AnchorStatus::Resolved => egui::Color32::GREEN,
        AnchorStatus::Failed { .. } => egui::Color32::RED,
        AnchorStatus::Unresolved => egui::Color32::YELLOW,
    };

    painter.circle_filled(pos, radius, color);
    painter.circle_stroke(pos, radius, egui::Stroke::new(2.0, egui::Color32::WHITE));

    painter.text(
        pos + egui::vec2(radius + 4.0, 0.0),
        egui::Align2::LEFT_CENTER,
        &anchor.name,
        egui::FontId::proportional(10.0),
        egui::Color32::CYAN,
    );
}
