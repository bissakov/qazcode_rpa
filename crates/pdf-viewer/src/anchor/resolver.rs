use crate::anchor::{content, position, regex_anchor, relative, types::*};
use crate::error::Result;
use crate::viewer::pdf_loader::PdfDocument;
use crate::viewer::text_analyzer;
use std::collections::HashMap;

pub fn resolve_all_anchors(
    template: &mut AnchorTemplate,
    document: &PdfDocument,
) -> HashMap<String, Result<()>> {
    let mut results = HashMap::new();

    let mut anchors_by_page: HashMap<usize, Vec<usize>> = HashMap::new();
    for (idx, anchor) in template.anchors.iter().enumerate() {
        anchors_by_page.entry(anchor.page).or_default().push(idx);
    }

    for (page_num, anchor_indices) in anchors_by_page {
        let word_tokens = match text_analyzer::extract_word_tokens_for_page(document, page_num) {
            Ok(tokens) => tokens,
            Err(e) => {
                for &idx in &anchor_indices {
                    let anchor = &mut template.anchors[idx];
                    anchor.status = AnchorStatus::Failed {
                        reason: FailureReason::InvalidPage,
                    };
                    results.insert(anchor.id.clone(), Err(e.clone()));
                }
                continue;
            }
        };

        for &idx in &anchor_indices {
            let result = resolve_single_anchor(&mut template.anchors, idx, &word_tokens);
            let anchor_id = template.anchors[idx].id.clone();
            results.insert(anchor_id, result);
        }
    }

    results
}

fn resolve_single_anchor(
    anchors: &mut [AnchorPoint],
    idx: usize,
    word_tokens: &text_analyzer::PdfPageWordTokens,
) -> Result<()> {
    let anchor_type = anchors[idx].anchor_type.clone();

    match &anchor_type {
        AnchorType::Position { x, y } => {
            position::resolve_position_anchor(&mut anchors[idx], *x, *y, word_tokens)
        }
        AnchorType::Content {
            search_text,
            anchor_at,
        } => {
            content::resolve_content_anchor(&mut anchors[idx], word_tokens, search_text, *anchor_at)
        }
        AnchorType::Regex { pattern, anchor_at } => {
            regex_anchor::resolve_regex_anchor(&mut anchors[idx], word_tokens, pattern, *anchor_at)
        }
        AnchorType::Relative {
            base_anchor_id,
            offset_x,
            offset_y,
        } => {
            let base_anchor = anchors.iter().find(|a| a.id == *base_anchor_id).cloned();

            match base_anchor {
                Some(base) => relative::resolve_relative_anchor(
                    &mut anchors[idx],
                    &base,
                    *offset_x,
                    *offset_y,
                ),
                None => {
                    anchors[idx].status = AnchorStatus::Failed {
                        reason: FailureReason::BaseAnchorMissing,
                    };
                    Err(crate::error::PdfError::AnchorResolutionFailed(
                        "Base anchor not found".into(),
                    ))
                }
            }
        }
    }
}

pub fn extract_all_regions(
    template: &mut AnchorTemplate,
    document: &PdfDocument,
) -> HashMap<String, Result<String>> {
    let mut results = HashMap::new();

    for region in &mut template.regions {
        let pages: Vec<usize> = region
            .anchor_ids
            .iter()
            .filter_map(|id| {
                template
                    .anchors
                    .iter()
                    .find(|a| a.id == *id)
                    .map(|a| a.page)
            })
            .collect();

        if pages.is_empty() {
            results.insert(
                region.name.clone(),
                Err(crate::error::PdfError::AnchorResolutionFailed(
                    "No anchors found for region".into(),
                )),
            );
            continue;
        }

        let page = pages[0];
        if !pages.iter().all(|p| *p == page) {
            results.insert(
                region.name.clone(),
                Err(crate::error::PdfError::AnchorResolutionFailed(
                    "Region spans multiple pages (not supported)".into(),
                )),
            );
            continue;
        }

        let word_tokens = match text_analyzer::extract_word_tokens_for_page(document, page) {
            Ok(tokens) => tokens,
            Err(e) => {
                results.insert(region.name.clone(), Err(e));
                continue;
            }
        };

        match crate::anchor::extractor::extract_text_from_region(
            region,
            &template.anchors,
            &word_tokens,
            template.overlap_threshold,
        ) {
            Ok(text) => {
                region.extracted_text = Some(text.clone());
                region.status = RegionStatus::Resolved;
                results.insert(region.name.clone(), Ok(text));
            }
            Err(e) => {
                region.status = RegionStatus::Failed;
                results.insert(region.name.clone(), Err(e));
            }
        }
    }

    results
}
