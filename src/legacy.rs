use crate::{
    AttributeValue, Confidence, Edge, EdgeKind, EvidenceKind, Graph, GraphBuilder, Node, NodeId,
    NodeKind, Provenance, Result, SourcePosition, SourceSpan,
};
use crate::{String, Vec};
use alloc::collections::BTreeMap;
use core::str::FromStr;
use serde::{Deserialize, Serialize};

/// Compatibility representation for the current JavaScript Weavatrix graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyGraph {
    #[serde(default)]
    pub nodes: Vec<LegacyNode>,
    #[serde(default)]
    pub links: Vec<LegacyLink>,
    #[serde(flatten)]
    pub metadata: BTreeMap<String, AttributeValue>,
}

impl LegacyGraph {
    /// Converts legacy `{ nodes, links }` data into a validated graph.
    ///
    /// # Errors
    ///
    /// Returns an error when node ids are empty, nodes conflict, edge endpoints
    /// are missing, or source spans are invalid.
    pub fn into_graph(self, extractor: impl Into<String>) -> Result<Graph> {
        let extractor = extractor.into();
        let mut builder = GraphBuilder::new();
        for node in self.nodes {
            builder.add_node(node.into_node()?)?;
        }
        for link in self.links {
            builder.add_edge(link.into_edge(extractor.clone())?)?;
        }
        builder.build()
    }
}

impl TryFrom<LegacyGraph> for Graph {
    type Error = crate::GraphError;

    fn try_from(value: LegacyGraph) -> Result<Self> {
        value.into_graph("weavatrix.legacy")
    }
}

/// Legacy node shape with all unknown fields preserved as attributes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyNode {
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default, rename = "type")]
    pub node_type: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub source_file: Option<String>,
    #[serde(default)]
    pub source_range: Option<LegacyRange>,
    #[serde(default)]
    pub selection_start: Option<LegacyPoint>,
    #[serde(default)]
    pub selection_end: Option<LegacyPoint>,
    #[serde(flatten)]
    pub attributes: BTreeMap<String, AttributeValue>,
}

impl LegacyNode {
    /// Converts this compatibility node into a graph node.
    ///
    /// # Errors
    ///
    /// Returns an error when the id is empty or the source span is invalid.
    pub fn into_node(mut self) -> Result<Node> {
        let inferred_label = infer_label(&self.id);
        let kind = parse_node_kind(self.kind.as_deref().or(self.node_type.as_deref()), &self.id);
        let mut node = Node::new(self.id, self.label.unwrap_or(inferred_label), kind)?;
        node.language = self.language.take();
        if let Some(span) = self.source_range.take().and_then(|range| {
            self.source_file
                .as_ref()
                .map(|file| range.into_span(file.clone()))
        }) {
            node.span = Some(span);
        }
        if let Some(source_file) = self.source_file {
            node.attributes
                .insert("source_file".into(), source_file.into());
        }
        if let Some(selection_start) = self.selection_start {
            node.attributes
                .insert("selection_start".into(), selection_start.into());
        }
        if let Some(selection_end) = self.selection_end {
            node.attributes
                .insert("selection_end".into(), selection_end.into());
        }
        node.attributes.extend(self.attributes);
        Ok(node)
    }
}

/// Legacy link shape with all unknown fields preserved as attributes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyLink {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub relation: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default, rename = "type")]
    pub edge_type: Option<String>,
    #[serde(default)]
    pub confidence: Option<String>,
    #[serde(default)]
    pub provenance: Option<String>,
    #[serde(default)]
    pub line: Option<u32>,
    #[serde(default)]
    pub character: Option<u32>,
    #[serde(default, rename = "compileOnly")]
    pub compile_only: Option<bool>,
    #[serde(default, rename = "typeOnly")]
    pub type_only: Option<bool>,
    #[serde(default)]
    pub specifier: Option<String>,
    #[serde(default)]
    pub usage: Option<String>,
    #[serde(flatten)]
    pub attributes: BTreeMap<String, AttributeValue>,
}

impl LegacyLink {
    /// Converts this compatibility link into a graph edge.
    ///
    /// # Errors
    ///
    /// Returns an error when endpoint ids are empty or kinds are invalid.
    pub fn into_edge(mut self, extractor: impl Into<String>) -> Result<Edge> {
        let kind_value = self
            .relation
            .as_deref()
            .or(self.kind.as_deref())
            .or(self.edge_type.as_deref())
            .unwrap_or("references");
        let kind = EdgeKind::from_str(kind_value)?;
        let evidence = parse_evidence(self.provenance.as_deref().or(self.confidence.as_deref()));
        let confidence = parse_confidence(self.confidence.as_deref(), &evidence);
        let mut provenance = Provenance::new(extractor, evidence, confidence)?;
        if let Some(line) = self.line {
            let column = self.character.unwrap_or(0).saturating_add(1);
            provenance.span = Some(SourceSpan::new(
                infer_edge_file(&self.source),
                SourcePosition::new(line, column),
                SourcePosition::new(line, column.saturating_add(1)),
            ));
            self.attributes
                .insert("line".into(), i64::from(line).into());
        }
        if let Some(character) = self.character {
            self.attributes
                .insert("character".into(), i64::from(character).into());
        }
        insert_optional(&mut self.attributes, "compileOnly", self.compile_only);
        insert_optional(&mut self.attributes, "typeOnly", self.type_only);
        if let Some(specifier) = self.specifier {
            self.attributes.insert("specifier".into(), specifier.into());
        }
        if let Some(usage) = self.usage {
            self.attributes.insert("usage".into(), usage.into());
        }
        Ok(Edge {
            source: NodeId::new(self.source)?,
            target: NodeId::new(self.target)?,
            kind,
            provenance,
            attributes: self.attributes,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyPoint {
    pub line: u32,
    pub character: u32,
}

impl From<LegacyPoint> for AttributeValue {
    fn from(value: LegacyPoint) -> Self {
        let mut object = BTreeMap::new();
        object.insert("line".into(), i64::from(value.line).into());
        object.insert("character".into(), i64::from(value.character).into());
        Self::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyRange {
    pub start: LegacyPoint,
    pub end: LegacyPoint,
}

impl LegacyRange {
    #[must_use]
    pub fn into_span(self, file: String) -> SourceSpan {
        SourceSpan::new(
            file,
            SourcePosition::new(
                self.start.line.saturating_add(1),
                self.start.character.saturating_add(1),
            ),
            SourcePosition::new(
                self.end.line.saturating_add(1),
                self.end.character.saturating_add(1),
            ),
        )
    }
}

fn infer_label(id: &str) -> String {
    String::from(id.rsplit(['/', '#']).next().unwrap_or(id))
}

fn infer_edge_file(source: &str) -> String {
    String::from(source.split('#').next().unwrap_or(source))
}

fn parse_node_kind(value: Option<&str>, id: &str) -> NodeKind {
    if let Some(value) = value.and_then(|value| NodeKind::from_str(value).ok()) {
        return value;
    }
    if id.contains('#') {
        NodeKind::Function
    } else {
        NodeKind::File
    }
}

fn parse_evidence(value: Option<&str>) -> EvidenceKind {
    value
        .and_then(|value| EvidenceKind::from_str(value).ok())
        .unwrap_or(EvidenceKind::Extracted)
}

fn parse_confidence(value: Option<&str>, evidence: &EvidenceKind) -> Confidence {
    match value
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "exact" | "exact_lsp" => Confidence::Exact,
        "high" | "extracted" | "resolved" => Confidence::High,
        "medium" => Confidence::Medium,
        "low" | "inferred" | "conflict" => Confidence::Low,
        _ => match evidence {
            EvidenceKind::ExactLsp => Confidence::Exact,
            EvidenceKind::Inferred | EvidenceKind::Conflict => Confidence::Low,
            _ => Confidence::High,
        },
    }
}

fn insert_optional(
    attributes: &mut BTreeMap<String, AttributeValue>,
    key: &'static str,
    value: Option<bool>,
) {
    if let Some(value) = value {
        attributes.insert(key.into(), value.into());
    }
}
