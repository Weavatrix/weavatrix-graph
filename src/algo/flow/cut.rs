use crate::IndexGraphView;
use crate::Vec;
use alloc::collections::VecDeque;

pub(super) fn indexed_nodes<G: IndexGraphView>(graph: &G) -> Vec<Option<G::Node>> {
    let mut nodes = vec![None; graph.node_bound()];
    for node in graph.node_indices() {
        nodes[G::node_slot(node)] = Some(node);
    }
    nodes
}

pub(super) fn residual_reachable<G>(
    graph: &G,
    source: G::Node,
    capacities: &[u64],
    flows: &[u64],
) -> Vec<bool>
where
    G: IndexGraphView,
{
    let nodes = indexed_nodes(graph);
    let mut seen = vec![false; graph.node_bound()];
    let source = G::node_slot(source);
    seen[source] = true;
    let mut queue = VecDeque::from([source]);
    while let Some(node) = queue.pop_front() {
        let Some(node_key) = nodes[node] else {
            continue;
        };
        for edge in graph.outgoing_edges(node_key) {
            let slot = G::edge_slot(edge);
            if capacities[slot] > flows[slot]
                && let Some(target) = graph.edge_endpoints(edge).map(|e| G::node_slot(e.target()))
            {
                push_unseen(target, &mut seen, &mut queue);
            }
        }
        for edge in graph.incoming_edges(node_key) {
            if flows[G::edge_slot(edge)] > 0
                && let Some(target) = graph.edge_endpoints(edge).map(|e| G::node_slot(e.source()))
            {
                push_unseen(target, &mut seen, &mut queue);
            }
        }
    }
    seen
}

fn push_unseen(node: usize, seen: &mut [bool], queue: &mut VecDeque<usize>) {
    if !seen[node] {
        seen[node] = true;
        queue.push_back(node);
    }
}
