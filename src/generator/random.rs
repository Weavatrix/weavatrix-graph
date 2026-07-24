use crate::Vec;
use crate::{EdgeEndpoints, GraphError, NodeIndex, Result, Topology, UndirectedTopology};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RandomGraphGenerator {
    state: u64,
}

impl RandomGraphGenerator {
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Generates a directed Erdos-Renyi graph without self-loops.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid probability or compact capacity overflow.
    pub fn directed(
        &mut self,
        node_count: usize,
        numerator: u64,
        denominator: u64,
    ) -> Result<Topology> {
        validate_probability(numerator, denominator)?;
        validate_node_count(node_count)?;
        let mut edges = Vec::new();
        for source in 0..node_count {
            for target in 0..node_count {
                if source != target && self.sample(numerator, denominator) {
                    edges.push(endpoints(source, target)?);
                }
            }
        }
        Topology::try_from_edges(node_count, edges)
    }

    /// Generates an undirected Erdos-Renyi graph without self-loops.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid probability or compact capacity overflow.
    pub fn undirected(
        &mut self,
        node_count: usize,
        numerator: u64,
        denominator: u64,
    ) -> Result<UndirectedTopology> {
        validate_probability(numerator, denominator)?;
        validate_node_count(node_count)?;
        let mut edges = Vec::new();
        for source in 0..node_count {
            for target in (source + 1)..node_count {
                if self.sample(numerator, denominator) {
                    edges.push(endpoints(source, target)?);
                }
            }
        }
        UndirectedTopology::try_from_edges(node_count, edges)
    }

    /// Generates a DAG with edges from lower to higher node ids.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid probability or compact capacity overflow.
    pub fn dag(&mut self, node_count: usize, numerator: u64, denominator: u64) -> Result<Topology> {
        validate_probability(numerator, denominator)?;
        validate_node_count(node_count)?;
        let mut edges = Vec::new();
        for source in 0..node_count {
            for target in (source + 1)..node_count {
                if self.sample(numerator, denominator) {
                    edges.push(endpoints(source, target)?);
                }
            }
        }
        Topology::try_from_edges(node_count, edges)
    }

    fn sample(&mut self, numerator: u64, denominator: u64) -> bool {
        if numerator == denominator {
            return true;
        }
        if numerator == 0 {
            return false;
        }
        self.next_u64() % denominator < numerator
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }
}

fn validate_probability(numerator: u64, denominator: u64) -> Result<()> {
    if denominator == 0 || numerator > denominator {
        return Err(GraphError::InvalidProbability {
            numerator,
            denominator,
        });
    }
    Ok(())
}

fn validate_node_count(node_count: usize) -> Result<()> {
    u32::try_from(node_count)
        .map(|_| ())
        .map_err(|_| GraphError::IndexCapacityExceeded {
            category: "generated nodes",
            count: node_count,
        })
}

fn endpoints(source: usize, target: usize) -> Result<EdgeEndpoints> {
    let source = compact(source)?;
    let target = compact(target)?;
    Ok(EdgeEndpoints::new(source, target))
}

fn compact(index: usize) -> Result<NodeIndex> {
    u32::try_from(index)
        .map(NodeIndex::new)
        .map_err(|_| GraphError::IndexCapacityExceeded {
            category: "generated node index",
            count: index,
        })
}
