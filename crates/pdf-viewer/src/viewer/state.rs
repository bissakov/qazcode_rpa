use super::cache;
use super::pdf_loader;
use super::text_analyzer::{self, PdfPageWordTokens};
use crate::constants::*;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Debug, Serialize, Deserialize)]
pub struct PdfViewerState {
    pub file_path: Option<PathBuf>,
    pub current_page: usize,
    pub total_pages: usize,
    pub zoom: f32,
    #[serde(skip)]
    pub last_error: Option<String>,
    #[serde(skip)]
    pub page_input_text: String,
    #[serde(skip)]
    document: Option<Rc<pdf_loader::PdfDocument>>,
    #[serde(skip)]
    cache: cache::PageCache,
    #[serde(skip)]
    pub pan_offset: egui::Vec2,
    #[serde(skip)]
    pub is_panning: bool,
    #[serde(skip)]
    pub show_word_boundaries: bool,
    #[serde(skip)]
    cached_word_tokens: Option<Rc<PdfPageWordTokens>>,
    #[serde(skip)]
    cached_tokens_page: Option<usize>,
    #[serde(skip)]
    hovered_word: Option<String>,
    #[serde(skip)]
    pub show_anchors: bool,
}

impl PdfViewerState {
    pub fn new() -> Self {
        Self {
            file_path: None,
            current_page: 0,
            total_pages: 0,
            zoom: DEFAULT_ZOOM,
            last_error: None,
            page_input_text: String::new(),
            document: None,
            cache: cache::PageCache::new(cache::CacheConfig::default()),
            pan_offset: egui::Vec2::new(DEFAULT_PAN_OFFSET_X, DEFAULT_PAN_OFFSET_Y),
            is_panning: false,
            show_word_boundaries: false,
            cached_word_tokens: None,
            cached_tokens_page: None,
            hovered_word: None,
            show_anchors: false,
        }
    }

    pub fn open_pdf(&mut self, path: PathBuf) -> Result<()> {
        self.clear_error();

        self.document = None;
        self.cache.clear();

        let doc = pdf_loader::PdfDocument::load(&path)?;
        let page_count = doc.page_count() as usize;

        self.file_path = Some(path);
        self.document = Some(doc);
        self.current_page = 0;
        self.total_pages = page_count;
        self.page_input_text = "1".to_string();

        self.zoom = DEFAULT_ZOOM;
        self.pan_offset = egui::Vec2::new(DEFAULT_PAN_OFFSET_X, DEFAULT_PAN_OFFSET_Y);
        self.clear_word_tokens_cache();

        Ok(())
    }

    pub fn next_page(&mut self) {
        if self.current_page + 1 < self.total_pages {
            self.current_page += 1;
            self.page_input_text = (self.current_page + 1).to_string();
        }
    }

    pub fn prev_page(&mut self) {
        if self.current_page > 0 {
            self.current_page -= 1;
            self.page_input_text = (self.current_page + 1).to_string();
        }
    }

    pub fn goto_page(&mut self, page: usize) -> Result<()> {
        if page >= self.total_pages {
            self.last_error = Some(format!(
                "Page {} out of range (0-{})",
                page,
                self.total_pages - 1
            ));
            return Err(crate::error::PdfError::PageOutOfRange);
        }
        self.current_page = page;
        self.page_input_text = (page + 1).to_string();
        self.last_error = None;
        Ok(())
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    }

    pub fn zoom_at_cursor(
        &mut self,
        new_zoom: f32,
        cursor_in_viewport: egui::Vec2,
        image_size: egui::Vec2,
    ) {
        let old_zoom = self.zoom;
        let clamped_zoom = new_zoom.clamp(MIN_ZOOM, MAX_ZOOM);

        if (old_zoom - clamped_zoom).abs() < 0.001 {
            return;
        }

        let cursor_in_image = cursor_in_viewport - self.pan_offset;
        let cursor_ratio = egui::Vec2::new(
            cursor_in_image.x / (image_size.x * old_zoom),
            cursor_in_image.y / (image_size.y * old_zoom),
        );

        self.zoom = clamped_zoom;

        let new_image_size = image_size * clamped_zoom;
        let new_cursor_in_image = egui::Vec2::new(
            cursor_ratio.x * new_image_size.x,
            cursor_ratio.y * new_image_size.y,
        );

        self.pan_offset = cursor_in_viewport - new_cursor_in_image;
    }

    pub fn pan(&mut self, delta: egui::Vec2) {
        self.pan_offset += delta;
    }

    pub fn reset_view(&mut self) {
        self.zoom = DEFAULT_ZOOM;
        self.pan_offset = egui::Vec2::new(DEFAULT_PAN_OFFSET_X, DEFAULT_PAN_OFFSET_Y);
    }

    pub fn clamp_pan_bounds(&mut self, viewport_size: egui::Vec2, image_size: egui::Vec2) {
        let scaled_image_size = image_size * self.zoom;

        if scaled_image_size.x < viewport_size.x {
            self.pan_offset.x = (viewport_size.x - scaled_image_size.x) * 0.5;
        } else {
            let max_x = MIN_VISIBLE_PIXELS;
            let min_x = viewport_size.x - scaled_image_size.x - MIN_VISIBLE_PIXELS;
            self.pan_offset.x = self.pan_offset.x.clamp(min_x, max_x);
        }

        if scaled_image_size.y < viewport_size.y {
            self.pan_offset.y = (viewport_size.y - scaled_image_size.y) * 0.5;
        } else {
            let max_y = MIN_VISIBLE_PIXELS;
            let min_y = viewport_size.y - scaled_image_size.y - MIN_VISIBLE_PIXELS;
            self.pan_offset.y = self.pan_offset.y.clamp(min_y, max_y);
        }
    }

    pub fn get_last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn clear_error(&mut self) {
        self.last_error = None;
    }

    pub fn document(&self) -> Option<&Rc<pdf_loader::PdfDocument>> {
        self.document.as_ref()
    }

    pub fn is_document_loaded(&self) -> bool {
        self.document.is_some()
    }

    pub fn cache(&self) -> &cache::PageCache {
        &self.cache
    }

    pub fn cache_mut(&mut self) -> &mut cache::PageCache {
        &mut self.cache
    }

    pub fn get_cache_stats(&self) -> cache::CacheStats {
        self.cache.get_stats()
    }

    pub fn toggle_word_boundaries(&mut self) {
        self.show_word_boundaries = !self.show_word_boundaries;
    }

    pub fn toggle_anchors(&mut self) {
        self.show_anchors = !self.show_anchors;
    }

    pub fn get_word_tokens_for_current_page(&mut self) -> Option<Rc<PdfPageWordTokens>> {
        if let Some(cached_page) = self.cached_tokens_page
            && cached_page == self.current_page
        {
            return self.cached_word_tokens.clone();
        }

        let document = self.document.as_ref()?;

        match text_analyzer::extract_word_tokens_for_page(document, self.current_page) {
            Ok(tokens) => {
                let tokens_arc = Rc::new(tokens);
                self.cached_word_tokens = Some(tokens_arc.clone());
                self.cached_tokens_page = Some(self.current_page);
                Some(tokens_arc)
            }
            Err(_) => None,
        }
    }

    pub fn clear_word_tokens_cache(&mut self) {
        self.cached_word_tokens = None;
        self.cached_tokens_page = None;
    }

    pub fn set_hovered_word(&mut self, text: Option<String>) {
        self.hovered_word = text;
    }

    pub fn hovered_word(&self) -> Option<&str> {
        self.hovered_word.as_deref()
    }
}

impl Default for PdfViewerState {
    fn default() -> Self {
        Self::new()
    }
}
