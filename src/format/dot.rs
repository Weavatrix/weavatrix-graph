use crate::{
    EdgeEndpoints, GraphError, IndexGraphView, IndexUndirectedGraphView, NodeIndex, Result,
    Topology, UndirectedTopology,
};
use crate::{String, Vec};
use core::fmt::Write as _;

/// Exports a directed numeric topology as deterministic strict DOT.
#[must_use]
pub fn topology_to_dot<G>(graph: &G) -> String
where
    G: IndexGraphView,
{
    render_dot(
        "digraph",
        "->",
        graph.node_indices().map(G::node_slot).collect(),
        graph
            .edge_references()
            .map(|(_, edge)| (G::node_slot(edge.source()), G::node_slot(edge.target())))
            .collect(),
    )
}

/// Exports an undirected numeric topology as deterministic strict DOT.
#[must_use]
pub fn undirected_to_dot<G>(graph: &G) -> String
where
    G: IndexUndirectedGraphView,
{
    render_dot(
        "graph",
        "--",
        graph.node_indices().map(G::node_slot).collect(),
        graph
            .edge_indices()
            .filter_map(|edge| graph.edge_endpoints(edge))
            .map(|edge| (G::node_slot(edge.source()), G::node_slot(edge.target())))
            .collect(),
    )
}

/// Imports the numeric strict-DOT subset emitted by [`topology_to_dot`].
///
/// Node and edge attributes are ignored. Numeric ids may be quoted or prefixed
/// with `n`; chained edges are accepted.
///
/// # Errors
///
/// Returns an error for malformed headers, ids, or directed edge statements.
pub fn topology_from_dot(input: &str) -> Result<Topology> {
    let parsed = parse_dot(input, DotKind::Directed)?;
    Topology::try_from_edges(parsed.node_count, parsed.edges)
}

/// Imports the numeric strict-DOT subset emitted by [`undirected_to_dot`].
///
/// # Errors
///
/// Returns an error for malformed headers, ids, or undirected edge statements.
pub fn undirected_from_dot(input: &str) -> Result<UndirectedTopology> {
    let parsed = parse_dot(input, DotKind::Undirected)?;
    UndirectedTopology::try_from_edges(parsed.node_count, parsed.edges)
}

fn render_dot(
    kind: &str,
    arrow: &str,
    mut nodes: Vec<usize>,
    mut edges: Vec<(usize, usize)>,
) -> String {
    nodes.sort_unstable();
    edges.sort_unstable();
    let mut output = format!("strict {kind} G {{\n");
    for node in nodes {
        let _ = writeln!(output, "  {node};");
    }
    for (source, target) in edges {
        let _ = writeln!(output, "  {source} {arrow} {target};");
    }
    output.push_str("}\n");
    output
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DotKind {
    Directed,
    Undirected,
}

struct ParsedDot {
    node_count: usize,
    edges: Vec<EdgeEndpoints>,
}

fn parse_dot(input: &str, expected: DotKind) -> Result<ParsedDot> {
    let open = input
        .find('{')
        .ok_or_else(|| invalid("missing opening brace"))?;
    let close = input
        .rfind('}')
        .ok_or_else(|| invalid("missing closing brace"))?;
    if close <= open {
        return Err(invalid("closing brace precedes graph body"));
    }
    validate_header(&input[..open], expected)?;
    let mut maximum = None;
    let mut edges = Vec::new();
    for raw in input[open + 1..close].split(';') {
        let statement = strip_comments(raw).trim();
        if statement.is_empty() {
            continue;
        }
        let statement = statement.split('[').next().unwrap_or(statement).trim();
        if statement.is_empty() || is_default_attribute(statement) {
            continue;
        }
        let arrow = match expected {
            DotKind::Directed => "->",
            DotKind::Undirected => "--",
        };
        if statement.contains(if expected == DotKind::Directed {
            "--"
        } else {
            "->"
        }) {
            return Err(invalid("edge operator does not match graph kind"));
        }
        let ids = statement
            .split(arrow)
            .map(parse_id)
            .collect::<Result<Vec<_>>>()?;
        if ids.len() == 1 {
            maximum = Some(maximum.map_or(ids[0], |value: usize| value.max(ids[0])));
            continue;
        }
        for pair in ids.windows(2) {
            maximum = Some(maximum.unwrap_or(0).max(pair[0]).max(pair[1]));
            edges.push(EdgeEndpoints::new(node(pair[0])?, node(pair[1])?));
        }
    }
    let node_count = maximum.map_or(0, |node| node.saturating_add(1));
    Ok(ParsedDot { node_count, edges })
}

fn validate_header(header: &str, expected: DotKind) -> Result<()> {
    let header = header.trim().to_ascii_lowercase();
    let directed = header
        .split_ascii_whitespace()
        .any(|token| token == "digraph");
    let undirected = header
        .split_ascii_whitespace()
        .any(|token| token == "graph");
    match expected {
        DotKind::Directed if directed => Ok(()),
        DotKind::Undirected if undirected && !directed => Ok(()),
        _ => Err(invalid("graph header has the wrong kind")),
    }
}

fn parse_id(value: &str) -> Result<usize> {
    let value = value.trim();
    let value = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value);
    let value = value
        .strip_prefix('n')
        .filter(|rest| !rest.is_empty())
        .unwrap_or(value);
    value
        .trim()
        .parse()
        .map_err(|_| invalid("node ids must be compact nonnegative integers"))
}

fn strip_comments(value: &str) -> &str {
    let value = value.split_once("//").map_or(value, |(before, _)| before);
    value.split_once('#').map_or(value, |(before, _)| before)
}

fn is_default_attribute(statement: &str) -> bool {
    ["graph", "node", "edge"]
        .iter()
        .any(|prefix| statement.starts_with(prefix))
}

fn node(index: usize) -> Result<NodeIndex> {
    u32::try_from(index)
        .map(NodeIndex::new)
        .map_err(|_| GraphError::IndexCapacityExceeded {
            category: "DOT nodes",
            count: index.saturating_add(1),
        })
}

fn invalid(reason: &str) -> GraphError {
    GraphError::InvalidFormat {
        format: "DOT",
        reason: String::from(reason),
    }
}
