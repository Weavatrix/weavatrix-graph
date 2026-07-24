use crate::IndexUndirectedGraphView;
use crate::Vec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliqueEnumeration<Node> {
    cliques: Vec<Vec<Node>>,
    truncated: bool,
}

impl<Node> CliqueEnumeration<Node> {
    #[must_use]
    pub fn cliques(&self) -> &[Vec<Node>] {
        &self.cliques
    }

    #[must_use]
    pub const fn truncated(&self) -> bool {
        self.truncated
    }
}

pub fn maximal_cliques<G>(graph: &G, max_cliques: usize) -> CliqueEnumeration<G::Node>
where
    G: IndexUndirectedGraphView,
{
    if max_cliques == 0 {
        return CliqueEnumeration {
            cliques: Vec::new(),
            truncated: false,
        };
    }
    let (nodes, adjacency) = matrix(graph);
    let candidates = nodes
        .iter()
        .enumerate()
        .filter_map(|(slot, node)| node.is_some().then_some(slot))
        .collect();
    let mut state = CliqueState {
        current: Vec::new(),
        cliques: Vec::new(),
        limit: max_cliques,
        truncated: false,
    };
    bron_kerbosch(&adjacency, candidates, Vec::new(), &nodes, &mut state);
    CliqueEnumeration {
        cliques: state.cliques,
        truncated: state.truncated,
    }
}

fn matrix<G: IndexUndirectedGraphView>(graph: &G) -> (Vec<Option<G::Node>>, Vec<Vec<bool>>) {
    let mut nodes = vec![None; graph.node_bound()];
    let mut adjacency = vec![vec![false; graph.node_bound()]; graph.node_bound()];
    for node in graph.node_indices() {
        let source = G::node_slot(node);
        nodes[source] = Some(node);
        for edge in graph.incident_edges(node) {
            if let Some(target) = graph.opposite(edge, node) {
                let target = G::node_slot(target);
                if source != target {
                    adjacency[source][target] = true;
                }
            }
        }
    }
    (nodes, adjacency)
}

struct CliqueState<Node> {
    current: Vec<usize>,
    cliques: Vec<Vec<Node>>,
    limit: usize,
    truncated: bool,
}

fn bron_kerbosch<Node: Copy>(
    adjacency: &[Vec<bool>],
    mut candidates: Vec<usize>,
    mut excluded: Vec<usize>,
    nodes: &[Option<Node>],
    state: &mut CliqueState<Node>,
) {
    if candidates.is_empty() && excluded.is_empty() {
        if state.cliques.len() == state.limit {
            state.truncated = true;
            return;
        }
        let mut clique = state
            .current
            .iter()
            .filter_map(|slot| nodes[*slot].map(|node| (*slot, node)))
            .collect::<Vec<_>>();
        clique.sort_unstable_by_key(|(slot, _)| *slot);
        state
            .cliques
            .push(clique.into_iter().map(|(_, node)| node).collect());
        return;
    }
    let pivot = candidates
        .iter()
        .chain(&excluded)
        .copied()
        .max_by_key(|pivot| {
            candidates
                .iter()
                .filter(|candidate| adjacency[*pivot][**candidate])
                .count()
        });
    let branch = candidates
        .iter()
        .copied()
        .filter(|candidate| pivot.is_none_or(|pivot| !adjacency[pivot][*candidate]))
        .collect::<Vec<_>>();
    for node in branch {
        state.current.push(node);
        bron_kerbosch(
            adjacency,
            candidates
                .iter()
                .copied()
                .filter(|candidate| adjacency[node][*candidate])
                .collect(),
            excluded
                .iter()
                .copied()
                .filter(|candidate| adjacency[node][*candidate])
                .collect(),
            nodes,
            state,
        );
        state.current.pop();
        if state.truncated {
            return;
        }
        candidates.retain(|candidate| *candidate != node);
        excluded.push(node);
    }
}
