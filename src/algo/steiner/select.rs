use super::disjoint::DisjointSet;
use crate::IndexUndirectedGraphView;
use crate::Vec;
use alloc::collections::VecDeque;

pub(super) fn candidate_tree<G, F>(
    graph: &G,
    terminals: &[G::Node],
    selected: &[bool],
    edge_cost: &F,
) -> Vec<G::Edge>
where
    G: IndexUndirectedGraphView,
    F: Fn(G::Edge) -> u64,
{
    let mut edges = minimum_selected_tree(graph, selected, edge_cost);
    prune_non_terminal_leaves(graph, terminals, &mut edges);
    edges.sort_unstable_by_key(|edge| G::edge_slot(*edge));
    edges
}

fn minimum_selected_tree<G, F>(graph: &G, selected: &[bool], edge_cost: &F) -> Vec<G::Edge>
where
    G: IndexUndirectedGraphView,
    F: Fn(G::Edge) -> u64,
{
    let mut edges = graph
        .edge_indices()
        .filter(|edge| selected[G::edge_slot(*edge)])
        .collect::<Vec<_>>();
    edges.sort_unstable_by_key(|edge| (edge_cost(*edge), G::edge_slot(*edge)));
    let mut sets = DisjointSet::new(graph.node_bound());
    edges
        .into_iter()
        .filter(|edge| {
            graph.edge_endpoints(*edge).is_some_and(|endpoints| {
                sets.union(
                    G::node_slot(endpoints.source()),
                    G::node_slot(endpoints.target()),
                )
            })
        })
        .collect()
}

fn prune_non_terminal_leaves<G>(graph: &G, terminals: &[G::Node], edges: &mut Vec<G::Edge>)
where
    G: IndexUndirectedGraphView,
{
    let mut selected = vec![false; graph.edge_bound()];
    let mut degree = vec![0_usize; graph.node_bound()];
    let mut terminal = vec![false; graph.node_bound()];
    let mut incident = vec![Vec::new(); graph.node_bound()];
    for node in terminals {
        terminal[G::node_slot(*node)] = true;
    }
    for &edge in edges.iter() {
        selected[G::edge_slot(edge)] = true;
        if let Some(endpoints) = graph.edge_endpoints(edge) {
            let source = G::node_slot(endpoints.source());
            let target = G::node_slot(endpoints.target());
            degree[source] += 1;
            degree[target] += 1;
            incident[source].push(edge);
            incident[target].push(edge);
        }
    }
    let mut queue = (0..degree.len())
        .filter(|node| !terminal[*node] && degree[*node] == 1)
        .collect::<VecDeque<_>>();
    while let Some(node) = queue.pop_front() {
        let edge = incident[node]
            .iter()
            .copied()
            .find(|edge| selected[G::edge_slot(*edge)]);
        let Some(edge) = edge else {
            continue;
        };
        selected[G::edge_slot(edge)] = false;
        if let Some(endpoints) = graph.edge_endpoints(edge) {
            for slot in [
                G::node_slot(endpoints.source()),
                G::node_slot(endpoints.target()),
            ] {
                degree[slot] = degree[slot].saturating_sub(1);
                if !terminal[slot] && degree[slot] == 1 {
                    queue.push_back(slot);
                }
            }
        }
    }
    edges.retain(|edge| selected[G::edge_slot(*edge)]);
}
