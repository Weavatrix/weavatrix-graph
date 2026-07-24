use crate::IndexGraphView;
use crate::Vec;
use alloc::collections::VecDeque;

#[must_use]
pub fn topological_sort<G>(graph: &G) -> Option<Vec<G::Node>>
where
    G: IndexGraphView,
{
    sort_with_filter(graph, &|_| true)
}

#[must_use]
pub fn topological_sort_filtered<G, F>(graph: &G, allows_edge: F) -> Option<Vec<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> bool,
{
    sort_with_filter(graph, &allows_edge)
}

#[must_use]
pub fn topological_generations<G>(graph: &G) -> Option<Vec<Vec<G::Node>>>
where
    G: IndexGraphView,
{
    generations_with_filter(graph, &|_| true)
}

#[must_use]
pub fn topological_generations_filtered<G, F>(
    graph: &G,
    allows_edge: F,
) -> Option<Vec<Vec<G::Node>>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> bool,
{
    generations_with_filter(graph, &allows_edge)
}

#[must_use]
pub fn has_cycle<G>(graph: &G) -> bool
where
    G: IndexGraphView,
{
    topological_sort(graph).is_none()
}

#[must_use]
pub fn has_cycle_filtered<G, F>(graph: &G, allows_edge: F) -> bool
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> bool,
{
    topological_sort_filtered(graph, allows_edge).is_none()
}

#[must_use]
pub fn find_cycle<G>(graph: &G) -> Option<Vec<G::Node>>
where
    G: IndexGraphView,
{
    find_with_filter(graph, &|_| true)
}

#[must_use]
pub fn find_cycle_filtered<G, F>(graph: &G, allows_edge: F) -> Option<Vec<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> bool,
{
    find_with_filter(graph, &allows_edge)
}

fn sort_with_filter<G, F>(graph: &G, allows_edge: &F) -> Option<Vec<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> bool,
{
    let mut indegree = vec![0_usize; graph.node_bound()];
    for (edge, endpoints) in graph.edge_references() {
        if !allows_edge(edge) {
            continue;
        }
        if let Some(degree) = indegree.get_mut(G::node_slot(endpoints.target())) {
            *degree += 1;
        }
    }
    let mut ready = graph
        .node_indices()
        .filter(|node| indegree[G::node_slot(*node)] == 0)
        .collect::<VecDeque<_>>();
    let mut order = Vec::with_capacity(graph.node_count());
    while let Some(node) = ready.pop_front() {
        order.push(node);
        for edge in graph.outgoing_edges(node) {
            if !allows_edge(edge) {
                continue;
            }
            let Some(endpoints) = graph.edge_endpoints(edge) else {
                continue;
            };
            let Some(degree) = indegree.get_mut(G::node_slot(endpoints.target())) else {
                continue;
            };
            *degree = degree.saturating_sub(1);
            if *degree == 0 {
                ready.push_back(endpoints.target());
            }
        }
    }
    (order.len() == graph.node_count()).then_some(order)
}

fn generations_with_filter<G, F>(graph: &G, allows_edge: &F) -> Option<Vec<Vec<G::Node>>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> bool,
{
    let mut indegree = vec![0_usize; graph.node_bound()];
    for (edge, endpoints) in graph.edge_references() {
        if allows_edge(edge) {
            indegree[G::node_slot(endpoints.target())] += 1;
        }
    }
    let mut ready = graph
        .node_indices()
        .filter(|node| indegree[G::node_slot(*node)] == 0)
        .collect::<Vec<_>>();
    let mut generations = Vec::new();
    let mut visited = 0_usize;
    while !ready.is_empty() {
        visited += ready.len();
        let generation = ready;
        let mut next = Vec::new();
        for &node in &generation {
            for edge in graph.outgoing_edges(node) {
                if !allows_edge(edge) {
                    continue;
                }
                let Some(endpoints) = graph.edge_endpoints(edge) else {
                    continue;
                };
                let degree = &mut indegree[G::node_slot(endpoints.target())];
                *degree = degree.saturating_sub(1);
                if *degree == 0 {
                    next.push(endpoints.target());
                }
            }
        }
        generations.push(generation);
        ready = next;
    }
    (visited == graph.node_count()).then_some(generations)
}

fn find_with_filter<G, F>(graph: &G, allows_edge: &F) -> Option<Vec<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> bool,
{
    let mut adjacency = vec![Vec::new(); graph.node_bound()];
    for (edge, endpoints) in graph.edge_references() {
        if !allows_edge(edge) {
            continue;
        }
        if let Some(neighbors) = adjacency.get_mut(G::node_slot(endpoints.source())) {
            neighbors.push(endpoints.target());
        }
    }
    let mut color = vec![0_u8; graph.node_bound()];
    let mut parent = vec![None; graph.node_bound()];
    for start in graph.node_indices() {
        if color[G::node_slot(start)] != 0 {
            continue;
        }
        color[G::node_slot(start)] = 1;
        let mut stack = vec![(start, 0_usize)];
        while let Some((node, next)) = stack.last_mut() {
            let slot = G::node_slot(*node);
            let Some(&target) = adjacency[slot].get(*next) else {
                color[slot] = 2;
                stack.pop();
                continue;
            };
            *next += 1;
            let target_slot = G::node_slot(target);
            if color[target_slot] == 0 {
                parent[target_slot] = Some(*node);
                color[target_slot] = 1;
                stack.push((target, 0));
            } else if color[target_slot] == 1 {
                return reconstruct_cycle::<G>(*node, target, &parent);
            }
        }
    }
    None
}

fn reconstruct_cycle<G>(
    node: G::Node,
    target: G::Node,
    parent: &[Option<G::Node>],
) -> Option<Vec<G::Node>>
where
    G: IndexGraphView,
{
    let mut cycle = vec![node];
    let mut cursor = node;
    while cursor != target {
        cursor = parent[G::node_slot(cursor)]?;
        cycle.push(cursor);
    }
    cycle.reverse();
    cycle.push(target);
    Some(cycle)
}
