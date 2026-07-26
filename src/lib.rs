#![cfg_attr(not(feature = "unsafe-fast"), forbid(unsafe_code))]
#![deny(unsafe_op_in_unsafe_fn)]
#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

#[macro_use]
extern crate alloc;

pub(crate) use alloc::{
    string::{String, ToString},
    vec::Vec,
};

mod algo;
mod attribute;
mod error;
mod filter;
mod format;
mod generator;
mod graph;
mod kind;
mod legacy;
mod matrix;
mod model;
mod operator;
mod payload;
mod topology;
mod traversal_cache;
mod undirected;
mod view;
mod working;

pub use algo::{
    AllPairsShortestPaths, AllPairsStrategy, AutoAllPairs, BellmanFord, Bfs, BiconnectedComponents,
    BipartiteMatching, BipartitePartition, ChainDecomposition, ChainStep, CliqueEnumeration,
    Coloring, Communities, Condensation, CycleEnumeration, DagTransitive, Dfs, DfsEvent,
    DfsEventWorkspace, Dijkstra, DijkstraWorkspace, Direction, DistanceAnalytics,
    DominanceFrontiers, Dominators, DominatorsIter, FeedbackArcSet, Hits, HitsScores,
    IsomorphismSearch, IterativeCentrality, MaxFlow, MaximumMatching, Measure, MinCostFlow,
    PathEnumeration, SignedPath, SpanningForest, SteinerTree, StoerWagnerCut, SubgraphMode,
    TraversalControl, TraversalWorkspace, UndirectedCuts, WeightedPath, all_pairs_auto,
    all_pairs_auto_filtered, all_simple_paths, astar, astar_filtered, bellman_ford,
    bellman_ford_filtered, bellman_ford_measure, bellman_ford_measure_filtered,
    betweenness_centrality, bfs, bfs_filtered, bfs_iter, bfs_iter_filtered, biconnected_components,
    biconnected_components_filtered, bidirectional_dijkstra, bipartite_partition,
    bridges_and_articulation_points, center, chain_decomposition, chain_decomposition_filtered,
    chain_decomposition_from, chain_decomposition_from_filtered, closeness_centrality,
    condensation, condensation_filtered, cycle_basis, dag_longest_path, dag_longest_path_filtered,
    dag_longest_path_length, dag_longest_path_length_filtered, dag_transitive_reduction_closure,
    dag_transitive_reduction_closure_filtered, dag_weighted_longest_path,
    dag_weighted_longest_path_length, degree_centrality, depth_first_search,
    depth_first_search_filtered, dfs, dfs_filtered, dfs_iter, dfs_iter_filtered, diameter,
    dijkstra, dijkstra_filtered, dijkstra_iter, dijkstra_iter_filtered, dijkstra_measure,
    dijkstra_measure_filtered, distance_analytics, distance_analytics_filtered,
    dominance_frontiers, dominance_frontiers_filtered, dominators, dominators_filtered,
    dsatur_coloring, eccentricity, edge_betweenness_centrality,
    edge_betweenness_centrality_filtered, edmonds_karp, eigenvector_centrality,
    feedback_arc_set_heuristic, find_cycle, find_cycle_filtered, floyd_warshall,
    floyd_warshall_filtered, graph_isomorphic, has_cycle, has_cycle_filtered, hits, hits_filtered,
    johnson_all_pairs, johnson_all_pairs_filtered, johnson_cycles, k_core_numbers,
    k_shortest_paths, katz_centrality, label_propagation_communities, maximal_cliques,
    maximum_bipartite_matching, maximum_flow, maximum_matching, min_cost_max_flow,
    minimum_spanning_forest, page_rank, page_rank_filtered, periphery, prim_spanning_forest,
    push_relabel, radius, reachable, reachable_filtered, shortest_path, shortest_path_filtered,
    spfa, spfa_filtered, steiner_tree_approximation, stoer_wagner_min_cut,
    stoer_wagner_min_cut_filtered, strongly_connected_components,
    strongly_connected_components_filtered, subgraph_isomorphisms, topological_generations,
    topological_generations_filtered, topological_sort, topological_sort_filtered,
    undirected_edge_betweenness_centrality, undirected_edge_betweenness_centrality_filtered,
    weakly_connected_components, weakly_connected_components_filtered,
};
#[cfg(feature = "rayon")]
pub use algo::{
    betweenness_centrality_parallel, bfs_batch_parallel, closeness_centrality_parallel,
    dijkstra_batch_parallel, edge_betweenness_centrality_parallel, johnson_all_pairs_parallel,
    undirected_edge_betweenness_centrality_parallel,
};
pub use attribute::{AttributeValue, FiniteF64};
pub use error::{GraphError, Result};
pub use filter::EdgeFilter;
pub use format::{
    GraphMlTopology, graph6_decode, graph6_encode, graphml_decode, topology_from_dot,
    topology_to_dot, topology_to_graphml, undirected_from_dot, undirected_to_dot,
    undirected_to_graphml,
};
pub use generator::{
    RandomGraphGenerator, complete_bipartite_topology, complete_topology, cycle_topology,
    grid_topology, path_topology, star_topology,
};
pub use graph::{Graph, GraphBuilder, GraphNodeIndex};
pub use kind::{EdgeKind, EvidenceKind, NodeKind};
pub use legacy::{LegacyGraph, LegacyLink, LegacyNode, LegacyPoint, LegacyRange};
pub use matrix::{BitMatrix, DenseMatrix};
pub use model::{Confidence, Edge, Node, NodeId, Provenance, SourcePosition, SourceSpan};
pub use operator::{TopologyProjection, complement, union};
pub use payload::{
    AcyclicPayloadGraph, FrozenPayloadGraph, FrozenUndirectedPayloadGraph, KeyedPayloadGraph,
    PayloadFreezeMap, PayloadGraph, StablePayloadGraph, StableUndirectedPayloadGraph,
    UndirectedPayloadGraph,
};
pub use topology::{EdgeEndpoints, EdgeIndex, GraphView, IndexGraphView, NodeIndex, Topology};
pub use traversal_cache::{
    CacheBfs, CacheDfs, NeighborIter, TraversalCache, TraversalCacheWorkspace, TraversalLayout,
    TraversalStorage,
};
pub use undirected::{IndexUndirectedGraphView, UndirectedGraphView, UndirectedTopology};
pub use view::{
    EdgeFiltered, NodeFiltered, Reversed, edge_filtered, induced_subgraph_view, reversed,
};
pub use working::{FreezeMap, FrozenGraph, StableEdgeKey, StableNodeKey, WorkingGraph};
