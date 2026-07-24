use crate::Vec;
use crate::{GraphError, NodeIndex, Result};
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DenseMatrix<T> {
    node_count: u32,
    edge_count: usize,
    cells: Vec<Option<T>>,
}

impl<T> DenseMatrix<T> {
    /// Creates a fixed-size directed adjacency matrix.
    ///
    /// # Errors
    ///
    /// Returns an error when the dimensions exceed index or address capacity.
    pub fn try_new(node_count: usize) -> Result<Self> {
        let compact = u32::try_from(node_count).map_err(|_| GraphError::IndexCapacityExceeded {
            category: "matrix nodes",
            count: node_count,
        })?;
        let cells = node_count
            .checked_mul(node_count)
            .ok_or(GraphError::ArithmeticOverflow {
                operation: "dense matrix dimensions",
            })?;
        Ok(Self {
            node_count: compact,
            edge_count: 0,
            cells: core::iter::repeat_with(|| None).take(cells).collect(),
        })
    }

    #[must_use]
    pub const fn node_count(&self) -> usize {
        self.node_count as usize
    }

    #[must_use]
    pub const fn edge_count(&self) -> usize {
        self.edge_count
    }

    #[must_use]
    pub fn get(&self, source: NodeIndex, target: NodeIndex) -> Option<&T> {
        self.slot(source, target)
            .ok()
            .and_then(|slot| self.cells[slot].as_ref())
    }

    #[must_use]
    pub fn get_mut(&mut self, source: NodeIndex, target: NodeIndex) -> Option<&mut T> {
        self.slot(source, target)
            .ok()
            .and_then(|slot| self.cells[slot].as_mut())
    }

    /// Inserts or replaces one directed edge value.
    ///
    /// # Errors
    ///
    /// Returns an error when either endpoint is outside the fixed matrix.
    pub fn insert(&mut self, source: NodeIndex, target: NodeIndex, value: T) -> Result<Option<T>> {
        let slot = self.slot(source, target)?;
        let previous = self.cells[slot].replace(value);
        self.edge_count += usize::from(previous.is_none());
        Ok(previous)
    }

    pub fn remove(&mut self, source: NodeIndex, target: NodeIndex) -> Option<T> {
        let slot = self.slot(source, target).ok()?;
        let previous = self.cells[slot].take();
        self.edge_count -= usize::from(previous.is_some());
        previous
    }

    pub fn outgoing(&self, source: NodeIndex) -> impl Iterator<Item = (NodeIndex, &T)> {
        let valid = source.index() < self.node_count();
        (0..self.node_count()).filter_map(move |target| {
            let target = NodeIndex::new(u32::try_from(target).ok()?);
            valid
                .then(|| self.get(source, target))
                .flatten()
                .map(|value| (target, value))
        })
    }

    pub fn incoming(&self, target: NodeIndex) -> impl Iterator<Item = (NodeIndex, &T)> {
        let valid = target.index() < self.node_count();
        (0..self.node_count()).filter_map(move |source| {
            let source = NodeIndex::new(u32::try_from(source).ok()?);
            valid
                .then(|| self.get(source, target))
                .flatten()
                .map(|value| (source, value))
        })
    }

    pub fn edges(&self) -> impl Iterator<Item = (NodeIndex, NodeIndex, &T)> {
        self.cells.iter().enumerate().filter_map(|(slot, value)| {
            let value = value.as_ref()?;
            let source = u32::try_from(slot / self.node_count()).ok()?;
            let target = u32::try_from(slot % self.node_count()).ok()?;
            Some((NodeIndex::new(source), NodeIndex::new(target), value))
        })
    }

    fn slot(&self, source: NodeIndex, target: NodeIndex) -> Result<usize> {
        let count = self.node_count();
        if source.index() >= count {
            return Err(GraphError::InvalidNodeIndex {
                node: source.index(),
                node_count: count,
            });
        }
        if target.index() >= count {
            return Err(GraphError::InvalidNodeIndex {
                node: target.index(),
                node_count: count,
            });
        }
        Ok(source.index() * count + target.index())
    }
}

impl<T: Clone> DenseMatrix<T> {
    /// Inserts the same value in both directions.
    ///
    /// # Errors
    ///
    /// Returns an error when either endpoint is outside the fixed matrix.
    pub fn insert_undirected(
        &mut self,
        left: NodeIndex,
        right: NodeIndex,
        value: T,
    ) -> Result<(Option<T>, Option<T>)> {
        let reverse = value.clone();
        let first = self.insert(left, right, value)?;
        let second = if left == right {
            first.clone()
        } else {
            self.insert(right, left, reverse)?
        };
        Ok((first, second))
    }
}

#[derive(Deserialize)]
struct DenseWire<T> {
    node_count: u32,
    #[serde(rename = "edge_count")]
    _edge_count: usize,
    cells: Vec<Option<T>>,
}

impl<'de, T> Deserialize<'de> for DenseMatrix<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = DenseWire::deserialize(deserializer)?;
        let node_count = wire.node_count as usize;
        let expected = node_count
            .checked_mul(node_count)
            .ok_or_else(|| D::Error::custom("dense matrix dimensions overflow"))?;
        if wire.cells.len() != expected {
            return Err(D::Error::custom(format!(
                "dense matrix has {} cells, expected {expected}",
                wire.cells.len()
            )));
        }
        let edge_count = wire.cells.iter().filter(|cell| cell.is_some()).count();
        Ok(Self {
            node_count: wire.node_count,
            edge_count,
            cells: wire.cells,
        })
    }
}
