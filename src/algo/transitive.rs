use super::topological_sort_filtered;
use crate::Vec;
use crate::{EdgeEndpoints, IndexGraphView};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagTransitive<Node> {
    reduction: Vec<EdgeEndpoints<Node>>,
    closure: Vec<EdgeEndpoints<Node>>,
}

impl<Node> DagTransitive<Node> {
    #[must_use]
    pub fn reduction_edges(&self) -> &[EdgeEndpoints<Node>] {
        &self.reduction
    }

    #[must_use]
    pub fn closure_edges(&self) -> &[EdgeEndpoints<Node>] {
        &self.closure
    }

    #[must_use]
    pub fn into_parts(self) -> (Vec<EdgeEndpoints<Node>>, Vec<EdgeEndpoints<Node>>) {
        (self.reduction, self.closure)
    }
}

/// Computes a DAG's unique transitive reduction and transitive closure.
///
/// Returns `None` when the selected graph contains a directed cycle.
#[must_use]
pub fn dag_transitive_reduction_closure<G>(graph: &G) -> Option<DagTransitive<G::Node>>
where
    G: IndexGraphView,
{
    dag_transitive_reduction_closure_filtered(graph, |_| true)
}

/// Computes transitive reduction and closure over selected edges.
///
/// Results are deterministically ordered by the graph's topological order.
/// Returns `None` when the selected graph contains a directed cycle.
#[must_use]
pub fn dag_transitive_reduction_closure_filtered<G, F>(
    graph: &G,
    allows_edge: F,
) -> Option<DagTransitive<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> bool,
{
    let order = topological_sort_filtered(graph, &allows_edge)?;
    let mut position = vec![None; graph.node_bound()];
    for (index, &node) in order.iter().enumerate() {
        position[G::node_slot(node)] = Some(index);
    }
    let mut successors = vec![Vec::new(); order.len()];
    for (edge, endpoints) in graph.edge_references() {
        if !allows_edge(edge) {
            continue;
        }
        let source = position[G::node_slot(endpoints.source())]?;
        let target = position[G::node_slot(endpoints.target())]?;
        successors[source].push(target);
    }
    for targets in &mut successors {
        targets.sort_unstable();
        targets.dedup();
    }

    let words = order.len().div_ceil(u64::BITS as usize);
    let mut reachable = vec![vec![0_u64; words]; order.len()];
    let mut reduction = Vec::new();
    for source in (0..order.len()).rev() {
        for &target in &successors[source] {
            if contains(&reachable[source], target) {
                continue;
            }
            reduction.push((source, target));
            let (before_target, target_and_after) = reachable.split_at_mut(target);
            let source_row = &mut before_target[source];
            let target_row = &target_and_after[0];
            insert(source_row, target);
            for (word, inherited) in source_row.iter_mut().zip(target_row) {
                *word |= inherited;
            }
        }
    }
    reduction.sort_unstable();
    let reduction = reduction
        .into_iter()
        .map(|(source, target)| EdgeEndpoints::new(order[source], order[target]))
        .collect();
    let mut closure = Vec::new();
    for (source, row) in reachable.iter().enumerate() {
        for target in 0..order.len() {
            if contains(row, target) {
                closure.push(EdgeEndpoints::new(order[source], order[target]));
            }
        }
    }
    Some(DagTransitive { reduction, closure })
}

fn contains(bits: &[u64], index: usize) -> bool {
    bits[index / u64::BITS as usize] & (1_u64 << (index % u64::BITS as usize)) != 0
}

fn insert(bits: &mut [u64], index: usize) {
    bits[index / u64::BITS as usize] |= 1_u64 << (index % u64::BITS as usize);
}
