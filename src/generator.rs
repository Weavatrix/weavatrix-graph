mod deterministic;
mod random;

pub use deterministic::{
    complete_bipartite_topology, complete_topology, cycle_topology, grid_topology, path_topology,
    star_topology,
};
pub use random::RandomGraphGenerator;
