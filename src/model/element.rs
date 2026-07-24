use super::{NodeId, Provenance, SourceSpan};
use crate::String;
use crate::{AttributeValue, EdgeKind, NodeKind, Result};
use alloc::collections::BTreeMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub label: String,
    pub kind: NodeKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attributes: BTreeMap<String, AttributeValue>,
}

impl Node {
    /// Creates a graph node.
    ///
    /// # Errors
    ///
    /// Returns an error when the node identifier is empty.
    pub fn new(id: impl Into<String>, label: impl Into<String>, kind: NodeKind) -> Result<Self> {
        Ok(Self {
            id: NodeId::new(id)?,
            label: label.into(),
            kind,
            language: None,
            span: None,
            attributes: BTreeMap::new(),
        })
    }

    #[must_use]
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    #[must_use]
    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }

    #[must_use]
    pub fn with_attribute(
        mut self,
        key: impl Into<String>,
        value: impl Into<AttributeValue>,
    ) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Edge {
    pub source: NodeId,
    pub target: NodeId,
    pub kind: EdgeKind,
    pub provenance: Provenance,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attributes: BTreeMap<String, AttributeValue>,
}

impl Edge {
    #[must_use]
    pub fn new(source: NodeId, target: NodeId, kind: EdgeKind, provenance: Provenance) -> Self {
        Self {
            source,
            target,
            kind,
            provenance,
            attributes: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn with_attribute(
        mut self,
        key: impl Into<String>,
        value: impl Into<AttributeValue>,
    ) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}
