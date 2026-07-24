use crate::{
    EdgeEndpoints, GraphError, IndexGraphView, IndexUndirectedGraphView, NodeIndex, Result,
    Topology, UndirectedTopology,
};
use crate::{String, Vec};
use alloc::collections::BTreeMap;
use core::fmt::Write as _;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphMlTopology {
    Directed(Topology),
    Undirected(UndirectedTopology),
}

impl GraphMlTopology {
    #[must_use]
    pub const fn is_directed(&self) -> bool {
        matches!(self, Self::Directed(_))
    }
}

/// Exports a directed numeric topology as deterministic `GraphML`.
#[must_use]
pub fn topology_to_graphml<G>(graph: &G) -> String
where
    G: IndexGraphView,
{
    render(
        true,
        graph.node_indices().map(G::node_slot).collect(),
        graph
            .edge_references()
            .map(|(_, edge)| (G::node_slot(edge.source()), G::node_slot(edge.target())))
            .collect(),
    )
}

/// Exports an undirected numeric topology as deterministic `GraphML`.
#[must_use]
pub fn undirected_to_graphml<G>(graph: &G) -> String
where
    G: IndexUndirectedGraphView,
{
    render(
        false,
        graph.node_indices().map(G::node_slot).collect(),
        graph
            .edge_indices()
            .filter_map(|edge| graph.edge_endpoints(edge))
            .map(|edge| (G::node_slot(edge.source()), G::node_slot(edge.target())))
            .collect(),
    )
}

/// Imports the structural `GraphML` subset: one graph, nodes, and edges.
///
/// Arbitrary node ids are accepted and mapped by node declaration order.
/// Nested graphs, ports, hyperedges, and per-edge direction overrides are
/// rejected instead of being silently discarded.
///
/// # Errors
///
/// Returns an error for malformed XML tags, references, or unsupported graph
/// features.
pub fn graphml_decode(input: &str) -> Result<GraphMlTopology> {
    if !input.contains("<graphml") {
        return Err(invalid("missing graphml root"));
    }
    let graphs = elements(input, "graph")?;
    if graphs.len() != 1 {
        return Err(invalid("exactly one graph element is required"));
    }
    let directed = match attribute(graphs[0], "edgedefault") {
        Some("directed") => true,
        Some("undirected") => false,
        _ => return Err(invalid("edgedefault must be directed or undirected")),
    };
    reject_elements(input, ["hyperedge", "port", "locator"])?;
    let nodes = elements(input, "node")?;
    let mut by_id = BTreeMap::new();
    for tag in nodes {
        let id = required_attribute(tag, "id")?;
        if by_id.insert(String::from(id), by_id.len()).is_some() {
            return Err(invalid("duplicate node id"));
        }
    }
    let mut edges = Vec::new();
    for tag in elements(input, "edge")? {
        if attribute(tag, "directed").is_some() {
            return Err(unsupported("per-edge direction overrides"));
        }
        let source = required_attribute(tag, "source")?;
        let target = required_attribute(tag, "target")?;
        let source = by_id
            .get(source)
            .copied()
            .ok_or_else(|| invalid("edge references an unknown source"))?;
        let target = by_id
            .get(target)
            .copied()
            .ok_or_else(|| invalid("edge references an unknown target"))?;
        edges.push(EdgeEndpoints::new(node(source)?, node(target)?));
    }
    if directed {
        Ok(GraphMlTopology::Directed(Topology::try_from_edges(
            by_id.len(),
            edges,
        )?))
    } else {
        Ok(GraphMlTopology::Undirected(
            UndirectedTopology::try_from_edges(by_id.len(), edges)?,
        ))
    }
}

fn render(directed: bool, mut nodes: Vec<usize>, mut edges: Vec<(usize, usize)>) -> String {
    nodes.sort_unstable();
    edges.sort_unstable();
    let default = if directed { "directed" } else { "undirected" };
    let mut output = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <graphml xmlns=\"http://graphml.graphdrawing.org/xmlns\">\n",
    );
    let _ = writeln!(output, "  <graph id=\"G\" edgedefault=\"{default}\">");
    for node in nodes {
        let _ = writeln!(output, "    <node id=\"n{node}\"/>");
    }
    for (index, (source, target)) in edges.into_iter().enumerate() {
        let _ = writeln!(
            output,
            "    <edge id=\"e{index}\" source=\"n{source}\" target=\"n{target}\"/>"
        );
    }
    output.push_str("  </graph>\n</graphml>\n");
    output
}

fn elements<'a>(input: &'a str, name: &str) -> Result<Vec<&'a str>> {
    let mut result = Vec::new();
    let mut offset = 0_usize;
    while let Some(relative) = input[offset..].find('<') {
        let start = offset + relative;
        let rest = &input[start + 1..];
        offset = start + 1;
        if rest.starts_with('/') || rest.starts_with('?') || rest.starts_with('!') {
            continue;
        }
        let Some(after_name) = rest.strip_prefix(name) else {
            continue;
        };
        if !after_name.chars().next().is_some_and(|character| {
            character.is_ascii_whitespace() || matches!(character, '/' | '>')
        }) {
            continue;
        }
        let end = rest
            .find('>')
            .ok_or_else(|| invalid("unterminated XML tag"))?;
        result.push(&rest[..end]);
        offset = start + end + 2;
    }
    Ok(result)
}

fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let mut search = tag;
    while let Some(position) = search.find(name) {
        let before_ok = position == 0 || search.as_bytes()[position - 1].is_ascii_whitespace();
        let after = search.get(position + name.len()..)?;
        let after = after.trim_start();
        if before_ok {
            let after = after.strip_prefix('=')?.trim_start();
            let quote = after.chars().next()?;
            if matches!(quote, '"' | '\'') {
                let value = &after[quote.len_utf8()..];
                let end = value.find(quote)?;
                return Some(&value[..end]);
            }
        }
        search = &after[name.len().min(after.len())..];
    }
    None
}

fn required_attribute<'a>(tag: &'a str, name: &str) -> Result<&'a str> {
    attribute(tag, name).ok_or_else(|| invalid(&format!("missing {name} attribute")))
}

fn reject_elements<const N: usize>(input: &str, names: [&'static str; N]) -> Result<()> {
    for name in names {
        if !elements(input, name)?.is_empty() {
            return Err(unsupported(name));
        }
    }
    Ok(())
}

fn node(index: usize) -> Result<NodeIndex> {
    u32::try_from(index)
        .map(NodeIndex::new)
        .map_err(|_| GraphError::IndexCapacityExceeded {
            category: "GraphML nodes",
            count: index.saturating_add(1),
        })
}

fn invalid(reason: &str) -> GraphError {
    GraphError::InvalidFormat {
        format: "GraphML",
        reason: String::from(reason),
    }
}

fn unsupported(feature: &'static str) -> GraphError {
    GraphError::UnsupportedGraphFeature {
        format: "GraphML",
        feature,
    }
}
