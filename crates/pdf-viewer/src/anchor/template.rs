use crate::anchor::types::*;
use crate::error::Result;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn save_template(template: &AnchorTemplate, path: &Path) -> Result<()> {
    let mut clean_template = template.clone();
    clean_template.anchors.retain(|a| !a.id.starts_with("doc-"));

    let json = serde_json::to_string_pretty(&clean_template)
        .map_err(|e| crate::error::PdfError::SerializationError(e.to_string()))?;

    fs::write(path, json).map_err(|e| crate::error::PdfError::FileWriteError(e.to_string()))?;

    Ok(())
}

pub fn load_template(path: &Path) -> Result<AnchorTemplate> {
    let json = fs::read_to_string(path)
        .map_err(|e| crate::error::PdfError::FileReadError(e.to_string()))?;

    let mut template: AnchorTemplate = serde_json::from_str(&json)
        .map_err(|e| crate::error::PdfError::SerializationError(e.to_string()))?;

    for anchor in &mut template.anchors {
        anchor.status = AnchorStatus::Unresolved;
        anchor.resolved_position = None;
    }

    for region in &mut template.regions {
        region.status = RegionStatus::Failed;
        region.extracted_text = None;
    }

    Ok(template)
}

pub fn export_results(template: &AnchorTemplate, output_path: &Path) -> Result<()> {
    let mut results = HashMap::new();

    for region in &template.regions {
        let key = sanitize_json_key(&region.name);
        let value = region.extracted_text.clone().unwrap_or_default();
        results.insert(key, value);
    }

    let json = serde_json::to_string_pretty(&results)
        .map_err(|e| crate::error::PdfError::SerializationError(e.to_string()))?;

    fs::write(output_path, json)
        .map_err(|e| crate::error::PdfError::FileWriteError(e.to_string()))?;

    Ok(())
}

fn sanitize_json_key(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

pub fn new_template(name: String) -> AnchorTemplate {
    AnchorTemplate {
        name,
        version: "1.0.0".to_string(),
        anchors: Vec::new(),
        regions: Vec::new(),
        overlap_threshold: DEFAULT_OVERLAP_THRESHOLD,
        created_at: chrono::Utc::now().to_rfc3339(),
    }
}

pub fn generate_document_anchors(
    page_num: usize,
    page_width: f32,
    page_height: f32,
) -> Vec<AnchorPoint> {
    let page_display = page_num + 1;
    let half_width = page_width / 2.0;
    let half_height = page_height / 2.0;

    vec![
        AnchorPoint {
            id: format!("doc-top-left-p{}", page_display),
            name: format!("Doc: Top-Left (P{})", page_display),
            anchor_type: AnchorType::Position {
                x: 0.0,
                y: page_height,
            },
            page: page_num,
            resolved_position: Some((0.0, page_height)),
            status: AnchorStatus::Resolved,
            is_critical: false,
        },
        AnchorPoint {
            id: format!("doc-top-center-p{}", page_display),
            name: format!("Doc: Top-Center (P{})", page_display),
            anchor_type: AnchorType::Position {
                x: half_width,
                y: page_height,
            },
            page: page_num,
            resolved_position: Some((half_width, page_height)),
            status: AnchorStatus::Resolved,
            is_critical: false,
        },
        AnchorPoint {
            id: format!("doc-top-right-p{}", page_display),
            name: format!("Doc: Top-Right (P{})", page_display),
            anchor_type: AnchorType::Position {
                x: page_width,
                y: page_height,
            },
            page: page_num,
            resolved_position: Some((page_width, page_height)),
            status: AnchorStatus::Resolved,
            is_critical: false,
        },
        AnchorPoint {
            id: format!("doc-middle-left-p{}", page_display),
            name: format!("Doc: Middle-Left (P{})", page_display),
            anchor_type: AnchorType::Position {
                x: 0.0,
                y: half_height,
            },
            page: page_num,
            resolved_position: Some((0.0, half_height)),
            status: AnchorStatus::Resolved,
            is_critical: false,
        },
        AnchorPoint {
            id: format!("doc-center-p{}", page_display),
            name: format!("Doc: Center (P{})", page_display),
            anchor_type: AnchorType::Position {
                x: half_width,
                y: half_height,
            },
            page: page_num,
            resolved_position: Some((half_width, half_height)),
            status: AnchorStatus::Resolved,
            is_critical: false,
        },
        AnchorPoint {
            id: format!("doc-middle-right-p{}", page_display),
            name: format!("Doc: Middle-Right (P{})", page_display),
            anchor_type: AnchorType::Position {
                x: page_width,
                y: half_height,
            },
            page: page_num,
            resolved_position: Some((page_width, half_height)),
            status: AnchorStatus::Resolved,
            is_critical: false,
        },
        AnchorPoint {
            id: format!("doc-bottom-left-p{}", page_display),
            name: format!("Doc: Bottom-Left (P{})", page_display),
            anchor_type: AnchorType::Position { x: 0.0, y: 0.0 },
            page: page_num,
            resolved_position: Some((0.0, 0.0)),
            status: AnchorStatus::Resolved,
            is_critical: false,
        },
        AnchorPoint {
            id: format!("doc-bottom-center-p{}", page_display),
            name: format!("Doc: Bottom-Center (P{})", page_display),
            anchor_type: AnchorType::Position {
                x: half_width,
                y: 0.0,
            },
            page: page_num,
            resolved_position: Some((half_width, 0.0)),
            status: AnchorStatus::Resolved,
            is_critical: false,
        },
        AnchorPoint {
            id: format!("doc-bottom-right-p{}", page_display),
            name: format!("Doc: Bottom-Right (P{})", page_display),
            anchor_type: AnchorType::Position {
                x: page_width,
                y: 0.0,
            },
            page: page_num,
            resolved_position: Some((page_width, 0.0)),
            status: AnchorStatus::Resolved,
            is_critical: false,
        },
    ]
}

pub fn inject_document_anchors(
    template: &mut AnchorTemplate,
    document: &crate::viewer::pdf_loader::PdfDocument,
    current_page: usize,
) -> Result<()> {
    template
        .anchors
        .retain(|a| !a.id.starts_with("doc-") || a.page != current_page);

    let page_info = document.get_page_info(current_page as u16)?;
    let anchors = generate_document_anchors(current_page, page_info.width, page_info.height);

    let mut doc_anchors = anchors;
    doc_anchors.extend(template.anchors.drain(..));
    template.anchors = doc_anchors;

    Ok(())
}
