use crate::{Edge, GraphError, Node, NodeId, Result, Topology};
use crate::{ToString, Vec};
#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap as NodeMap;
#[cfg(feature = "std")]
use std::collections::HashMap as NodeMap;

/// Groups edges by source position, orders and deduplicates each group by
/// `Edge` ordering, and returns the canonical edge list with its topology.
///
/// Edges are grouped by counting sort and ordered by sorting a permutation of
/// indices, so each edge is moved exactly once and no per-source allocation is
/// made. The resulting order is identical to sorting the edges themselves.
pub(super) fn canonicalize_edges(
    nodes: &[Node],
    edges: Vec<Edge>,
) -> Result<(Vec<Edge>, Topology)> {
    let positions = node_positions(nodes);
    let count = edges.len();
    let mut slots = Vec::with_capacity(count);
    let mut targets = Vec::with_capacity(count);
    let mut sources = Vec::with_capacity(count);
    // One extra slot holds the total, so `starts[source..=source + 1]` bounds
    // every group after the prefix sum.
    let mut starts = vec![0_usize; nodes.len() + 1];
    for edge in edges {
        let source = position(&positions, &edge.source, true)?;
        let target = position(&positions, &edge.target, false)?;
        sources.push(source);
        targets.push(target);
        starts[source + 1] += 1;
        slots.push(Some(edge));
    }
    for index in 0..nodes.len() {
        starts[index + 1] += starts[index];
    }
    let mut order = vec![0_usize; count];
    let mut cursors = starts.clone();
    for (index, source) in sources.into_iter().enumerate() {
        order[cursors[source]] = index;
        cursors[source] += 1;
    }

    let mut canonical = Vec::with_capacity(count);
    let mut endpoints = Vec::with_capacity(count);
    let mut kept = Vec::new();
    for source in 0..nodes.len() {
        let group = &mut order[starts[source]..starts[source + 1]];
        if group.is_empty() {
            continue;
        }
        group.sort_unstable_by(|left, right| slots[*left].cmp(&slots[*right]));
        // Ordering places equal edges next to each other, so one comparison
        // per neighbour deduplicates the group.
        kept.clear();
        for index in group.iter().copied() {
            if kept
                .last()
                .is_some_and(|previous: &usize| slots[*previous] == slots[index])
            {
                continue;
            }
            kept.push(index);
        }
        for index in kept.iter().copied() {
            let Some(edge) = slots[index].take() else {
                continue;
            };
            canonical.push(edge);
            endpoints.push((source, targets[index]));
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

/// Position lookup over the node list.
///
/// A hash index is measurably faster here than binary searching the sorted
/// node list: identifiers are long shared-prefix strings, so each search costs
/// a dozen string comparisons while a hash costs one pass over the bytes.
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

#[cfg(test)]
mod tests {
    use crate::{
        Confidence, Edge, EdgeKind, EvidenceKind, Graph, GraphError, Node, NodeId, NodeKind,
        Provenance, Result, Vec,
    };

    fn node(id: &str) -> Result<Node> {
        Node::new(id, id, NodeKind::File)
    }

    fn edge(source: &str, target: &str, kind: EdgeKind, extractor: &str) -> Result<Edge> {
        Ok(Edge::new(
            NodeId::new(source)?,
            NodeId::new(target)?,
            kind,
            Provenance::new(extractor, EvidenceKind::Parsed, Confidence::High)?,
        ))
    }

    /// The canonical order is a public contract: consumers persist snapshots
    /// and diff them across builds. Scrambled input with duplicates must come
    /// out in exactly the order the already-sorted path produces.
    #[test]
    fn canonical_order_matches_the_pre_sorted_path() -> Result<()> {
        let nodes = ["a", "b", "c"]
            .map(node)
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        let scrambled = vec![
            edge("c", "a", EdgeKind::Calls, "second")?,
            edge("a", "c", EdgeKind::Imports, "first")?,
            edge("b", "a", EdgeKind::Calls, "first")?,
            edge("a", "b", EdgeKind::Calls, "first")?,
            edge("c", "a", EdgeKind::Calls, "first")?,
            // Exact duplicate of an earlier edge: canonicalization keeps one.
            edge("a", "b", EdgeKind::Calls, "first")?,
            edge("a", "b", EdgeKind::Calls, "second")?,
        ];
        let canonical = Graph::try_from_parts(nodes.clone(), scrambled.clone())?;

        let mut expected = scrambled;
        expected.sort();
        expected.dedup();
        let sorted = Graph::try_from_sorted_parts(nodes, expected)?;

        assert_eq!(
            canonical.edges(),
            sorted.edges(),
            "canonicalization must agree with the already-sorted path"
        );
        assert_eq!(canonical.edge_count(), 6, "the duplicate edge is dropped");
        assert_eq!(
            canonical.edges().first().map(|edge| edge.source.as_str()),
            Some("a"),
            "edges are grouped by source position"
        );
        Ok(())
    }

    #[test]
    fn dangling_endpoints_are_reported_by_side() -> Result<()> {
        let nodes = vec![node("a")?];
        let missing_source =
            Graph::try_from_parts(nodes.clone(), vec![edge("z", "a", EdgeKind::Calls, "x")?]);
        assert!(
            matches!(missing_source, Err(GraphError::MissingEdgeSource { .. })),
            "an unknown source is reported as a missing source"
        );
        let missing_target =
            Graph::try_from_parts(nodes, vec![edge("a", "z", EdgeKind::Calls, "x")?]);
        assert!(
            matches!(missing_target, Err(GraphError::MissingEdgeTarget { .. })),
            "an unknown target is reported as a missing target"
        );
        Ok(())
    }
}
