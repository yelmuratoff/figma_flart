//! Generate Dart or DTCG design tokens from read-only Figma snapshots.

pub mod canonical;
pub mod codegen;
mod error;
pub mod export;
pub mod generate;
pub mod snapshot;

pub use error::Error;
pub use snapshot::{Assembled, PartKind, Report, Snapshot};

const COLLECTOR: &str = include_str!("../collector/collect.js");
const PART_PLACEHOLDER: &str = "__FLART_PART__";

/// The read-only Plugin API script that returns one snapshot part when run
/// through the Figma MCP `use_figma` tool.
pub fn collector_script(part: &PartKind) -> String {
    COLLECTOR.replacen(PART_PLACEHOLDER, &part.to_string(), 1)
}
