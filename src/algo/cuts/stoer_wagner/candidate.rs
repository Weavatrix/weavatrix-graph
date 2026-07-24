use crate::{GraphError, IndexUndirectedGraphView, Measure, Result, String, Vec};
use core::cmp::Ordering;

pub(super) struct Candidate<M> {
    pub(super) weight: M,
    pub(super) partition: Vec<usize>,
    pub(super) complement: Vec<usize>,
}

pub(super) fn canonical_candidate<M: Measure>(
    weight: M,
    group: &[usize],
    nodes: &[usize],
) -> Candidate<M> {
    let mut partition = group.to_vec();
    partition.sort_unstable();
    let complement = nodes
        .iter()
        .copied()
        .filter(|node| partition.binary_search(node).is_err())
        .collect::<Vec<_>>();
    if partition.len() > complement.len()
        || (partition.len() == complement.len() && partition > complement)
    {
        Candidate {
            weight,
            partition: complement,
            complement: partition,
        }
    } else {
        Candidate {
            weight,
            partition,
            complement,
        }
    }
}

pub(super) fn better<M: Measure>(candidate: &Candidate<M>, best: Option<&Candidate<M>>) -> bool {
    let Some(best) = best else {
        return true;
    };
    match candidate.weight.compare(best.weight) {
        Some(Ordering::Less) => true,
        Some(Ordering::Equal) => candidate.partition < best.partition,
        _ => false,
    }
}

pub(super) fn nodes<G: IndexUndirectedGraphView>(graph: &G) -> (Vec<Option<G::Node>>, Vec<usize>) {
    let mut by_slot = vec![None; graph.node_bound()];
    let mut slots = Vec::with_capacity(graph.node_count());
    for node in graph.node_indices() {
        let slot = G::node_slot(node);
        by_slot[slot] = Some(node);
        slots.push(slot);
    }
    slots.sort_unstable();
    (by_slot, slots)
}

pub(super) fn map_nodes<Node: Copy>(slots: &[usize], nodes: &[Option<Node>]) -> Vec<Node> {
    slots
        .iter()
        .map(|&slot| nodes[slot].expect("active slot has a node"))
        .collect()
}

pub(super) fn validate_weight<M: Measure>(weight: M) -> Result<()> {
    if weight.is_valid() && !weight.is_negative() {
        Ok(())
    } else {
        Err(GraphError::InvalidAlgorithmParameter {
            algorithm: "Stoer-Wagner",
            parameter: "edge weight",
            value: String::from("must be finite and non-negative"),
        })
    }
}
