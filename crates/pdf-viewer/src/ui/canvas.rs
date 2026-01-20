use crate::constants::*;
use crate::ui::anchor_overlay;
use crate::ui::anchor_panel::AnchorPanel;
use crate::viewer::state::PdfViewerState;
use crate::viewer::text_analyzer::PdfWordToken;
use eframe::egui;
use image::RgbImage;
use pdfium_render::prelude::PdfRect;
use std::sync::Arc;

pub fn show_canvas(ui: &mut egui::Ui, state: &mut PdfViewerState, anchor_panel: &mut AnchorPanel) {
    if !state.is_document_loaded() {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.label("Open a PDF file to view");
        });
        return;
    }

    let viewport_rect = ui.available_rect_before_wrap();
    let viewport_size = viewport_rect.size();
    let page_num = state.current_page;

    let rendered_image = {
        let cache = state.cache_mut();

        if let Some(cached) = cache.get_page(page_num) {
            cached
        } else {
            match render_page(state, page_num, viewport_rect) {
                Ok(image) => {
                    let image_arc = Arc::new(image);
                    state.cache_mut().put_page(page_num, image_arc.clone());
                    image_arc
                }
                Err(e) => {
                    ui.label(format!("Failed to render page: {}", e));
                    return;
                }
            }
        }
    };

    let base_image_size = egui::Vec2::new(
        rendered_image.width() as f32,
        rendered_image.height() as f32,
    );

    handle_input(ui, state, viewport_rect, base_image_size);

    state.clamp_pan_bounds(viewport_size, base_image_size);

    render_transformed_image(ui, state, viewport_rect, rendered_image.clone());

    render_word_boundaries_overlay(ui, state, viewport_rect, &rendered_image);

    if state.show_anchors {
        anchor_overlay::render_anchor_overlay(
            ui,
            state,
            anchor_panel,
            viewport_rect,
            &rendered_image,
        );
    }
}

fn render_page(
    state: &PdfViewerState,
    page_num: usize,
    available_rect: egui::Rect,
) -> crate::error::Result<RgbImage> {
    let doc = state
        .document()
        .ok_or(crate::error::PdfError::RenderFailed(
            "No document loaded".into(),
        ))?;

    let page_info = doc.get_page_info(page_num as u16)?;
    let aspect_ratio = page_info.aspect_ratio();

    let render_width = (available_rect.width() as u32).max(100).min(4096);
    let render_height = ((render_width as f32 / aspect_ratio) as u32)
        .max(100)
        .min(4096);

    doc.render_page_to_image(page_num, render_width, render_height)
}

fn render_transformed_image(
    ui: &mut egui::Ui,
    state: &PdfViewerState,
    viewport_rect: egui::Rect,
    image: Arc<RgbImage>,
) {
    let width = image.width();
    let height = image.height();
    let zoom = state.zoom;

    let color_image = egui::ColorImage::from_rgb([width as usize, height as usize], image.as_raw());

    let texture_handle = ui.ctx().load_texture(
        format!("pdf_page_{}", state.current_page),
        color_image,
        Default::default(),
    );

    let scaled_size = egui::Vec2::new(width as f32 * zoom, height as f32 * zoom);
    let image_rect = egui::Rect::from_min_size(viewport_rect.min + state.pan_offset, scaled_size);

    ui.set_clip_rect(viewport_rect);

    let image_widget = egui::Image::new(&texture_handle).fit_to_exact_size(scaled_size);

    ui.put(image_rect, image_widget);
}

fn handle_input(
    ui: &mut egui::Ui,
    state: &mut PdfViewerState,
    viewport_rect: egui::Rect,
    image_size: egui::Vec2,
) {
    let viewport_size = viewport_rect.size();

    handle_keyboard_shortcuts(ui, state, viewport_size, image_size);
    handle_wheel_input(ui, state, viewport_rect, image_size);
    handle_drag_input(ui, state);
    update_cursor_icon(ui, state);
}

fn handle_keyboard_shortcuts(
    ui: &mut egui::Ui,
    state: &mut PdfViewerState,
    viewport_size: egui::Vec2,
    image_size: egui::Vec2,
) {
    ui.input_mut(|i| {
        if i.consume_key(egui::Modifiers::CTRL, egui::Key::Plus)
            || i.consume_key(egui::Modifiers::CTRL, egui::Key::Equals)
        {
            let viewport_center = viewport_size * 0.5;
            state.zoom_at_cursor(state.zoom * ZOOM_BUTTON_FACTOR, viewport_center, image_size);
        }

        if i.consume_key(egui::Modifiers::CTRL, egui::Key::Minus) {
            let viewport_center = viewport_size * 0.5;
            state.zoom_at_cursor(state.zoom / ZOOM_BUTTON_FACTOR, viewport_center, image_size);
        }

        if i.consume_key(egui::Modifiers::CTRL, egui::Key::Num0) {
            state.reset_view();
        }

        let mut pan_delta = egui::Vec2::ZERO;
        if i.key_pressed(egui::Key::ArrowLeft) {
            pan_delta.x += KEYBOARD_PAN_STEP;
        }
        if i.key_pressed(egui::Key::ArrowRight) {
            pan_delta.x -= KEYBOARD_PAN_STEP;
        }
        if i.key_pressed(egui::Key::ArrowUp) {
            pan_delta.y += KEYBOARD_PAN_STEP;
        }
        if i.key_pressed(egui::Key::ArrowDown) {
            pan_delta.y -= KEYBOARD_PAN_STEP;
        }

        if pan_delta != egui::Vec2::ZERO {
            state.pan(pan_delta);
        }
    });
}

fn handle_wheel_input(
    ui: &mut egui::Ui,
    state: &mut PdfViewerState,
    viewport_rect: egui::Rect,
    image_size: egui::Vec2,
) {
    ui.input(|i| {
        let scroll_delta = i.raw_scroll_delta;

        if scroll_delta.length() < 0.1 {
            return;
        }

        if i.modifiers.ctrl {
            let zoom_factor = 1.0 + (scroll_delta.y * ZOOM_WHEEL_SENSITIVITY);
            let new_zoom = (state.zoom * zoom_factor).clamp(MIN_ZOOM, MAX_ZOOM);

            if let Some(hover_pos) = i.pointer.hover_pos() {
                let cursor_in_viewport = hover_pos - viewport_rect.min;
                state.zoom_at_cursor(new_zoom, cursor_in_viewport, image_size);
            } else {
                let viewport_center = viewport_rect.size() * 0.5;
                state.zoom_at_cursor(new_zoom, viewport_center, image_size);
            }
        } else {
            state.pan(scroll_delta);
        }
    });
}

fn handle_drag_input(ui: &mut egui::Ui, state: &mut PdfViewerState) {
    ui.input(|i| {
        let middle_mouse = i.pointer.middle_down();
        let space_drag = i.modifiers.command && i.pointer.primary_down();

        if middle_mouse || space_drag {
            state.is_panning = true;
            let delta = i.pointer.delta();
            if delta != egui::Vec2::ZERO {
                state.pan(delta);
            }
        } else {
            state.is_panning = false;
        }
    });
}

fn update_cursor_icon(ui: &mut egui::Ui, state: &PdfViewerState) {
    let space_held = ui.input(|i| i.modifiers.command);
    let middle_down = ui.input(|i| i.pointer.middle_down());
    let primary_down = ui.input(|i| i.pointer.primary_down());

    if state.is_panning {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if space_held && primary_down {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if space_held || middle_down {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    } else {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
    }
}

pub fn display_cache_stats(ui: &mut egui::Ui, stats: crate::viewer::cache::CacheStats) {
    ui.horizontal(|ui| {
        ui.label(format!(
            "Cache - GPU: {} ({:.1}MB) | RAM: {} ({:.1}MB) | Renders: {} | Hits: {} | Misses: {}",
            stats.gpu_cached_pages,
            stats.gpu_memory_mb,
            stats.ram_cached_pages,
            stats.ram_memory_mb,
            stats.total_rendered_pages,
            stats.cache_hits,
            stats.cache_misses
        ));
    });
}

fn pdf_rect_to_screen_rect(
    pdf_rect: &PdfRect,
    page_width: f32,
    page_height: f32,
    render_width: f32,
    render_height: f32,
    zoom: f32,
    pan_offset: egui::Vec2,
    viewport_min: egui::Pos2,
) -> egui::Rect {
    let pdf_left = pdf_rect.left().value;
    let pdf_bottom = pdf_rect.bottom().value;
    let pdf_right = pdf_rect.right().value;
    let pdf_top = pdf_rect.top().value;

    let norm_x = pdf_left / page_width;
    let norm_y = 1.0 - (pdf_top / page_height);
    let norm_w = (pdf_right - pdf_left) / page_width;
    let norm_h = (pdf_top - pdf_bottom) / page_height;

    let render_x = norm_x * render_width;
    let render_y = norm_y * render_height;
    let render_w = norm_w * render_width;
    let render_h = norm_h * render_height;

    let zoomed_x = render_x * zoom;
    let zoomed_y = render_y * zoom;
    let zoomed_w = render_w * zoom;
    let zoomed_h = render_h * zoom;

    let screen_x = viewport_min.x + pan_offset.x + zoomed_x;
    let screen_y = viewport_min.y + pan_offset.y + zoomed_y;

    egui::Rect::from_min_size(
        egui::pos2(screen_x, screen_y),
        egui::vec2(zoomed_w, zoomed_h),
    )
}

fn find_hovered_word(
    mouse_pos: egui::Pos2,
    tokens: &[PdfWordToken],
    page_width: f32,
    page_height: f32,
    render_width: f32,
    render_height: f32,
    zoom: f32,
    pan_offset: egui::Vec2,
    viewport_min: egui::Pos2,
) -> Option<String> {
    for token in tokens {
        let screen_rect = pdf_rect_to_screen_rect(
            &token.bounds,
            page_width,
            page_height,
            render_width,
            render_height,
            zoom,
            pan_offset,
            viewport_min,
        );

        if screen_rect.contains(mouse_pos) {
            return Some(token.text.clone());
        }
    }
    None
}

fn render_word_boundaries_overlay(
    ui: &mut egui::Ui,
    state: &mut PdfViewerState,
    viewport_rect: egui::Rect,
    rendered_image: &RgbImage,
) {
    if !state.show_word_boundaries {
        return;
    }

    let tokens = match state.get_word_tokens_for_current_page() {
        Some(t) => t,
        None => return,
    };

    let render_width = rendered_image.width() as f32;
    let render_height = rendered_image.height() as f32;
    let zoom = state.zoom;
    let pan_offset = state.pan_offset;

    let mouse_pos = ui.input(|i| i.pointer.hover_pos());
    if let Some(pos) = mouse_pos {
        let hovered = find_hovered_word(
            pos,
            &tokens.tokens,
            tokens.page_width,
            tokens.page_height,
            render_width,
            render_height,
            zoom,
            pan_offset,
            viewport_rect.min,
        );
        state.set_hovered_word(hovered);
    } else {
        state.set_hovered_word(None);
    }

    let painter = ui.painter();

    for token in &tokens.tokens {
        let screen_rect = pdf_rect_to_screen_rect(
            &token.bounds,
            tokens.page_width,
            tokens.page_height,
            render_width,
            render_height,
            zoom,
            pan_offset,
            viewport_rect.min,
        );

        if !viewport_rect.intersects(screen_rect) {
            continue;
        }

        let is_hovered = state
            .hovered_word()
            .map(|hw| hw == token.text)
            .unwrap_or(false);

        let color = if is_hovered {
            WORD_BOUNDARY_COLOR_HOVER
        } else {
            WORD_BOUNDARY_COLOR
        };

        painter.rect_stroke(
            screen_rect,
            0.0,
            egui::Stroke::new(WORD_BOUNDARY_STROKE_WIDTH, color),
            egui::StrokeKind::Outside,
        );
    }

    if let Some(text) = state.hovered_word() {
        if let Some(pointer_pos) = ui.input(|i| i.pointer.hover_pos()) {
            let sanitized_text: String = text
                .chars()
                .filter(|c| !c.is_control() || *c == ' ')
                .collect();

            egui::Area::new(egui::Id::new("word_tooltip").with(&sanitized_text))
                .pivot(egui::Align2::RIGHT_BOTTOM)
                .fixed_pos(pointer_pos)
                .order(egui::Order::Tooltip)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.label(sanitized_text);
                    });
                });
        }
    }
}
