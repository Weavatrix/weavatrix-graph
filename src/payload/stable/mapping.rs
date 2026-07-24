use super::StablePayloadGraph;
use super::core::{EdgeSlot, NodeSlot};

impl<NodePayload, EdgePayload> StablePayloadGraph<NodePayload, EdgePayload> {
    #[must_use]
    pub fn map_payloads<MappedNode, MappedEdge, NodeMap, EdgeMap>(
        self,
        mut map_node: NodeMap,
        mut map_edge: EdgeMap,
    ) -> StablePayloadGraph<MappedNode, MappedEdge>
    where
        NodeMap: FnMut(NodePayload) -> MappedNode,
        EdgeMap: FnMut(EdgePayload) -> MappedEdge,
    {
        StablePayloadGraph {
            nodes: self
                .nodes
                .into_iter()
                .map(|slot| NodeSlot {
                    generation: slot.generation,
                    value: slot.value.map(&mut map_node),
                    outgoing: slot.outgoing,
                    incoming: slot.incoming,
                })
                .collect(),
            edges: self
                .edges
                .into_iter()
                .map(|slot| EdgeSlot {
                    generation: slot.generation,
                    value: slot.value.map(&mut map_edge),
                    source: slot.source,
                    target: slot.target,
                    next_outgoing: slot.next_outgoing,
                    next_incoming: slot.next_incoming,
                })
                .collect(),
            free_nodes: self.free_nodes,
            free_edges: self.free_edges,
            node_count: self.node_count,
            edge_count: self.edge_count,
        }
    }
}
