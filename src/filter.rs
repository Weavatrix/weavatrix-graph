use crate::{Confidence, Edge, EdgeKind, EvidenceKind};
use crate::{String, Vec};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EdgeFilter {
    kinds: Vec<EdgeKind>,
    evidence: Vec<EvidenceKind>,
    extractors: Vec<String>,
    minimum_confidence: Option<Confidence>,
}

impl EdgeFilter {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            kinds: Vec::new(),
            evidence: Vec::new(),
            extractors: Vec::new(),
            minimum_confidence: None,
        }
    }

    #[must_use]
    pub fn with_kind(mut self, kind: EdgeKind) -> Self {
        if !self.kinds.contains(&kind) {
            self.kinds.push(kind);
        }
        self
    }

    #[must_use]
    pub fn with_evidence(mut self, evidence: EvidenceKind) -> Self {
        if !self.evidence.contains(&evidence) {
            self.evidence.push(evidence);
        }
        self
    }

    #[must_use]
    pub fn with_extractor(mut self, extractor: impl Into<String>) -> Self {
        let extractor = extractor.into();
        if !self.extractors.contains(&extractor) {
            self.extractors.push(extractor);
        }
        self
    }

    #[must_use]
    pub const fn with_minimum_confidence(mut self, confidence: Confidence) -> Self {
        self.minimum_confidence = Some(confidence);
        self
    }

    #[must_use]
    pub fn matches(&self, edge: &Edge) -> bool {
        (self.kinds.is_empty() || self.kinds.contains(&edge.kind))
            && (self.evidence.is_empty() || self.evidence.contains(&edge.provenance.evidence))
            && (self.extractors.is_empty() || self.extractors.contains(&edge.provenance.extractor))
            && self.minimum_confidence.is_none_or(|minimum| {
                confidence_rank(edge.provenance.confidence) >= confidence_rank(minimum)
            })
    }
}

const fn confidence_rank(confidence: Confidence) -> u8 {
    match confidence {
        Confidence::Exact => 4,
        Confidence::High => 3,
        Confidence::Medium => 2,
        Confidence::Low => 1,
    }
}
