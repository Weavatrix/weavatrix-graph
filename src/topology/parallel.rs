use super::csr::Csr;
use super::{EdgeEndpoints, Topology};
use crate::{Result, Vec};

impl Topology {
    /// Chooses the stable sequential or Rayon builder from the edge count.
    ///
    /// The current crossover is tuned conservatively for commodity multicore
    /// CPUs; output is identical to [`Self::try_from_edges`].
    ///
    /// # Errors
    ///
    /// Returns an error for excessive capacity or an invalid endpoint.
    pub fn try_from_edges_auto(
        node_count: usize,
        edges: impl IntoIterator<Item = EdgeEndpoints>,
    ) -> Result<Self> {
        const PARALLEL_EDGE_THRESHOLD: usize = 1_500_000;
        let endpoints = edges.into_iter().collect::<Vec<_>>();
        if endpoints.len() < PARALLEL_EDGE_THRESHOLD {
            return Self::try_from_collected(node_count, endpoints);
        }
        Self::try_from_collected_parallel(node_count, endpoints, true)
    }

    /// Builds a validated topology while constructing both CSR directions in
    /// parallel.
    ///
    /// Edge indexes and adjacency iteration retain input order.
    ///
    /// # Errors
    ///
    /// Returns an error for excessive capacity or an invalid endpoint.
    pub fn try_from_edges_parallel(
        node_count: usize,
        edges: impl IntoIterator<Item = EdgeEndpoints>,
    ) -> Result<Self> {
        Self::try_from_collected_parallel(node_count, edges.into_iter().collect(), true)
    }

    /// Builds both CSR directions concurrently without canonicalizing adjacency
    /// order.
    ///
    /// Edge indexes and endpoints retain input order, but adjacency iteration
    /// order is unspecified.
    ///
    /// # Errors
    ///
    /// Returns an error for excessive capacity or an invalid endpoint.
    pub fn try_from_edges_parallel_unordered(
        node_count: usize,
        edges: impl IntoIterator<Item = EdgeEndpoints>,
    ) -> Result<Self> {
        Self::try_from_collected_parallel(node_count, edges.into_iter().collect(), false)
    }

    /// Builds a stable-order topology with the direct-write `unsafe-fast`
    /// backend.
    ///
    /// The safe public operation contains an audited unsafe scatter loop.
    ///
    /// # Errors
    ///
    /// Returns an error for excessive capacity or an invalid endpoint.
    #[cfg(feature = "unsafe-fast")]
    pub fn try_from_edges_parallel_fast(
        node_count: usize,
        edges: impl IntoIterator<Item = EdgeEndpoints>,
    ) -> Result<Self> {
        Self::try_from_collected_parallel_fast(node_count, edges.into_iter().collect(), true)
    }

    /// Builds an unspecified-order topology with the direct-write `unsafe-fast`
    /// backend.
    ///
    /// Edge indexes and endpoints still retain input order.
    ///
    /// # Errors
    ///
    /// Returns an error for excessive capacity or an invalid endpoint.
    #[cfg(feature = "unsafe-fast")]
    pub fn try_from_edges_parallel_unordered_fast(
        node_count: usize,
        edges: impl IntoIterator<Item = EdgeEndpoints>,
    ) -> Result<Self> {
        Self::try_from_collected_parallel_fast(node_count, edges.into_iter().collect(), false)
    }

    fn try_from_collected_parallel(
        node_count: usize,
        endpoints: Vec<EdgeEndpoints>,
        stable_order: bool,
    ) -> Result<Self> {
        let compact = Self::validate_capacity(node_count, endpoints.len())?;
        let (outgoing, incoming) =
            Csr::try_build_pair_parallel(node_count, &endpoints, stable_order)?;
        Ok(Self::from_parts(compact, endpoints, outgoing, incoming))
    }

    #[cfg(feature = "unsafe-fast")]
    fn try_from_collected_parallel_fast(
        node_count: usize,
        endpoints: Vec<EdgeEndpoints>,
        stable_order: bool,
    ) -> Result<Self> {
        let compact = Self::validate_capacity(node_count, endpoints.len())?;
        let (outgoing, incoming) =
            Csr::try_build_pair_parallel_fast(node_count, &endpoints, stable_order)?;
        Ok(Self::from_parts(compact, endpoints, outgoing, incoming))
    }
}
