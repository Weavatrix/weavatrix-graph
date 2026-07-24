use super::core::Graph;
use super::index::{canonicalize_edges, index_sorted_edges};
use super::validate::{validate_edge, validate_node};
use crate::{Edge, GraphError, Node, Result};
use crate::{ToString, Vec};

pub(super) fn canonical(
    nodes: impl IntoIterator<Item = Node>,
    edges: impl IntoIterator<Item = Edge>,
) -> Result<Graph> {
    let mut nodes = validated(nodes, validate_node)?;
    nodes.sort_unstable_by(|left, right| left.id.cmp(&right.id));
    deduplicate_nodes(&mut nodes)?;
    let edges = validated(edges, validate_edge)?;
    let (edges, topology) = canonicalize_edges(&nodes, edges)?;
    Ok(Graph::from_indexed_parts(nodes, edges, topology))
}

pub(super) fn from_sorted_nodes(
    nodes: Vec<Node>,
    edges: impl IntoIterator<Item = Edge>,
) -> Result<Graph> {
    if !strictly_sorted_nodes(&nodes) {
        return canonical(nodes, edges);
    }
    for node in &nodes {
        validate_node(node)?;
    }
    let edges = validated(edges, validate_edge)?;
    let (edges, topology) = canonicalize_edges(&nodes, edges)?;
    Ok(Graph::from_indexed_parts(nodes, edges, topology))
}

pub(super) fn from_sorted_parts(nodes: Vec<Node>, mut edges: Vec<Edge>) -> Result<Graph> {
    if !strictly_sorted_nodes(&nodes) {
        return canonical(nodes, edges);
    }
    if !edges.is_sorted() {
        return from_sorted_nodes(nodes, edges);
    }
    for node in &nodes {
        validate_node(node)?;
    }
    for edge in &edges {
        validate_edge(edge)?;
    }
    edges.dedup();
    let topology = index_sorted_edges(&nodes, &edges)?;
    Ok(Graph::from_indexed_parts(nodes, edges, topology))
}

fn validated<T>(
    values: impl IntoIterator<Item = T>,
    validate: impl Fn(&T) -> Result<()>,
) -> Result<Vec<T>> {
    values
        .into_iter()
        .map(|value| {
            validate(&value)?;
            Ok(value)
        })
        .collect()
}

fn strictly_sorted_nodes(nodes: &[Node]) -> bool {
    nodes.windows(2).all(|pair| pair[0].id < pair[1].id)
}

fn deduplicate_nodes(nodes: &mut Vec<Node>) -> Result<()> {
    let mut conflict = None;
    nodes.dedup_by(|right, left| {
        if right.id != left.id {
            return false;
        }
        if right != left {
            conflict = Some(right.id.to_string());
        }
        true
    });
    conflict.map_or(Ok(()), |id| Err(GraphError::ConflictingNode { id }))
}
