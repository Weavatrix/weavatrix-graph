use super::SourceSpan;
use crate::String;
use crate::{EvidenceKind, GraphError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Confidence {
    Exact,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Provenance {
    pub extractor: String,
    pub evidence: EvidenceKind,
    pub confidence: Confidence,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl Provenance {
    /// Creates evidence provenance with a non-empty extractor identity.
    ///
    /// # Errors
    ///
    /// Returns an error when the extractor is empty.
    pub fn new(
        extractor: impl Into<String>,
        evidence: EvidenceKind,
        confidence: Confidence,
    ) -> Result<Self> {
        let extractor = extractor.into();
        if extractor.is_empty() {
            return Err(GraphError::EmptyExtractor);
        }
        Ok(Self {
            extractor,
            evidence,
            confidence,
            span: None,
            detail: None,
        })
    }

    #[must_use]
    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }

    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}
