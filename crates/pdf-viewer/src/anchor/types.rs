use serde::{Deserialize, Serialize};

pub const DEFAULT_OVERLAP_THRESHOLD: f32 = 0.5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorPoint {
    pub id: String,
    pub name: String,
    pub anchor_type: AnchorType,
    pub page: usize,
    pub resolved_position: Option<(f32, f32)>,
    pub status: AnchorStatus,
    pub is_critical: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnchorType {
    Position {
        x: f32,
        y: f32,
    },
    Content {
        search_text: String,
        anchor_at: ContentAnchorPosition,
    },
    Regex {
        pattern: String,
        anchor_at: ContentAnchorPosition,
    },
    Relative {
        base_anchor_id: String,
        offset_x: f32,
        offset_y: f32,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ContentAnchorPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AnchorStatus {
    Unresolved,
    Resolved,
    Failed { reason: FailureReason },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FailureReason {
    NotFound,
    InvalidPattern,
    BaseAnchorMissing,
    BaseAnchorUnresolved,
    InvalidPage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorRegion {
    pub id: String,
    pub name: String,
    pub anchor_ids: Vec<String>,
    pub extracted_text: Option<String>,
    pub status: RegionStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RegionStatus {
    Resolved,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorTemplate {
    pub name: String,
    pub version: String,
    pub anchors: Vec<AnchorPoint>,
    pub regions: Vec<AnchorRegion>,
    pub overlap_threshold: f32,
    pub created_at: String,
}
