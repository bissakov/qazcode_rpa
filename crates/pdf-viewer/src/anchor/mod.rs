pub mod content;
pub mod extractor;
pub mod polygon;
pub mod position;
pub mod regex_anchor;
pub mod relative;
pub mod resolver;
pub mod template;
pub mod types;

pub use extractor::extract_text_from_region;
pub use resolver::{extract_all_regions, resolve_all_anchors};
pub use template::{export_results, load_template, new_template, save_template};
pub use types::*;
