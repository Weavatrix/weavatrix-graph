use super::strongly_connected_components_filtered;
use crate::IndexGraphView;
use crate::Vec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleEnumeration<Node> {
    paths: Vec<Vec<Node>>,
    truncated: bool,
}

impl<Node> CycleEnumeration<Node> {
    #[must_use]
    pub fn paths(&self) -> &[Vec<Node>] {
        &self.paths
    }

    #[must_use]
    pub const fn truncated(&self) -> bool {
        self.truncated
    }
}

/// Enumerates elementary directed circuits with Johnson's blocked-set algorithm.
///
/// Every returned circuit repeats its start node at the end. `max_cycles`
/// bounds exponential output.
pub fn johnson_cycles<G>(graph: &G, max_cycles: usize) -> CycleEnumeration<G::Node>
where
    G: IndexGraphView,
{
    if max_cycles == 0 {
        return CycleEnumeration {
            paths: Vec::new(),
            truncated: false,
        };
    }
    let (by_slot, adjacency) = indexed_adjacency(graph);
    let mut state = JohnsonState::new(graph.node_bound(), max_cycles);
    let mut lower = 0;
    while let Some(component) = least_cyclic_component(graph, &adjacency, lower) {
        let Some(start) = component.iter().min().copied() else {
            break;
        };
        let mut allowed = vec![false; graph.node_bound()];
        for &node in &component {
            allowed[node] = true;
        }
        state.reset();
        circuit(start, start, &adjacency, &allowed, &by_slot, &mut state);
        if state.truncated {
            break;
        }
        lower = start + 1;
    }
    CycleEnumeration {
        paths: state.results,
        truncated: state.truncated,
    }
}

fn indexed_adjacency<G: IndexGraphView>(graph: &G) -> (Vec<Option<G::Node>>, Vec<Vec<usize>>) {
    let mut by_slot = vec![None; graph.node_bound()];
    let mut adjacency = vec![Vec::new(); graph.node_bound()];
    for node in graph.node_indices() {
        let source = G::node_slot(node);
        by_slot[source] = Some(node);
        adjacency[source] = graph
            .outgoing_edges(node)
            .filter_map(|edge| graph.edge_endpoints(edge))
            .map(|endpoints| G::node_slot(endpoints.target()))
            .collect();
        adjacency[source].sort_unstable();
        adjacency[source].dedup();
    }
    (by_slot, adjacency)
}

fn least_cyclic_component<G>(
    graph: &G,
    adjacency: &[Vec<usize>],
    lower: usize,
) -> Option<Vec<usize>>
where
    G: IndexGraphView,
{
    strongly_connected_components_filtered(graph, |edge| {
        graph.edge_endpoints(edge).is_some_and(|endpoints| {
            G::node_slot(endpoints.source()) >= lower && G::node_slot(endpoints.target()) >= lower
        })
    })
    .into_iter()
    .map(|component| component.into_iter().map(G::node_slot).collect::<Vec<_>>())
    .filter(|component| {
        component.iter().all(|node| *node >= lower)
            && (component.len() > 1
                || component
                    .first()
                    .is_some_and(|node| adjacency[*node].binary_search(node).is_ok()))
    })
    .min_by_key(|component| component.iter().min().copied())
}

struct JohnsonState<Node> {
    blocked: Vec<bool>,
    blocked_by: Vec<Vec<usize>>,
    stack: Vec<usize>,
    results: Vec<Vec<Node>>,
    limit: usize,
    truncated: bool,
}

impl<Node> JohnsonState<Node> {
    fn new(bound: usize, limit: usize) -> Self {
        Self {
            blocked: vec![false; bound],
            blocked_by: vec![Vec::new(); bound],
            stack: Vec::new(),
            results: Vec::new(),
            limit,
            truncated: false,
        }
    }

    fn reset(&mut self) {
        self.blocked.fill(false);
        for dependencies in &mut self.blocked_by {
            dependencies.clear();
        }
        self.stack.clear();
    }
}

fn circuit<Node: Copy>(
    node: usize,
    start: usize,
    adjacency: &[Vec<usize>],
    allowed: &[bool],
    by_slot: &[Option<Node>],
    state: &mut JohnsonState<Node>,
) -> bool {
    let Some(start_node) = by_slot[start] else {
        return false;
    };
    let mut found = false;
    state.stack.push(node);
    state.blocked[node] = true;
    for &neighbor in &adjacency[node] {
        if !allowed[neighbor] {
            continue;
        }
        if neighbor == start {
            if state.results.len() == state.limit {
                state.truncated = true;
                break;
            }
            let mut cycle = state
                .stack
                .iter()
                .filter_map(|slot| by_slot[*slot])
                .collect::<Vec<_>>();
            cycle.push(start_node);
            state.results.push(cycle);
            found = true;
        } else if !state.blocked[neighbor]
            && circuit(neighbor, start, adjacency, allowed, by_slot, state)
        {
            found = true;
        }
        if state.truncated {
            break;
        }
    }
    if found {
        unblock(node, state);
    } else {
        for &neighbor in &adjacency[node] {
            if allowed[neighbor] && !state.blocked_by[neighbor].contains(&node) {
                state.blocked_by[neighbor].push(node);
            }
        }
    }
    state.stack.pop();
    found
}

fn unblock<Node>(node: usize, state: &mut JohnsonState<Node>) {
    state.blocked[node] = false;
    let dependencies = core::mem::take(&mut state.blocked_by[node]);
    for dependency in dependencies {
        if state.blocked[dependency] {
            unblock(dependency, state);
        }
    }
}
