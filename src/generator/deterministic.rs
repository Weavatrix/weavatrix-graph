use crate::{EdgeEndpoints, NodeIndex, Result, Topology};

/// Generates `0 -> 1 -> ... -> n-1`.
///
/// # Errors
///
/// Returns an error if the requested topology exceeds compact capacity.
pub fn path_topology(node_count: usize) -> Result<Topology> {
    Topology::try_from_edges(
        node_count,
        (1..node_count).map(|target| EdgeEndpoints::new(compact(target - 1), compact(target))),
    )
}

/// Generates one directed cycle, including a self-loop for one node.
///
/// # Errors
///
/// Returns an error if the requested topology exceeds compact capacity.
pub fn cycle_topology(node_count: usize) -> Result<Topology> {
    let edges = (node_count > 0)
        .then(|| {
            (0..node_count).map(|source| {
                EdgeEndpoints::new(compact(source), compact((source + 1) % node_count))
            })
        })
        .into_iter()
        .flatten();
    Topology::try_from_edges(node_count, edges)
}

/// Generates all directed edges between distinct nodes.
///
/// # Errors
///
/// Returns an error if the requested topology exceeds compact capacity.
pub fn complete_topology(node_count: usize) -> Result<Topology> {
    let edge_count = node_count.checked_mul(node_count.saturating_sub(1)).ok_or(
        crate::GraphError::IndexCapacityExceeded {
            category: "generated edges",
            count: usize::MAX,
        },
    )?;
    u32::try_from(edge_count).map_err(|_| crate::GraphError::IndexCapacityExceeded {
        category: "generated edges",
        count: edge_count,
    })?;
    Topology::try_from_edges(
        node_count,
        (0..node_count).flat_map(|source| {
            (0..node_count)
                .filter(move |&target| target != source)
                .map(move |target| EdgeEndpoints::new(compact(source), compact(target)))
        }),
    )
}

/// Generates a directed star with node zero as its center.
///
/// # Errors
///
/// Returns an error if the requested topology exceeds compact capacity.
pub fn star_topology(node_count: usize) -> Result<Topology> {
    Topology::try_from_edges(
        node_count,
        (1..node_count).map(|target| EdgeEndpoints::new(compact(0), compact(target))),
    )
}

/// Generates a directed rectangular grid with rightward and downward edges.
///
/// # Errors
///
/// Returns an error if dimensions or compact capacity overflow.
pub fn grid_topology(rows: usize, columns: usize) -> Result<Topology> {
    let node_count = rows
        .checked_mul(columns)
        .ok_or(crate::GraphError::ArithmeticOverflow {
            operation: "grid dimensions",
        })?;
    let horizontal = rows.saturating_mul(columns.saturating_sub(1));
    let vertical = columns.saturating_mul(rows.saturating_sub(1));
    validate_edge_count(horizontal.saturating_add(vertical))?;
    let edges = (0..rows).flat_map(move |row| {
        (0..columns).flat_map(move |column| {
            let source = row * columns + column;
            let right = (column + 1 < columns)
                .then(|| EdgeEndpoints::new(compact(source), compact(source + 1)));
            let down = (row + 1 < rows)
                .then(|| EdgeEndpoints::new(compact(source), compact(source + columns)));
            right.into_iter().chain(down)
        })
    });
    Topology::try_from_edges(node_count, edges)
}

/// Generates all directed edges from the left partition to the right.
///
/// # Errors
///
/// Returns an error if dimensions or compact capacity overflow.
pub fn complete_bipartite_topology(left: usize, right: usize) -> Result<Topology> {
    let node_count = left
        .checked_add(right)
        .ok_or(crate::GraphError::ArithmeticOverflow {
            operation: "bipartite node count",
        })?;
    let edge_count = left
        .checked_mul(right)
        .ok_or(crate::GraphError::ArithmeticOverflow {
            operation: "bipartite edge count",
        })?;
    validate_edge_count(edge_count)?;
    Topology::try_from_edges(
        node_count,
        (0..left).flat_map(|source| {
            (left..node_count)
                .map(move |target| EdgeEndpoints::new(compact(source), compact(target)))
        }),
    )
}

fn validate_edge_count(edge_count: usize) -> Result<()> {
    u32::try_from(edge_count)
        .map(|_| ())
        .map_err(|_| crate::GraphError::IndexCapacityExceeded {
            category: "generated edges",
            count: edge_count,
        })
}

fn compact(index: usize) -> NodeIndex {
    NodeIndex::new(u32::try_from(index).unwrap_or(u32::MAX))
}
