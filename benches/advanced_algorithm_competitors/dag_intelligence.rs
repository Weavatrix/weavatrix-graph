use super::support::{measure, print_measurement, topology_pairs};
use super::{dag_pairs, node, ours_graph, positive_weight, weighted_pet};
use petgraph::algo::{dominators::simple_fast, toposort};
use petgraph::graph::{DiGraph, NodeIndex as PetNodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::BTreeSet;
use std::hint::black_box;
use weavatrix_graph::{dag_weighted_longest_path, dominance_frontiers, topological_generations};

pub(super) fn compare() {
    compare_dag_paths();
    compare_generations();
    compare_frontiers();
}

fn compare_dag_paths() {
    const NODES: usize = 10_000;
    let pairs = dag_pairs(NODES, 30_000);
    let weights = pairs
        .iter()
        .copied()
        .map(positive_weight)
        .collect::<Vec<_>>();
    let ours = ours_graph(NODES, &pairs);
    let (pet, _) = weighted_pet(NODES, &pairs, positive_weight);
    let actual = dag_weighted_longest_path(&ours, |edge| Some(weights[edge.index()]))
        .unwrap()
        .unwrap();
    let expected = pet_longest_path(&pet);
    assert_eq!(actual.total_cost(), expected.1);
    let ours_measurement = measure(|| {
        black_box(dag_weighted_longest_path(&ours, |edge| {
            Some(weights[edge.index()])
        }))
    });
    let pet_measurement = measure(|| black_box(pet_longest_path(&pet)));
    print_measurement(
        "dag-longest-path",
        "library=weavatrix-graph",
        &ours_measurement,
    );
    print_measurement(
        "dag-longest-path",
        "library=petgraph-adapter",
        &pet_measurement,
    );
}

fn compare_generations() {
    const NODES: usize = 10_000;
    let pairs = dag_pairs(NODES, 30_000);
    let ours = ours_graph(NODES, &pairs);
    let (pet, _) = weighted_pet(NODES, &pairs, |_| ());
    let actual = topological_generations(&ours).unwrap();
    let expected = pet_generations(&pet);
    assert_eq!(actual.len(), expected.len());
    assert_eq!(
        actual.iter().map(Vec::len).collect::<Vec<_>>(),
        expected.iter().map(Vec::len).collect::<Vec<_>>()
    );
    let ours_measurement = measure(|| black_box(topological_generations(&ours)));
    let pet_measurement = measure(|| black_box(pet_generations(&pet)));
    print_measurement(
        "topological-generations",
        "library=weavatrix-graph",
        &ours_measurement,
    );
    print_measurement(
        "topological-generations",
        "library=petgraph-adapter",
        &pet_measurement,
    );
}

fn compare_frontiers() {
    const NODES: usize = 10_000;
    let pairs = topology_pairs(NODES, 30_000);
    let ours = ours_graph(NODES, &pairs);
    let (pet, nodes) = weighted_pet(NODES, &pairs, |_| ());
    let actual = dominance_frontiers(&ours, node(0)).unwrap();
    let expected = pet_frontiers(&pet, &nodes);
    for (index, expected) in expected.iter().enumerate() {
        let actual = actual
            .frontier(node(index))
            .map(|frontier| frontier.iter().map(|node| node.index()).collect());
        assert_eq!(actual.as_ref(), expected.as_ref());
    }
    let ours_measurement = measure(|| black_box(dominance_frontiers(&ours, node(0))));
    let pet_measurement = measure(|| black_box(pet_frontiers(&pet, &nodes)));
    print_measurement(
        "dominance-frontiers",
        "library=weavatrix-graph",
        &ours_measurement,
    );
    print_measurement(
        "dominance-frontiers",
        "library=petgraph-adapter",
        &pet_measurement,
    );
}

fn pet_longest_path(graph: &DiGraph<(), u64>) -> (Vec<PetNodeIndex>, u64) {
    let order = toposort(graph, None).unwrap();
    let mut distances = vec![0_u64; graph.node_count()];
    let mut predecessors = vec![None; graph.node_count()];
    for &source in &order {
        for edge in graph.edges(source) {
            let target = edge.target();
            let candidate = distances[source.index()] + edge.weight();
            if candidate > distances[target.index()] {
                distances[target.index()] = candidate;
                predecessors[target.index()] = Some(source);
            }
        }
    }
    let end = order
        .into_iter()
        .max_by_key(|node| distances[node.index()])
        .unwrap();
    let mut path = vec![end];
    let mut cursor = end;
    while let Some(parent) = predecessors[cursor.index()] {
        path.push(parent);
        cursor = parent;
    }
    path.reverse();
    (path, distances[end.index()])
}

fn pet_generations(graph: &DiGraph<(), ()>) -> Vec<Vec<PetNodeIndex>> {
    let mut indegree = vec![0_usize; graph.node_count()];
    for edge in graph.edge_references() {
        indegree[edge.target().index()] += 1;
    }
    let mut ready = graph
        .node_indices()
        .filter(|node| indegree[node.index()] == 0)
        .collect::<Vec<_>>();
    let mut generations = Vec::new();
    while !ready.is_empty() {
        let generation = ready;
        let mut next = Vec::new();
        for &node in &generation {
            for edge in graph.edges(node) {
                let target = edge.target();
                indegree[target.index()] -= 1;
                if indegree[target.index()] == 0 {
                    next.push(target);
                }
            }
        }
        generations.push(generation);
        ready = next;
    }
    generations
}

fn pet_frontiers(graph: &DiGraph<(), ()>, nodes: &[PetNodeIndex]) -> Vec<Option<BTreeSet<usize>>> {
    let dominators = simple_fast(graph, nodes[0]);
    let mut immediate = vec![None; graph.node_count()];
    immediate[0] = Some(0);
    for node in 1..graph.node_count() {
        immediate[node] = dominators
            .immediate_dominator(nodes[node])
            .map(PetNodeIndex::index);
    }
    let mut predecessors = vec![BTreeSet::new(); graph.node_count()];
    for edge in graph.edge_references() {
        let source = edge.source().index();
        let target = edge.target().index();
        if immediate[source].is_some() && immediate[target].is_some() {
            predecessors[target].insert(source);
        }
    }
    let mut frontiers = vec![BTreeSet::new(); graph.node_count()];
    for join in 0..graph.node_count() {
        if predecessors[join].len() < 2 {
            continue;
        }
        let stop = immediate[join].unwrap();
        for &predecessor in &predecessors[join] {
            let mut runner = predecessor;
            while runner != stop {
                frontiers[runner].insert(join);
                runner = immediate[runner].unwrap();
            }
        }
    }
    immediate
        .into_iter()
        .enumerate()
        .map(|(node, parent)| parent.map(|_| core::mem::take(&mut frontiers[node])))
        .collect()
}
