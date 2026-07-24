use std::collections::BTreeMap;
use std::error::Error as _;
use std::str::FromStr as _;
use weavatrix_graph::{
    AttributeValue, Confidence, Edge, EdgeKind, EvidenceKind, FiniteF64, Graph, GraphBuilder,
    GraphError, LegacyGraph, LegacyLink, LegacyNode, LegacyPoint, LegacyRange, Node, NodeId,
    NodeKind, Provenance, SourcePosition, SourceSpan,
};

fn node(id: &str) -> Node {
    Node::new(id, id, NodeKind::File).unwrap()
}

fn provenance() -> Provenance {
    Provenance::new("coverage.contract", EvidenceKind::Parsed, Confidence::High).unwrap()
}

#[test]
fn attribute_values_cover_all_public_conversions_and_float_contracts() {
    assert_eq!(
        FiniteF64::new(1.25).unwrap().get().to_bits(),
        1.25_f64.to_bits()
    );
    assert!(FiniteF64::new(f64::NAN).unwrap_err().contains("finite"));
    assert!(FiniteF64::new(f64::INFINITY).is_err());

    let mut object = BTreeMap::new();
    object.insert("nested".to_owned(), AttributeValue::from("value"));
    assert_eq!(AttributeValue::from(9_u64), AttributeValue::Unsigned(9));
    assert_eq!(AttributeValue::from(4_u32), AttributeValue::Unsigned(4));
    let values = vec![
        AttributeValue::Null,
        true.into(),
        (-7_i64).into(),
        AttributeValue::Unsigned(u64::MAX),
        3_i32.into(),
        "u32 conversion checked directly above".into(),
        AttributeValue::try_from(2.5).unwrap(),
        "text".into(),
        String::from("owned").into(),
        vec![AttributeValue::from(false)].into(),
        object.into(),
    ];

    let encoded = serde_json::to_string(&values).unwrap();
    let decoded: Vec<AttributeValue> = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, values);
    assert!(serde_json::from_str::<FiniteF64>("null").is_err());
}

#[test]
fn graph_helpers_and_validation_errors_are_contractually_visible() {
    let empty = Graph::try_from_parts([], []).unwrap();
    assert!(empty.is_empty());
    assert_eq!(empty.into_parts(), (Vec::new(), Vec::new()));

    let invalid_language = node("bad-language").with_language(" rust ");
    assert!(matches!(
        GraphBuilder::new().add_node(invalid_language),
        Err(GraphError::InvalidKind {
            category: "language",
            ..
        })
    ));

    let empty_file_span = SourceSpan::new("", SourcePosition::new(1, 1), SourcePosition::new(1, 2));
    assert!(matches!(
        GraphBuilder::new().add_node(node("empty-file").with_span(empty_file_span)),
        Err(GraphError::InvalidSpan { reason, .. }) if reason == "file must not be empty"
    ));

    let zero_position_span = SourceSpan::new(
        "src/lib.rs",
        SourcePosition::new(0, 1),
        SourcePosition::new(1, 1),
    );
    let bad_edge = Edge::new(
        NodeId::new("source").unwrap(),
        NodeId::new("target").unwrap(),
        EdgeKind::Calls,
        provenance().with_span(zero_position_span),
    );
    assert!(matches!(
        GraphBuilder::new().add_edge(bad_edge),
        Err(GraphError::InvalidSpan { reason, .. }) if reason == "positions are one-based"
    ));
}

#[test]
fn graph_error_display_messages_cover_every_variant() {
    let errors = [
        GraphError::EmptyNodeId,
        GraphError::InvalidKind {
            category: "node",
            value: " bad ".into(),
        },
        GraphError::ConflictingNode { id: "same".into() },
        GraphError::MissingEdgeSource {
            id: "source".into(),
        },
        GraphError::MissingEdgeTarget {
            id: "target".into(),
        },
        GraphError::EmptyExtractor,
        GraphError::InvalidSpan {
            file: "src/lib.rs".into(),
            reason: "positions are one-based",
        },
        GraphError::IndexCapacityExceeded {
            category: "nodes",
            count: usize::MAX,
        },
        GraphError::InvalidTopologyEndpoint {
            edge: 4,
            node: 9,
            node_count: 3,
        },
        GraphError::ArithmeticOverflow {
            operation: "test operation",
        },
        GraphError::InvalidNodeIndex {
            node: 4,
            node_count: 2,
        },
        GraphError::InvalidProbability {
            numerator: 2,
            denominator: 1,
        },
        GraphError::InvalidAlgorithmParameter {
            algorithm: "PageRank",
            parameter: "damping",
            value: "NaN".into(),
        },
        GraphError::NegativeCycle {
            algorithm: "Bellman-Ford",
        },
        GraphError::CyclicGraph {
            algorithm: "DAG longest path",
        },
        GraphError::MissingKeyedNode { endpoint: "source" },
    ];

    let messages = errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(messages.contains("node id must not be empty"));
    assert!(messages.contains("invalid node kind"));
    assert!(messages.contains("conflicting definitions"));
    assert!(messages.contains("edge source does not exist"));
    assert!(messages.contains("edge target does not exist"));
    assert!(messages.contains("extractor must not be empty"));
    assert!(messages.contains("invalid source span"));
    assert!(messages.contains("exceeds u32 index capacity"));
    assert!(messages.contains("references node index"));
    assert!(messages.contains("arithmetic overflow"));
    assert!(messages.contains("outside matrix node count"));
    assert!(messages.contains("probability 2/1"));
    assert!(messages.contains("invalid damping for PageRank"));
    assert!(messages.contains("reachable negative cycle"));
    assert!(messages.contains("requires an acyclic graph"));
    assert!(messages.contains("keyed graph source node does not exist"));
    assert!(GraphError::EmptyNodeId.source().is_none());
}

#[test]
fn node_ids_kinds_and_provenance_round_trip_through_public_traits() {
    let id = NodeId::from_str("node:1").unwrap();
    assert_eq!(id.to_string(), "node:1");
    let decoded: NodeId = serde_json::from_str("\"node:2\"").unwrap();
    assert_eq!(decoded.as_str(), "node:2");

    let custom = NodeKind::custom("domain_specific").unwrap();
    assert_eq!(custom.to_string(), "domain_specific");
    assert!(NodeKind::custom(" domain_specific").is_err());
    assert!(serde_json::from_str::<NodeKind>("\" \"").is_err());

    let span = SourceSpan::new(
        "src/lib.rs",
        SourcePosition::new(1, 1),
        SourcePosition::new(1, 5),
    );
    let provenance = provenance()
        .with_span(span.clone())
        .with_detail("checked in contract test");
    assert_eq!(provenance.span, Some(span));
    assert_eq!(
        provenance.detail.as_deref(),
        Some("checked in contract test")
    );
}

#[test]
fn legacy_conversion_covers_inference_attributes_and_fallbacks() {
    let graph = LegacyGraph {
        nodes: vec![
            LegacyNode {
                id: "src/lib.rs".into(),
                label: None,
                kind: None,
                node_type: None,
                language: Some("rust".into()),
                source_file: Some("src/lib.rs".into()),
                source_range: Some(LegacyRange {
                    start: LegacyPoint {
                        line: 0,
                        character: 0,
                    },
                    end: LegacyPoint {
                        line: 0,
                        character: 10,
                    },
                }),
                selection_start: Some(LegacyPoint {
                    line: 0,
                    character: 1,
                }),
                selection_end: Some(LegacyPoint {
                    line: 0,
                    character: 4,
                }),
                attributes: BTreeMap::from([("owned".into(), AttributeValue::from(true))]),
            },
            LegacyNode {
                id: "src/lib.rs#run".into(),
                label: None,
                kind: None,
                node_type: None,
                language: Some("rust".into()),
                source_file: None,
                source_range: None,
                selection_start: None,
                selection_end: None,
                attributes: BTreeMap::new(),
            },
            LegacyNode {
                id: "src/lib.rs#custom".into(),
                label: None,
                kind: Some("unknown-kind".into()),
                node_type: None,
                language: Some("rust".into()),
                source_file: None,
                source_range: None,
                selection_start: None,
                selection_end: None,
                attributes: BTreeMap::new(),
            },
        ],
        links: vec![LegacyLink {
            source: "src/lib.rs#run".into(),
            target: "src/lib.rs".into(),
            relation: None,
            kind: None,
            edge_type: Some("calls".into()),
            confidence: Some("conflict".into()),
            provenance: Some("not-real-evidence".into()),
            line: Some(3),
            character: None,
            compile_only: Some(true),
            type_only: Some(false),
            specifier: Some("crate::lib".into()),
            usage: Some("runtime".into()),
            attributes: BTreeMap::new(),
        }],
        metadata: BTreeMap::from([("schemaVersion".into(), AttributeValue::from("legacy"))]),
    };

    let converted = Graph::try_from(graph).unwrap();
    assert_eq!(converted.node_count(), 3);
    assert_eq!(converted.edge_count(), 1);
    assert_eq!(converted.node("src/lib.rs").unwrap().label, "lib.rs");
    assert_eq!(
        converted.node("src/lib.rs#run").unwrap().kind,
        NodeKind::Function
    );
    assert_eq!(
        converted.node("src/lib.rs#custom").unwrap().kind.as_str(),
        "unknown-kind"
    );

    let edge = &converted.edges()[0];
    assert_eq!(edge.kind, EdgeKind::Calls);
    assert_eq!(edge.provenance.evidence.as_str(), "not-real-evidence");
    assert_eq!(edge.provenance.confidence, Confidence::Low);
    assert!(edge.attributes.contains_key("compileOnly"));
    assert!(edge.attributes.contains_key("typeOnly"));
    assert!(edge.attributes.contains_key("specifier"));
    assert!(edge.attributes.contains_key("usage"));
}
