use crate::anchor::types::*;
use crate::error::Result;
use crate::viewer::text_analyzer::PdfPageWordTokens;
use pdfium_render::prelude::PdfRect;

pub fn resolve_content_anchor(
    anchor: &mut AnchorPoint,
    word_tokens: &PdfPageWordTokens,
    search_text: &str,
    anchor_at: ContentAnchorPosition,
) -> Result<()> {
    for token in &word_tokens.tokens {
        if token.text.contains(search_text) {
            let pos = calculate_anchor_position(&token.bounds, anchor_at);
            anchor.resolved_position = Some(pos);
            anchor.status = AnchorStatus::Resolved;
            return Ok(());
        }
    }

    anchor.status = AnchorStatus::Failed {
        reason: FailureReason::NotFound,
    };
    Err(crate::error::PdfError::AnchorResolutionFailed(format!(
        "Text '{}' not found",
        search_text
    )))
}

fn calculate_anchor_position(bounds: &PdfRect, anchor_at: ContentAnchorPosition) -> (f32, f32) {
    let left = bounds.left().value;
    let right = bounds.right().value;
    let top = bounds.top().value;
    let bottom = bounds.bottom().value;

    match anchor_at {
        ContentAnchorPosition::TopLeft => (left, top),
        ContentAnchorPosition::TopRight => (right, top),
        ContentAnchorPosition::BottomLeft => (left, bottom),
        ContentAnchorPosition::BottomRight => (right, bottom),
        ContentAnchorPosition::Center => ((left + right) / 2.0, (top + bottom) / 2.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewer::text_analyzer::{PdfPageWordTokens, PdfWordToken};

    fn create_test_anchor() -> AnchorPoint {
        AnchorPoint {
            id: "test".to_string(),
            name: "Test Anchor".to_string(),
            anchor_type: AnchorType::Content {
                search_text: "test".to_string(),
                anchor_at: ContentAnchorPosition::TopLeft,
            },
            page: 0,
            resolved_position: None,
            status: AnchorStatus::Unresolved,
            is_critical: false,
        }
    }

    fn create_mock_tokens(words: Vec<(&str, f32, f32, f32, f32)>) -> PdfPageWordTokens {
        let tokens = words
            .into_iter()
            .map(|(text, left, bottom, right, top)| PdfWordToken {
                text: text.to_string(),
                bounds: PdfRect::new_from_values(bottom, left, top, right),
            })
            .collect();

        PdfPageWordTokens {
            page_number: 0,
            tokens,
            page_width: 100.0,
            page_height: 100.0,
        }
    }

    #[test]
    fn test_find_exact_match() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![("Hello", 10.0, 10.0, 20.0, 20.0)]);

        let result = resolve_content_anchor(
            &mut anchor,
            &tokens,
            "Hello",
            ContentAnchorPosition::TopLeft,
        );
        assert!(result.is_ok());
        assert_eq!(anchor.status, AnchorStatus::Resolved);
        assert_eq!(anchor.resolved_position, Some((10.0, 20.0)));
    }

    #[test]
    fn test_find_partial_match() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![("HelloWorld", 10.0, 10.0, 30.0, 20.0)]);

        let result = resolve_content_anchor(
            &mut anchor,
            &tokens,
            "World",
            ContentAnchorPosition::TopLeft,
        );
        assert!(result.is_ok());
        assert_eq!(anchor.status, AnchorStatus::Resolved);
    }

    #[test]
    fn test_not_found() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![("Hello", 10.0, 10.0, 20.0, 20.0)]);

        let result = resolve_content_anchor(
            &mut anchor,
            &tokens,
            "Goodbye",
            ContentAnchorPosition::TopLeft,
        );
        assert!(result.is_err());
        assert_eq!(
            anchor.status,
            AnchorStatus::Failed {
                reason: FailureReason::NotFound
            }
        );
        assert_eq!(anchor.resolved_position, None);
    }

    #[test]
    fn test_first_match_wins() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![
            ("test", 10.0, 10.0, 20.0, 20.0),
            ("test", 30.0, 30.0, 40.0, 40.0),
        ]);

        let result =
            resolve_content_anchor(&mut anchor, &tokens, "test", ContentAnchorPosition::TopLeft);
        assert!(result.is_ok());
        assert_eq!(anchor.resolved_position, Some((10.0, 20.0)));
    }

    #[test]
    fn test_anchor_position_top_left() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![("Test", 10.0, 20.0, 30.0, 40.0)]);

        resolve_content_anchor(&mut anchor, &tokens, "Test", ContentAnchorPosition::TopLeft)
            .unwrap();
        assert_eq!(anchor.resolved_position, Some((10.0, 40.0)));
    }

    #[test]
    fn test_anchor_position_top_right() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![("Test", 10.0, 20.0, 30.0, 40.0)]);

        resolve_content_anchor(
            &mut anchor,
            &tokens,
            "Test",
            ContentAnchorPosition::TopRight,
        )
        .unwrap();
        assert_eq!(anchor.resolved_position, Some((30.0, 40.0)));
    }

    #[test]
    fn test_anchor_position_bottom_left() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![("Test", 10.0, 20.0, 30.0, 40.0)]);

        resolve_content_anchor(
            &mut anchor,
            &tokens,
            "Test",
            ContentAnchorPosition::BottomLeft,
        )
        .unwrap();
        assert_eq!(anchor.resolved_position, Some((10.0, 20.0)));
    }

    #[test]
    fn test_anchor_position_bottom_right() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![("Test", 10.0, 20.0, 30.0, 40.0)]);

        resolve_content_anchor(
            &mut anchor,
            &tokens,
            "Test",
            ContentAnchorPosition::BottomRight,
        )
        .unwrap();
        assert_eq!(anchor.resolved_position, Some((30.0, 20.0)));
    }

    #[test]
    fn test_anchor_position_center() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![("Test", 10.0, 20.0, 30.0, 40.0)]);

        resolve_content_anchor(&mut anchor, &tokens, "Test", ContentAnchorPosition::Center)
            .unwrap();
        assert_eq!(anchor.resolved_position, Some((20.0, 30.0)));
    }

    #[test]
    fn test_case_sensitive_search() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![("Hello", 10.0, 10.0, 20.0, 20.0)]);

        let result = resolve_content_anchor(
            &mut anchor,
            &tokens,
            "hello",
            ContentAnchorPosition::TopLeft,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_tokens() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![]);

        let result =
            resolve_content_anchor(&mut anchor, &tokens, "test", ContentAnchorPosition::TopLeft);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_message() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![]);

        let result = resolve_content_anchor(
            &mut anchor,
            &tokens,
            "missing",
            ContentAnchorPosition::TopLeft,
        );
        match result {
            Err(crate::error::PdfError::AnchorResolutionFailed(msg)) => {
                assert_eq!(msg, "Text 'missing' not found");
            }
            _ => panic!("Expected AnchorResolutionFailed error"),
        }
    }
}
