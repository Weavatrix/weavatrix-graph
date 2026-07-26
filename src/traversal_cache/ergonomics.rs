use super::{TraversalCache, TraversalStorage};
use crate::{Graph, Topology};

impl Topology {
    /// Derives a speed-oriented traversal cache and compresses it when useful.
    #[must_use]
    pub fn traversal_cache(&self) -> TraversalCache {
        TraversalCache::from_topology(self)
    }

    /// Derives a traversal cache with an explicit speed/space policy.
    #[must_use]
    pub fn traversal_cache_with(&self, storage: TraversalStorage) -> TraversalCache {
        TraversalCache::with_storage(self, storage)
    }
}

impl Graph {
    /// Derives an optional traversal cache without changing evidence storage.
    #[must_use]
    pub fn traversal_cache(&self) -> TraversalCache {
        self.topology().traversal_cache()
    }

    /// Derives a traversal cache with an explicit speed/space policy.
    #[must_use]
    pub fn traversal_cache_with(&self, storage: TraversalStorage) -> TraversalCache {
        self.topology().traversal_cache_with(storage)
    }
}

impl TraversalCache {
    /// Encoded bytes saved against a dual direct-`u32` traversal CSR.
    #[must_use]
    pub fn storage_savings_bytes(&self) -> usize {
        self.fast_equivalent_bytes()
            .saturating_sub(self.storage_bytes())
    }

    /// Encoded cache size divided by the equivalent direct cache size.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn storage_ratio(&self) -> f64 {
        let direct = self.fast_equivalent_bytes();
        if direct == 0 {
            1.0
        } else {
            self.storage_bytes() as f64 / direct as f64
        }
    }
}

impl From<&Topology> for TraversalCache {
    fn from(topology: &Topology) -> Self {
        Self::from_topology(topology)
    }
}

impl From<&Graph> for TraversalCache {
    fn from(graph: &Graph) -> Self {
        graph.traversal_cache()
    }
}
