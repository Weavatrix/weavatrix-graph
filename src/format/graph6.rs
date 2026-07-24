use crate::{
    BitMatrix, EdgeEndpoints, GraphError, IndexUndirectedGraphView, NodeIndex, Result,
    UndirectedTopology,
};
use crate::{String, Vec};

/// Encodes a compact simple undirected graph in Graph6 form.
///
/// # Errors
///
/// Returns an error for non-compact indexes, loops, parallel edges, or capacity
/// overflow because Graph6 cannot represent those contracts.
pub fn graph6_encode<G>(graph: &G) -> Result<String>
where
    G: IndexUndirectedGraphView,
{
    if graph.node_bound() != graph.node_count() {
        return Err(unsupported("non-compact node indexes"));
    }
    let mut seen = BitMatrix::try_new(graph.node_count())?;
    for node in graph.node_indices() {
        if G::node_slot(node) >= graph.node_count() {
            return Err(unsupported("non-compact node indexes"));
        }
    }
    for (_, endpoints) in graph
        .edge_indices()
        .filter_map(|edge| graph.edge_endpoints(edge).map(|ends| (edge, ends)))
    {
        let mut left = G::node_slot(endpoints.source());
        let mut right = G::node_slot(endpoints.target());
        if left == right {
            return Err(unsupported("self-loops"));
        }
        if left > right {
            core::mem::swap(&mut left, &mut right);
        }
        if !seen.insert(node(left), node(right))? {
            return Err(unsupported("parallel edges"));
        }
    }
    let mut encoded = encode_node_count(graph.node_count())?;
    let mut value = 0_u8;
    let mut width = 0_u8;
    for right in 1..graph.node_count() {
        for left in 0..right {
            value = (value << 1) | u8::from(seen.contains(node(left), node(right)));
            width += 1;
            if width == 6 {
                encoded.push(char::from(value + 63));
                value = 0;
                width = 0;
            }
        }
    }
    if width != 0 {
        encoded.push(char::from((value << (6 - width)) + 63));
    }
    Ok(encoded)
}

/// Decodes one Graph6 record into a compact undirected topology.
///
/// # Errors
///
/// Returns an error for malformed headers, lengths, padding, or capacity.
pub fn graph6_decode(input: &str) -> Result<UndirectedTopology> {
    let record = input.trim();
    let record = record.strip_prefix(">>graph6<<").unwrap_or(record);
    let bytes = record.as_bytes();
    let (node_count, offset) = decode_node_count(bytes)?;
    let bits = node_count
        .checked_mul(node_count.saturating_sub(1))
        .ok_or_else(|| invalid("node count overflows adjacency bits"))?
        / 2;
    let encoded_len = bits
        .checked_add(5)
        .ok_or_else(|| invalid("adjacency length overflow"))?
        / 6;
    if bytes.len() != offset + encoded_len {
        return Err(invalid("adjacency payload has the wrong length"));
    }
    let values = bytes[offset..]
        .iter()
        .map(|byte| six(*byte))
        .collect::<Result<Vec<_>>>()?;
    if let Some(last) = values.last() {
        let used = bits % 6;
        if used != 0 && last & ((1_u8 << (6 - used)) - 1) != 0 {
            return Err(invalid("nonzero Graph6 padding bits"));
        }
    }
    let mut edges = Vec::new();
    let mut position = 0_usize;
    for right in 1..node_count {
        for left in 0..right {
            let value = values[position / 6];
            let bit = 5 - position % 6;
            if value & (1_u8 << bit) != 0 {
                edges.push(EdgeEndpoints::new(node(left), node(right)));
            }
            position += 1;
        }
    }
    UndirectedTopology::try_from_edges(node_count, edges)
}

fn encode_node_count(node_count: usize) -> Result<String> {
    let count = u64::try_from(node_count).map_err(|_| GraphError::IndexCapacityExceeded {
        category: "Graph6 nodes",
        count: node_count,
    })?;
    let mut output = String::new();
    if count <= 62 {
        push_six(&mut output, count)?;
    } else if count <= 258_047 {
        output.push('~');
        for shift in [12, 6, 0] {
            push_six(&mut output, (count >> shift) & 0x3f)?;
        }
    } else if count <= 0xffff_fffff {
        output.push_str("~~");
        for shift in [30, 24, 18, 12, 6, 0] {
            push_six(&mut output, (count >> shift) & 0x3f)?;
        }
    } else {
        return Err(GraphError::IndexCapacityExceeded {
            category: "Graph6 nodes",
            count: node_count,
        });
    }
    Ok(output)
}

fn decode_node_count(bytes: &[u8]) -> Result<(usize, usize)> {
    let Some(&first) = bytes.first() else {
        return Err(invalid("empty record"));
    };
    if first != b'~' {
        return Ok((usize::from(six(first)?), 1));
    }
    if bytes.get(1) != Some(&b'~') {
        return Ok((decode_groups(bytes, 1, 3)?, 4));
    }
    Ok((decode_groups(bytes, 2, 6)?, 8))
}

fn decode_groups(bytes: &[u8], offset: usize, count: usize) -> Result<usize> {
    let slice = bytes
        .get(offset..offset + count)
        .ok_or_else(|| invalid("truncated node-count header"))?;
    slice.iter().try_fold(0_usize, |value, byte| {
        Ok((value << 6) | usize::from(six(*byte)?))
    })
}

fn push_six(output: &mut String, value: u64) -> Result<()> {
    let value = u8::try_from(value).map_err(|_| invalid("invalid six-bit value"))?;
    output.push(char::from(value + 63));
    Ok(())
}

fn six(byte: u8) -> Result<u8> {
    byte.checked_sub(63)
        .filter(|value| *value < 64)
        .ok_or_else(|| invalid("character is outside the Graph6 alphabet"))
}

fn node(index: usize) -> NodeIndex {
    NodeIndex::new(u32::try_from(index).unwrap_or(u32::MAX))
}

fn invalid(reason: &str) -> GraphError {
    GraphError::InvalidFormat {
        format: "Graph6",
        reason: String::from(reason),
    }
}

fn unsupported(feature: &'static str) -> GraphError {
    GraphError::UnsupportedGraphFeature {
        format: "Graph6",
        feature,
    }
}
