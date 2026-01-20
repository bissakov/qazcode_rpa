use crate::error::{PdfError, Result};
use image::RgbImage;
use pdfium_render::prelude::*;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Mutex;

fn find_pdfium_library() -> Result<PathBuf> {
    let exe_path = std::env::current_exe()
        .map_err(|e| PdfError::InvalidPdf(format!("Failed to get executable path: {}", e)))?;

    let exe_dir = exe_path
        .parent()
        .ok_or_else(|| PdfError::InvalidPdf("Executable has no parent directory".into()))?;

    let dll_path = exe_dir.join("pdfium.dll");

    if dll_path.exists() {
        return Ok(dll_path);
    }

    Err(PdfError::InvalidPdf(format!(
        "PDFium DLL not found at {}. Please ensure pdfium.dll is in the same directory as the executable.",
        dll_path.display()
    )))
}

pub struct PdfDocument {
    file_path: PathBuf,
    page_count: u16,
    pdfium: Rc<Mutex<Pdfium>>,
}

impl std::fmt::Debug for PdfDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PdfDocument")
            .field("page_count", &self.page_count)
            .finish()
    }
}

impl PdfDocument {
    pub fn load(path: &Path) -> Result<Rc<Self>> {
        if !path.exists() {
            return Err(PdfError::FileNotFound(path.to_path_buf()));
        }

        let dll_path = find_pdfium_library()?;

        let bindings = Pdfium::bind_to_library(dll_path.clone()).map_err(|e| {
            PdfError::InvalidPdf(format!(
                "Failed to bind to PDF library at {}: {}",
                dll_path.display(),
                e
            ))
        })?;

        let pdfium = Pdfium::new(bindings);

        let page_count = {
            let document = pdfium
                .load_pdf_from_file(path, None)
                .map_err(|e| PdfError::InvalidPdf(format!("Failed to load PDF: {}", e)))?;
            document.pages().len()
        };

        Ok(Rc::new(PdfDocument {
            file_path: path.to_path_buf(),
            page_count,
            pdfium: Rc::new(Mutex::new(pdfium)),
        }))
    }

    pub fn page_count(&self) -> u16 {
        self.page_count
    }

    pub fn get_page_info(&self, index: u16) -> Result<PdfPageInfo> {
        let pdfium = self
            .pdfium
            .lock()
            .map_err(|_| PdfError::RenderFailed("Failed to lock pdfium".into()))?;

        let document = pdfium
            .load_pdf_from_file(&self.file_path, None)
            .map_err(|e| PdfError::InvalidPdf(format!("Failed to load PDF: {}", e)))?;

        let page = document
            .pages()
            .get(index)
            .map_err(|e| PdfError::InvalidPdf(format!("Failed to get page: {}", e)))?;

        Ok(PdfPageInfo {
            width: page.width().value,
            height: page.height().value,
        })
    }

    pub fn render_page_to_image(
        &self,
        page_num: usize,
        width: u32,
        height: u32,
    ) -> Result<RgbImage> {
        let pdfium = self
            .pdfium
            .lock()
            .map_err(|_| PdfError::RenderFailed("Failed to lock pdfium".into()))?;

        let document = pdfium
            .load_pdf_from_file(&self.file_path, None)
            .map_err(|e| PdfError::RenderFailed(format!("Failed to load PDF: {}", e)))?;

        let page = document
            .pages()
            .iter()
            .nth(page_num)
            .ok_or(PdfError::PageOutOfRange)?;

        let bindings_ref = pdfium.bindings();
        let mut bitmap = PdfBitmap::empty(
            width as i32,
            height as i32,
            PdfBitmapFormat::BGRx,
            bindings_ref,
        )
        .map_err(|e| PdfError::RenderFailed(format!("Failed to create bitmap: {:?}", e)))?;

        page.render_into_bitmap(&mut bitmap, width as i32, height as i32, None)
            .map_err(|e| PdfError::RenderFailed(format!("Failed to render bitmap: {:?}", e)))?;

        let pixels = bitmap.as_raw_bytes();
        let mut rgb_image = RgbImage::new(width, height);

        for (i, chunk) in pixels.chunks(4).enumerate() {
            if i < (width * height) as usize {
                let x = i as u32 % width;
                let y = i as u32 / width;
                if chunk.len() >= 4 {
                    rgb_image.put_pixel(x, y, image::Rgb([chunk[0], chunk[1], chunk[2]]));
                }
            }
        }

        Ok(rgb_image)
    }

    pub fn extract_page_chars(&self, page_num: usize) -> Result<PdfPageCharData> {
        let pdfium = self
            .pdfium
            .lock()
            .map_err(|_| PdfError::RenderFailed("Failed to lock pdfium".into()))?;

        let document = pdfium
            .load_pdf_from_file(&self.file_path, None)
            .map_err(|e| PdfError::RenderFailed(format!("Failed to load PDF: {}", e)))?;

        let page = document
            .pages()
            .get(page_num as u16)
            .map_err(|_| PdfError::PageOutOfRange)?;

        let page_size = page.page_size();
        let page_width = page_size.width().value;
        let page_height = page_size.height().value;

        let page_text = page
            .text()
            .map_err(|e| PdfError::RenderFailed(format!("Failed to get page text: {}", e)))?;

        let chars = page_text.chars();
        let mut char_data = Vec::new();

        for char in chars.iter() {
            if let Some(text) = char.unicode_string()
                && let Ok(bounds) = char.loose_bounds()
            {
                char_data.push(PdfCharInfo { text, bounds });
            }
        }

        Ok(PdfPageCharData {
            page_number: page_num,
            chars: char_data,
            page_width,
            page_height,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PdfCharInfo {
    pub text: String,
    pub bounds: PdfRect,
}

#[derive(Debug, Clone)]
pub struct PdfPageCharData {
    pub page_number: usize,
    pub chars: Vec<PdfCharInfo>,
    pub page_width: f32,
    pub page_height: f32,
}

#[derive(Debug, Clone)]
pub struct PdfPageInfo {
    pub width: f32,
    pub height: f32,
}

impl PdfPageInfo {
    pub fn aspect_ratio(&self) -> f32 {
        self.width / self.height.max(0.001)
    }
}
