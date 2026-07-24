use crate::Vec;
use crate::{EdgeEndpoints, IndexGraphView};

#[must_use]
pub fn strongly_connected_components<G>(graph: &G) -> Vec<Vec<G::Node>>
where
    G: IndexGraphView,
{
    with_filter(graph, &|_| true)
}

#[must_use]
pub fn strongly_connected_components_filtered<G, F>(graph: &G, allows_edge: F) -> Vec<Vec<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> bool,
{
    with_filter(graph, &allows_edge)
}

pub(super) fn with_filter<G, F>(graph: &G, allows_edge: &F) -> Vec<Vec<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> bool,
{
    let mut seen = vec![false; graph.node_bound()];
    let mut finish = Vec::with_capacity(graph.node_count());
    for start in graph.node_indices() {
        finish_from(graph, start, allows_edge, &mut seen, &mut finish);
    }

    seen.fill(false);
    let mut components = Vec::new();
    for start in finish.into_iter().rev() {
        if seen[G::node_slot(start)] {
            continue;
        }
        let mut component = Vec::new();
        let mut stack = vec![start];
        seen[G::node_slot(start)] = true;
        while let Some(node) = stack.pop() {
            component.push(node);
            for edge in graph.incoming_edges(node) {
                if !allows_edge(edge) {
                    continue;
                }
                let Some(source) = graph.edge_endpoints(edge).map(EdgeEndpoints::source) else {
                    continue;
                };
                let slot = G::node_slot(source);
                if !seen[slot] {
                    seen[slot] = true;
                    stack.push(source);
                }
            }
        }
        components.push(component);
    }
    components
}

fn finish_from<G, F>(
    graph: &G,
    start: G::Node,
    allows_edge: &F,
    seen: &mut [bool],
    finish: &mut Vec<G::Node>,
) where
    G: IndexGraphView,
    F: Fn(G::Edge) -> bool,
{
    if seen[G::node_slot(start)] {
        return;
    }
    let mut stack = vec![(start, false)];
    while let Some((node, exiting)) = stack.pop() {
        let slot = G::node_slot(node);
        if exiting {
            finish.push(node);
            continue;
        }
        if seen[slot] {
            continue;
        }
        seen[slot] = true;
        stack.push((node, true));
        for edge in graph.outgoing_edges(node) {
            if !allows_edge(edge) {
                continue;
            }
            let Some(target) = graph.edge_endpoints(edge).map(EdgeEndpoints::target) else {
                continue;
            };
            if !seen[G::node_slot(target)] {
                stack.push((target, false));
            }
        }
    }
}
