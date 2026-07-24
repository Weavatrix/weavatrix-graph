use crate::{Edge, GraphError, Node, NodeId, Result, Topology};
use crate::{ToString, Vec};
#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap as NodeMap;
#[cfg(feature = "std")]
use std::collections::HashMap as NodeMap;

pub(super) fn canonicalize_edges(
    nodes: &[Node],
    edges: Vec<Edge>,
) -> Result<(Vec<Edge>, Topology)> {
    let positions = node_positions(nodes);
    let mut mapped = Vec::with_capacity(edges.len());
    let mut counts = vec![0_usize; nodes.len()];
    for edge in edges {
        let source = position(&positions, &edge.source, true)?;
        let target = position(&positions, &edge.target, false)?;
        counts[source] += 1;
        mapped.push((source, edge, target));
    }
    let mut buckets = counts
        .into_iter()
        .map(Vec::with_capacity)
        .collect::<Vec<Vec<(Edge, usize)>>>();
    for (source, edge, target) in mapped {
        buckets[source].push((edge, target));
    }

    let edge_count = buckets.iter().map(Vec::len).sum();
    let mut canonical = Vec::with_capacity(edge_count);
    let mut endpoints = Vec::with_capacity(edge_count);
    for (source, mut bucket) in buckets.into_iter().enumerate() {
        bucket.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        bucket.dedup_by(|left, right| left.0 == right.0);
        for (edge, target) in bucket {
            canonical.push(edge);
            endpoints.push((source, target));
        }
    }
    let topology = Topology::try_from_usize_edges(nodes.len(), endpoints)?;
    Ok((canonical, topology))
}

pub(super) fn index_canonical_edges(nodes: &[Node], edges: &[Edge]) -> Result<Topology> {
    let positions = node_positions(nodes);
    let mut endpoints = Vec::with_capacity(edges.len());
    for edge in edges {
        let source = position(&positions, &edge.source, true)?;
        let target = position(&positions, &edge.target, false)?;
        endpoints.push((source, target));
    }
    Topology::try_from_usize_edges(nodes.len(), endpoints)
}

pub(super) fn index_sorted_edges(nodes: &[Node], edges: &[Edge]) -> Result<Topology> {
    let positions = node_positions(nodes);
    let mut endpoints = Vec::with_capacity(edges.len());
    let mut source_cursor = 0;
    for edge in edges {
        while source_cursor < nodes.len() && nodes[source_cursor].id < edge.source {
            source_cursor += 1;
        }
        if source_cursor == nodes.len() || nodes[source_cursor].id != edge.source {
            return Err(GraphError::MissingEdgeSource {
                id: edge.source.to_string(),
            });
        }
        let target = position(&positions, &edge.target, false)?;
        endpoints.push((source_cursor, target));
    }
    Topology::try_from_usize_edges(nodes.len(), endpoints)
}

fn node_positions(nodes: &[Node]) -> NodeMap<&NodeId, usize> {
    nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (&node.id, index))
        .collect()
}

fn position(positions: &NodeMap<&NodeId, usize>, id: &NodeId, source: bool) -> Result<usize> {
    positions.get(id).copied().ok_or_else(|| {
        if source {
            GraphError::MissingEdgeSource { id: id.to_string() }
        } else {
            GraphError::MissingEdgeTarget { id: id.to_string() }
        }
    })
}
