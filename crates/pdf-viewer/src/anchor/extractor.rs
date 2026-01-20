use crate::anchor::polygon::Polygon;
use crate::anchor::types::*;
use crate::error::Result;
use crate::viewer::text_analyzer::PdfPageWordTokens;

pub fn extract_text_from_region(
    region: &AnchorRegion,
    all_anchors: &[AnchorPoint],
    word_tokens: &PdfPageWordTokens,
    overlap_threshold: f32,
) -> Result<String> {
    let anchor_positions: Vec<(f32, f32)> = region
        .anchor_ids
        .iter()
        .filter_map(|id| all_anchors.iter().find(|a| a.id == *id)?.resolved_position)
        .collect();

    if anchor_positions.len() != region.anchor_ids.len() {
        let critical_missing = region.anchor_ids.iter().any(|id| {
            all_anchors
                .iter()
                .find(|a| a.id == *id)
                .map(|a| a.is_critical && a.resolved_position.is_none())
                .unwrap_or(false)
        });

        if critical_missing {
            return Err(crate::error::PdfError::AnchorResolutionFailed(
                "Critical anchor(s) not resolved".into(),
            ));
        }
    }

    if anchor_positions.len() < 3 {
        return Err(crate::error::PdfError::InvalidPolygon(
            "Need at least 3 resolved anchors to form region".into(),
        ));
    }

    let polygon = Polygon::from_points(anchor_positions)?;

    let mut words_in_region: Vec<(f32, f32, String)> = word_tokens
        .tokens
        .iter()
        .filter_map(|token| {
            let overlap = polygon.rect_overlap_ratio(&token.bounds);
            if overlap >= overlap_threshold {
                let center_x = token.bounds.left().value;
                let center_y = token.bounds.top().value;
                Some((center_y, center_x, token.text.clone()))
            } else {
                None
            }
        })
        .collect();

    words_in_region.sort_by(|a, b| {
        let y_diff = b.0 - a.0;
        if y_diff.abs() > 5.0 {
            y_diff.partial_cmp(&0.0).unwrap()
        } else {
            a.1.partial_cmp(&b.1).unwrap()
        }
    });

    let text = words_in_region
        .iter()
        .map(|(_, _, word)| word.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    Ok(text)
}
