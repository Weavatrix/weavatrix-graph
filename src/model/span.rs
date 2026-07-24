use crate::String;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SourcePosition {
    /// One-based line number.
    pub line: u32,
    /// One-based byte column.
    pub column: u32,
}

impl SourcePosition {
    #[must_use]
    pub const fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SourceSpan {
    pub file: String,
    pub start: SourcePosition,
    /// Exclusive end position.
    pub end: SourcePosition,
}

impl SourceSpan {
    #[must_use]
    pub fn new(file: impl Into<String>, start: SourcePosition, end: SourcePosition) -> Self {
        Self {
            file: file.into(),
            start,
            end,
        }
    }
}
