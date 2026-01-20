use crate::error::Result;
use crate::viewer::pdf_loader::PdfDocument;
use pdfium_render::prelude::PdfRect;

#[derive(Debug, Clone)]
pub struct PdfWordToken {
    pub text: String,
    pub bounds: PdfRect,
}

#[derive(Debug, Clone)]
pub struct PdfPageWordTokens {
    pub page_number: usize,
    pub tokens: Vec<PdfWordToken>,
    pub page_width: f32,
    pub page_height: f32,
}

pub fn extract_word_tokens_for_page(
    document: &PdfDocument,
    page_num: usize,
) -> Result<PdfPageWordTokens> {
    let char_data = document.extract_page_chars(page_num)?;

    let mut tokens = Vec::new();
    let mut current_word = String::new();
    let mut word_bounds: Option<PdfRect> = None;

    for char_info in &char_data.chars {
        let char_str = &char_info.text;
        let char_rect = char_info.bounds;

        if char_str.trim().is_empty() {
            if !current_word.is_empty() {
                if let Some(bounds) = word_bounds.take() {
                    tokens.push(PdfWordToken {
                        text: current_word.clone(),
                        bounds,
                    });
                }
                current_word.clear();
            }
        } else {
            current_word.push_str(char_str);

            word_bounds = Some(match word_bounds {
                Some(existing) => merge_rects(existing, char_rect),
                None => char_rect,
            });
        }
    }

    if !current_word.is_empty()
        && let Some(bounds) = word_bounds
    {
        tokens.push(PdfWordToken {
            text: current_word,
            bounds,
        });
    }

    Ok(PdfPageWordTokens {
        page_number: page_num,
        tokens,
        page_width: char_data.page_width,
        page_height: char_data.page_height,
    })
}

fn merge_rects(r1: PdfRect, r2: PdfRect) -> PdfRect {
    let min_x = r1.left().value.min(r2.left().value);
    let min_y = r1.bottom().value.min(r2.bottom().value);
    let max_x = r1.right().value.max(r2.right().value);
    let max_y = r1.top().value.max(r2.top().value);

    PdfRect::new_from_values(min_y, min_x, max_y, max_x)
}
