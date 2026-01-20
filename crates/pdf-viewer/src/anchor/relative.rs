use crate::anchor::types::*;
use crate::error::Result;

pub fn resolve_relative_anchor(
    anchor: &mut AnchorPoint,
    base_anchor: &AnchorPoint,
    offset_x: f32,
    offset_y: f32,
) -> Result<()> {
    match base_anchor.status {
        AnchorStatus::Resolved => {
            if let Some((base_x, base_y)) = base_anchor.resolved_position {
                let new_x = base_x + offset_x;
                let new_y = base_y + offset_y;
                anchor.resolved_position = Some((new_x, new_y));
                anchor.status = AnchorStatus::Resolved;
                Ok(())
            } else {
                anchor.status = AnchorStatus::Failed {
                    reason: FailureReason::BaseAnchorUnresolved,
                };
                Err(crate::error::PdfError::AnchorResolutionFailed(
                    "Base anchor position not available".into(),
                ))
            }
        }
        _ => {
            anchor.status = AnchorStatus::Failed {
                reason: FailureReason::BaseAnchorUnresolved,
            };
            Err(crate::error::PdfError::AnchorResolutionFailed(
                "Base anchor not resolved".into(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_anchor(id: &str) -> AnchorPoint {
        AnchorPoint {
            id: id.to_string(),
            name: format!("Test Anchor {}", id),
            anchor_type: AnchorType::Position { x: 0.0, y: 0.0 },
            page: 0,
            resolved_position: None,
            status: AnchorStatus::Unresolved,
            is_critical: false,
        }
    }

    #[test]
    fn test_resolve_with_resolved_base_positive_offset() {
        let mut anchor = create_test_anchor("relative");
        let mut base = create_test_anchor("base");
        base.status = AnchorStatus::Resolved;
        base.resolved_position = Some((10.0, 20.0));

        let result = resolve_relative_anchor(&mut anchor, &base, 5.0, 3.0);
        assert!(result.is_ok());
        assert_eq!(anchor.resolved_position, Some((15.0, 23.0)));
        assert_eq!(anchor.status, AnchorStatus::Resolved);
    }

    #[test]
    fn test_resolve_with_resolved_base_negative_offset() {
        let mut anchor = create_test_anchor("relative");
        let mut base = create_test_anchor("base");
        base.status = AnchorStatus::Resolved;
        base.resolved_position = Some((10.0, 20.0));

        let result = resolve_relative_anchor(&mut anchor, &base, -5.0, -10.0);
        assert!(result.is_ok());
        assert_eq!(anchor.resolved_position, Some((5.0, 10.0)));
        assert_eq!(anchor.status, AnchorStatus::Resolved);
    }

    #[test]
    fn test_resolve_with_resolved_base_zero_offset() {
        let mut anchor = create_test_anchor("relative");
        let mut base = create_test_anchor("base");
        base.status = AnchorStatus::Resolved;
        base.resolved_position = Some((10.0, 20.0));

        let result = resolve_relative_anchor(&mut anchor, &base, 0.0, 0.0);
        assert!(result.is_ok());
        assert_eq!(anchor.resolved_position, Some((10.0, 20.0)));
    }

    #[test]
    fn test_resolve_with_resolved_base_large_offset() {
        let mut anchor = create_test_anchor("relative");
        let mut base = create_test_anchor("base");
        base.status = AnchorStatus::Resolved;
        base.resolved_position = Some((10.0, 20.0));

        let result = resolve_relative_anchor(&mut anchor, &base, 1000.0, 2000.0);
        assert!(result.is_ok());
        assert_eq!(anchor.resolved_position, Some((1010.0, 2020.0)));
    }

    #[test]
    fn test_resolve_with_unresolved_base() {
        let mut anchor = create_test_anchor("relative");
        let base = create_test_anchor("base");

        let result = resolve_relative_anchor(&mut anchor, &base, 5.0, 3.0);
        assert!(result.is_err());
        assert_eq!(anchor.resolved_position, None);
        assert_eq!(
            anchor.status,
            AnchorStatus::Failed {
                reason: FailureReason::BaseAnchorUnresolved
            }
        );
    }

    #[test]
    fn test_resolve_with_failed_base() {
        let mut anchor = create_test_anchor("relative");
        let mut base = create_test_anchor("base");
        base.status = AnchorStatus::Failed {
            reason: FailureReason::NotFound,
        };

        let result = resolve_relative_anchor(&mut anchor, &base, 5.0, 3.0);
        assert!(result.is_err());
        assert_eq!(
            anchor.status,
            AnchorStatus::Failed {
                reason: FailureReason::BaseAnchorUnresolved
            }
        );
    }

    #[test]
    fn test_resolve_with_resolved_base_but_no_position() {
        let mut anchor = create_test_anchor("relative");
        let mut base = create_test_anchor("base");
        base.status = AnchorStatus::Resolved;
        base.resolved_position = None;

        let result = resolve_relative_anchor(&mut anchor, &base, 5.0, 3.0);
        assert!(result.is_err());
        match result {
            Err(crate::error::PdfError::AnchorResolutionFailed(msg)) => {
                assert_eq!(msg, "Base anchor position not available");
            }
            _ => panic!("Expected AnchorResolutionFailed error"),
        }
    }

    #[test]
    fn test_error_message_unresolved() {
        let mut anchor = create_test_anchor("relative");
        let base = create_test_anchor("base");

        let result = resolve_relative_anchor(&mut anchor, &base, 5.0, 3.0);
        match result {
            Err(crate::error::PdfError::AnchorResolutionFailed(msg)) => {
                assert_eq!(msg, "Base anchor not resolved");
            }
            _ => panic!("Expected AnchorResolutionFailed error"),
        }
    }

    #[test]
    fn test_floating_point_precision() {
        let mut anchor = create_test_anchor("relative");
        let mut base = create_test_anchor("base");
        base.status = AnchorStatus::Resolved;
        base.resolved_position = Some((0.1, 0.2));

        let result = resolve_relative_anchor(&mut anchor, &base, 0.3, 0.4);
        assert!(result.is_ok());
        let (x, y) = anchor.resolved_position.unwrap();
        assert!((x - 0.4).abs() < 0.0001);
        assert!((y - 0.6).abs() < 0.0001);
    }
}
