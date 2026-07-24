mod core;
mod result;
mod search;

pub use core::min_cost_max_flow;
pub use result::MinCostFlow;

use crate::Vec;

#[derive(Clone, Copy)]
pub(super) struct Step<Edge> {
    pub(super) previous: usize,
    pub(super) edge: Edge,
    pub(super) forward: bool,
}

pub(super) type Predecessors<Edge> = Vec<Option<Step<Edge>>>;
