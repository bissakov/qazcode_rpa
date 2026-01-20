use crate::anchor::types::*;
use crate::error::Result;
use crate::viewer::text_analyzer::PdfPageWordTokens;

pub fn resolve_position_anchor(
    anchor: &mut AnchorPoint,
    x: f32,
    y: f32,
    word_tokens: &PdfPageWordTokens,
) -> Result<()> {
    if x < 0.0 || y < 0.0 || x > word_tokens.page_width || y > word_tokens.page_height {
        anchor.status = AnchorStatus::Failed {
            reason: FailureReason::InvalidPage,
        };
        return Err(crate::error::PdfError::AnchorResolutionFailed(
            "Position outside page bounds".into(),
        ));
    }

    anchor.resolved_position = Some((x, y));
    anchor.status = AnchorStatus::Resolved;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewer::text_analyzer::PdfPageWordTokens;

    fn create_mock_word_tokens(width: f32, height: f32) -> PdfPageWordTokens {
        PdfPageWordTokens {
            page_number: 0,
            tokens: vec![],
            page_width: width,
            page_height: height,
        }
    }

    fn create_test_anchor() -> AnchorPoint {
        AnchorPoint {
            id: "test".to_string(),
            name: "Test Anchor".to_string(),
            anchor_type: AnchorType::Position { x: 0.0, y: 0.0 },
            page: 0,
            resolved_position: None,
            status: AnchorStatus::Unresolved,
            is_critical: false,
        }
    }

    #[test]
    fn test_valid_position_center() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_word_tokens(100.0, 100.0);
        let result = resolve_position_anchor(&mut anchor, 50.0, 50.0, &tokens);
        assert!(result.is_ok());
        assert_eq!(anchor.resolved_position, Some((50.0, 50.0)));
        assert_eq!(anchor.status, AnchorStatus::Resolved);
    }

    #[test]
    fn test_valid_position_origin() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_word_tokens(100.0, 100.0);
        let result = resolve_position_anchor(&mut anchor, 0.0, 0.0, &tokens);
        assert!(result.is_ok());
        assert_eq!(anchor.resolved_position, Some((0.0, 0.0)));
    }

    #[test]
    fn test_valid_position_max_bounds() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_word_tokens(100.0, 100.0);
        let result = resolve_position_anchor(&mut anchor, 100.0, 100.0, &tokens);
        assert!(result.is_ok());
        assert_eq!(anchor.resolved_position, Some((100.0, 100.0)));
    }

    #[test]
    fn test_invalid_position_negative_x() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_word_tokens(100.0, 100.0);
        let result = resolve_position_anchor(&mut anchor, -1.0, 50.0, &tokens);
        assert!(result.is_err());
        assert_eq!(anchor.resolved_position, None);
        assert_eq!(
            anchor.status,
            AnchorStatus::Failed {
                reason: FailureReason::InvalidPage
            }
        );
    }

    #[test]
    fn test_invalid_position_negative_y() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_word_tokens(100.0, 100.0);
        let result = resolve_position_anchor(&mut anchor, 50.0, -1.0, &tokens);
        assert!(result.is_err());
        assert_eq!(
            anchor.status,
            AnchorStatus::Failed {
                reason: FailureReason::InvalidPage
            }
        );
    }

    #[test]
    fn test_invalid_position_exceeds_width() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_word_tokens(100.0, 100.0);
        let result = resolve_position_anchor(&mut anchor, 100.1, 50.0, &tokens);
        assert!(result.is_err());
        assert_eq!(
            anchor.status,
            AnchorStatus::Failed {
                reason: FailureReason::InvalidPage
            }
        );
    }

    #[test]
    fn test_invalid_position_exceeds_height() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_word_tokens(100.0, 100.0);
        let result = resolve_position_anchor(&mut anchor, 50.0, 100.1, &tokens);
        assert!(result.is_err());
        assert_eq!(
            anchor.status,
            AnchorStatus::Failed {
                reason: FailureReason::InvalidPage
            }
        );
    }

    #[test]
    fn test_invalid_position_both_out_of_bounds() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_word_tokens(100.0, 100.0);
        let result = resolve_position_anchor(&mut anchor, 200.0, 200.0, &tokens);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_message() {
        let mut anchor = create_test_anchor();
        let tokens = create_mock_word_tokens(100.0, 100.0);
        let result = resolve_position_anchor(&mut anchor, -1.0, -1.0, &tokens);
        match result {
            Err(crate::error::PdfError::AnchorResolutionFailed(msg)) => {
                assert_eq!(msg, "Position outside page bounds");
            }
            _ => panic!("Expected AnchorResolutionFailed error"),
        }
    }
}
