use crate::anchor::types::*;
use crate::error::Result;
use crate::viewer::text_analyzer::PdfPageWordTokens;
use pdfium_render::prelude::PdfRect;
use regex::Regex;

pub fn resolve_regex_anchor(
    anchor: &mut AnchorPoint,
    word_tokens: &PdfPageWordTokens,
    pattern: &str,
    anchor_at: ContentAnchorPosition,
) -> Result<()> {
    let re = match Regex::new(pattern) {
        Ok(r) => r,
        Err(_) => {
            anchor.status = AnchorStatus::Failed {
                reason: FailureReason::InvalidPattern,
            };
            return Err(crate::error::PdfError::AnchorResolutionFailed(
                "Invalid regex pattern".into(),
            ));
        }
    };

    for token in &word_tokens.tokens {
        if re.is_match(&token.text) {
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
        "Pattern '{}' not found",
        pattern
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
            anchor_type: AnchorType::Regex {
                pattern: r"\d+".to_string(),
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
    fn test_match_digits() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![("Invoice123", 10.0, 10.0, 30.0, 20.0)]);

        let result =
            resolve_regex_anchor(&mut anchor, &tokens, r"\d+", ContentAnchorPosition::TopLeft);
        assert!(result.is_ok());
        assert_eq!(anchor.status, AnchorStatus::Resolved);
        assert_eq!(anchor.resolved_position, Some((10.0, 20.0)));
    }

    #[test]
    fn test_match_email_pattern() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![("user@example.com", 10.0, 10.0, 50.0, 20.0)]);

        let result = resolve_regex_anchor(
            &mut anchor,
            &tokens,
            r"\w+@\w+\.\w+",
            ContentAnchorPosition::Center,
        );
        assert!(result.is_ok());
        assert_eq!(anchor.resolved_position, Some((30.0, 15.0)));
    }

    #[test]
    fn test_match_date_pattern() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![("Date:2024-01-15", 10.0, 10.0, 40.0, 20.0)]);

        let result = resolve_regex_anchor(
            &mut anchor,
            &tokens,
            r"\d{4}-\d{2}-\d{2}",
            ContentAnchorPosition::TopLeft,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_match() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![("NoNumbers", 10.0, 10.0, 30.0, 20.0)]);

        let result =
            resolve_regex_anchor(&mut anchor, &tokens, r"\d+", ContentAnchorPosition::TopLeft);
        assert!(result.is_err());
        assert_eq!(
            anchor.status,
            AnchorStatus::Failed {
                reason: FailureReason::NotFound
            }
        );
    }

    #[test]
    fn test_invalid_regex_pattern() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![("Test", 10.0, 10.0, 20.0, 20.0)]);

        let result = resolve_regex_anchor(
            &mut anchor,
            &tokens,
            r"[invalid(",
            ContentAnchorPosition::TopLeft,
        );
        assert!(result.is_err());
        assert_eq!(
            anchor.status,
            AnchorStatus::Failed {
                reason: FailureReason::InvalidPattern
            }
        );
        match result {
            Err(crate::error::PdfError::AnchorResolutionFailed(msg)) => {
                assert_eq!(msg, "Invalid regex pattern");
            }
            _ => panic!("Expected AnchorResolutionFailed error"),
        }
    }

    #[test]
    fn test_first_match_wins() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![
            ("123", 10.0, 10.0, 20.0, 20.0),
            ("456", 30.0, 30.0, 40.0, 40.0),
        ]);

        let result =
            resolve_regex_anchor(&mut anchor, &tokens, r"\d+", ContentAnchorPosition::TopLeft);
        assert!(result.is_ok());
        assert_eq!(anchor.resolved_position, Some((10.0, 20.0)));
    }

    #[test]
    fn test_anchor_position_top_right() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![("123", 10.0, 20.0, 30.0, 40.0)]);

        resolve_regex_anchor(
            &mut anchor,
            &tokens,
            r"\d+",
            ContentAnchorPosition::TopRight,
        )
        .unwrap();
        assert_eq!(anchor.resolved_position, Some((30.0, 40.0)));
    }

    #[test]
    fn test_anchor_position_bottom_left() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![("123", 10.0, 20.0, 30.0, 40.0)]);

        resolve_regex_anchor(
            &mut anchor,
            &tokens,
            r"\d+",
            ContentAnchorPosition::BottomLeft,
        )
        .unwrap();
        assert_eq!(anchor.resolved_position, Some((10.0, 20.0)));
    }

    #[test]
    fn test_anchor_position_bottom_right() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![("123", 10.0, 20.0, 30.0, 40.0)]);

        resolve_regex_anchor(
            &mut anchor,
            &tokens,
            r"\d+",
            ContentAnchorPosition::BottomRight,
        )
        .unwrap();
        assert_eq!(anchor.resolved_position, Some((30.0, 20.0)));
    }

    #[test]
    fn test_anchor_position_center() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![("123", 10.0, 20.0, 30.0, 40.0)]);

        resolve_regex_anchor(&mut anchor, &tokens, r"\d+", ContentAnchorPosition::Center).unwrap();
        assert_eq!(anchor.resolved_position, Some((20.0, 30.0)));
    }

    #[test]
    fn test_case_insensitive_regex() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![("Hello", 10.0, 10.0, 20.0, 20.0)]);

        let result = resolve_regex_anchor(
            &mut anchor,
            &tokens,
            r"(?i)hello",
            ContentAnchorPosition::TopLeft,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_word_boundary_regex() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![
            ("test", 10.0, 10.0, 20.0, 20.0),
            ("testing", 30.0, 30.0, 40.0, 40.0),
        ]);

        let result = resolve_regex_anchor(
            &mut anchor,
            &tokens,
            r"\btest\b",
            ContentAnchorPosition::TopLeft,
        );
        assert!(result.is_ok());
        assert_eq!(anchor.resolved_position, Some((10.0, 20.0)));
    }

    #[test]
    fn test_empty_tokens() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![]);

        let result =
            resolve_regex_anchor(&mut anchor, &tokens, r"\d+", ContentAnchorPosition::TopLeft);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_message_not_found() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_tokens(vec![("test", 10.0, 10.0, 20.0, 20.0)]);

        let result =
            resolve_regex_anchor(&mut anchor, &tokens, r"\d+", ContentAnchorPosition::TopLeft);
        match result {
            Err(crate::error::PdfError::AnchorResolutionFailed(msg)) => {
                assert!(msg.contains("not found"));
            }
            _ => panic!("Expected AnchorResolutionFailed error"),
        }
    }
}
