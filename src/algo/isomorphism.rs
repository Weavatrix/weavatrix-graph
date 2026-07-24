use super::isomorphism_match::edge_lists_compatible;
use crate::IndexGraphView;
use crate::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubgraphMode {
    NonInduced,
    Induced,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsomorphismSearch<PatternNode, TargetNode> {
    mappings: Vec<Vec<(PatternNode, TargetNode)>>,
    truncated: bool,
}

impl<PatternNode, TargetNode> IsomorphismSearch<PatternNode, TargetNode> {
    #[must_use]
    pub fn mappings(&self) -> &[Vec<(PatternNode, TargetNode)>] {
        &self.mappings
    }

    #[must_use]
    pub const fn truncated(&self) -> bool {
        self.truncated
    }
}

pub fn graph_isomorphic<P, T, N, E>(left: &P, right: &T, node_match: N, edge_match: E) -> bool
where
    P: IndexGraphView,
    T: IndexGraphView,
    N: Fn(P::Node, T::Node) -> bool,
    E: Fn(P::Edge, T::Edge) -> bool,
{
    if left.node_count() != right.node_count() || left.edge_count() != right.edge_count() {
        return false;
    }
    !subgraph_isomorphisms(
        left,
        right,
        SubgraphMode::Induced,
        1,
        node_match,
        edge_match,
    )
    .mappings
    .is_empty()
}

pub fn subgraph_isomorphisms<P, T, N, E>(
    pattern: &P,
    target: &T,
    mode: SubgraphMode,
    max_mappings: usize,
    node_match: N,
    edge_match: E,
) -> IsomorphismSearch<P::Node, T::Node>
where
    P: IndexGraphView,
    T: IndexGraphView,
    N: Fn(P::Node, T::Node) -> bool,
    E: Fn(P::Edge, T::Edge) -> bool,
{
    if max_mappings == 0 || pattern.node_count() > target.node_count() {
        return IsomorphismSearch {
            mappings: Vec::new(),
            truncated: false,
        };
    }
    let pattern_matrix = EdgeMatrix::new(pattern);
    let target_matrix = EdgeMatrix::new(target);
    let mut pattern_nodes = pattern.node_indices().collect::<Vec<_>>();
    pattern_nodes.sort_unstable_by_key(|node| {
        let degree = pattern.outgoing_edges(*node).count() + pattern.incoming_edges(*node).count();
        (usize::MAX - degree, P::node_slot(*node))
    });
    let mut target_nodes = target.node_indices().collect::<Vec<_>>();
    target_nodes.sort_unstable_by_key(|node| T::node_slot(*node));
    let mut state = SearchState {
        mapping: vec![None; pattern.node_bound()],
        used: vec![false; target.node_bound()],
        results: Vec::new(),
        limit: max_mappings,
        truncated: false,
    };
    search(
        pattern,
        target,
        &pattern_matrix,
        &target_matrix,
        &pattern_nodes,
        &target_nodes,
        mode,
        &node_match,
        &edge_match,
        0,
        &mut state,
    );
    IsomorphismSearch {
        mappings: state.results,
        truncated: state.truncated,
    }
}

struct EdgeMatrix<Edge> {
    bound: usize,
    cells: Vec<Vec<Edge>>,
}

impl<Edge: Copy> EdgeMatrix<Edge> {
    fn new<G: IndexGraphView<Edge = Edge>>(graph: &G) -> Self {
        let bound = graph.node_bound();
        let mut cells = vec![Vec::new(); bound.saturating_mul(bound)];
        for (edge, endpoints) in graph.edge_references() {
            let source = G::node_slot(endpoints.source());
            let target = G::node_slot(endpoints.target());
            cells[source * bound + target].push(edge);
        }
        Self { bound, cells }
    }

    fn get(&self, source: usize, target: usize) -> &[Edge] {
        &self.cells[source * self.bound + target]
    }
}

struct SearchState<PatternNode, TargetNode> {
    mapping: Vec<Option<TargetNode>>,
    used: Vec<bool>,
    results: Vec<Vec<(PatternNode, TargetNode)>>,
    limit: usize,
    truncated: bool,
}

#[allow(clippy::too_many_arguments)]
fn search<P, T, N, E>(
    pattern: &P,
    target: &T,
    pattern_edges: &EdgeMatrix<P::Edge>,
    target_edges: &EdgeMatrix<T::Edge>,
    pattern_nodes: &[P::Node],
    target_nodes: &[T::Node],
    mode: SubgraphMode,
    node_match: &N,
    edge_match: &E,
    depth: usize,
    state: &mut SearchState<P::Node, T::Node>,
) where
    P: IndexGraphView,
    T: IndexGraphView,
    N: Fn(P::Node, T::Node) -> bool,
    E: Fn(P::Edge, T::Edge) -> bool,
{
    if state.results.len() == state.limit {
        state.truncated = true;
        return;
    }
    if depth == pattern_nodes.len() {
        let mut mapping = pattern_nodes
            .iter()
            .map(|node| {
                (
                    *node,
                    state.mapping[P::node_slot(*node)].expect("complete mapping"),
                )
            })
            .collect::<Vec<_>>();
        mapping.sort_unstable_by_key(|(node, _)| P::node_slot(*node));
        state.results.push(mapping);
        return;
    }
    let pattern_node = pattern_nodes[depth];
    for &target_node in target_nodes {
        let target_slot = T::node_slot(target_node);
        if state.used[target_slot]
            || !node_match(pattern_node, target_node)
            || !degree_compatible(pattern, target, pattern_node, target_node, mode)
            || !edges_compatible::<P, T, E>(
                pattern_edges,
                target_edges,
                pattern_node,
                target_node,
                &state.mapping,
                mode,
                edge_match,
            )
        {
            continue;
        }
        state.mapping[P::node_slot(pattern_node)] = Some(target_node);
        state.used[target_slot] = true;
        search(
            pattern,
            target,
            pattern_edges,
            target_edges,
            pattern_nodes,
            target_nodes,
            mode,
            node_match,
            edge_match,
            depth + 1,
            state,
        );
        state.used[target_slot] = false;
        state.mapping[P::node_slot(pattern_node)] = None;
        if state.truncated {
            return;
        }
    }
}

fn degree_compatible<P, T>(
    pattern: &P,
    target: &T,
    pattern_node: P::Node,
    target_node: T::Node,
    _mode: SubgraphMode,
) -> bool
where
    P: IndexGraphView,
    T: IndexGraphView,
{
    let pattern_degree = (
        pattern.outgoing_edges(pattern_node).count(),
        pattern.incoming_edges(pattern_node).count(),
    );
    let target_degree = (
        target.outgoing_edges(target_node).count(),
        target.incoming_edges(target_node).count(),
    );
    pattern_degree.0 <= target_degree.0 && pattern_degree.1 <= target_degree.1
}

#[allow(clippy::too_many_arguments)]
fn edges_compatible<P, T, E>(
    pattern: &EdgeMatrix<P::Edge>,
    target: &EdgeMatrix<T::Edge>,
    pattern_node: P::Node,
    target_node: T::Node,
    mapping: &[Option<T::Node>],
    mode: SubgraphMode,
    edge_match: &E,
) -> bool
where
    P: IndexGraphView,
    T: IndexGraphView,
    E: Fn(P::Edge, T::Edge) -> bool,
{
    let pattern_slot = P::node_slot(pattern_node);
    let target_slot = T::node_slot(target_node);
    if !edge_lists_compatible(
        pattern.get(pattern_slot, pattern_slot),
        target.get(target_slot, target_slot),
        mode,
        edge_match,
    ) {
        return false;
    }
    mapping.iter().enumerate().all(|(other_pattern, mapped)| {
        let Some(other_target) = mapped else {
            return true;
        };
        let other_target = T::node_slot(*other_target);
        edge_lists_compatible(
            pattern.get(pattern_slot, other_pattern),
            target.get(target_slot, other_target),
            mode,
            edge_match,
        ) && edge_lists_compatible(
            pattern.get(other_pattern, pattern_slot),
            target.get(other_target, target_slot),
            mode,
            edge_match,
        )
    })
}
