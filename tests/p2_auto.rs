use std::cell::Cell;
use weavatrix_graph::{
    AllPairsStrategy, EdgeEndpoints, GraphView, NodeIndex, Topology, all_pairs_auto,
    all_pairs_auto_filtered, floyd_warshall, johnson_all_pairs,
};

#[test]
fn auto_all_pairs_selects_dense_and_sparse_strategies() {
    let dense = graph(
        80,
        (0..80).flat_map(|source| {
            (0..80)
                .filter(move |target| *target != source)
                .map(move |target| (source, target))
        }),
    );
    assert_eq!(
        all_pairs_auto(&dense, |_| 1).unwrap().strategy(),
        AllPairsStrategy::FloydWarshall
    );

    let sparse = graph(100, (1..100).map(|target| (target - 1, target)));
    assert_eq!(
        all_pairs_auto(&sparse, |_| 1).unwrap().strategy(),
        AllPairsStrategy::Johnson
    );
}

#[test]
fn automatic_selection_preserves_reference_distances_and_paths() {
    for dense in [false, true] {
        let edges = (0_usize..90).flat_map(|source| {
            (0_usize..90).filter_map(move |target| {
                let selected =
                    source != target && (dense || (source * 31 + target * 17).is_multiple_of(97));
                selected.then_some((source, target))
            })
        });
        let graph = graph(90, edges);
        let weights = (0..graph.edge_count())
            .map(|edge| i64::try_from(edge % 13 + 1).unwrap())
            .collect::<Vec<_>>();
        let auto = all_pairs_auto(&graph, |edge| weights[edge.index()]).unwrap();
        let reference = match auto.strategy() {
            AllPairsStrategy::FloydWarshall => {
                floyd_warshall(&graph, |edge| weights[edge.index()]).unwrap()
            }
            AllPairsStrategy::Johnson => {
                johnson_all_pairs(&graph, |edge| weights[edge.index()]).unwrap()
            }
        };
        for source in graph.node_indices() {
            for target in graph.node_indices() {
                assert_eq!(
                    auto.paths().distance(source, target),
                    reference.distance(source, target)
                );
                assert_eq!(
                    auto.paths().path(source, target),
                    reference.path(source, target)
                );
            }
        }
    }
}

#[test]
fn filtered_auto_snapshots_each_edge_weight_once() {
    let graph = graph(70, (1..70).map(|target| (target - 1, target)));
    let calls = Cell::new(0);
    let result = all_pairs_auto_filtered(&graph, |edge| {
        calls.set(calls.get() + 1);
        edge.index().is_multiple_of(2).then_some(1)
    })
    .unwrap();
    assert_eq!(calls.get(), graph.edge_count());
    assert_eq!(result.strategy(), AllPairsStrategy::Johnson);
}

fn graph(node_count: usize, edges: impl IntoIterator<Item = (usize, usize)>) -> Topology {
    Topology::try_from_edges(
        node_count,
        edges
            .into_iter()
            .map(|(source, target)| EdgeEndpoints::new(node(source), node(target))),
    )
    .unwrap()
}

fn node(index: usize) -> NodeIndex {
    NodeIndex::new(u32::try_from(index).unwrap())
}
