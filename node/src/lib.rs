#![deny(unsafe_op_in_unsafe_fn)]

use napi::{Error, Result, Status};
use napi_derive::napi;
use serde::Serialize;
use weavatrix_graph::{
    Graph, GraphNodeIndex, bfs, has_cycle, page_rank, shortest_path, strongly_connected_components,
    topological_sort, topology_to_dot,
};

#[derive(Serialize)]
struct RankEntry<'a> {
    id: &'a str,
    score: f64,
}

#[napi]
pub struct NativeGraph {
    graph: Graph,
}

#[napi]
impl NativeGraph {
    #[napi(constructor)]
    pub fn new(graph_json: String) -> Result<Self> {
        let graph = serde_json::from_str(&graph_json).map_err(json_error)?;
        Ok(Self { graph })
    }

    #[napi(getter)]
    pub fn node_count(&self) -> u32 {
        u32::try_from(self.graph.node_count()).unwrap_or(u32::MAX)
    }

    #[napi(getter)]
    pub fn edge_count(&self) -> u32 {
        u32::try_from(self.graph.edge_count()).unwrap_or(u32::MAX)
    }

    #[napi]
    pub fn canonical_json(&self) -> Result<String> {
        serde_json::to_string(&self.graph).map_err(json_error)
    }

    #[napi]
    pub fn node_json(&self, id: String) -> Result<Option<String>> {
        self.graph
            .node(&id)
            .map(serde_json::to_string)
            .transpose()
            .map_err(json_error)
    }

    #[napi]
    pub fn outgoing_json(&self, id: String) -> Result<String> {
        let id = id.parse().map_err(graph_error)?;
        serde_json::to_string(&self.graph.outgoing(&id).collect::<Vec<_>>()).map_err(json_error)
    }

    #[napi]
    pub fn incoming_json(&self, id: String) -> Result<String> {
        let id = id.parse().map_err(graph_error)?;
        serde_json::to_string(&self.graph.incoming(&id).collect::<Vec<_>>()).map_err(json_error)
    }

    #[napi]
    pub fn bfs(&self, start: String) -> Result<Vec<String>> {
        let start = self.require_index(&start)?;
        Ok(indices_to_ids(&self.graph, bfs(&self.graph, start)))
    }

    #[napi]
    pub fn shortest_path(&self, source: String, target: String) -> Result<Option<Vec<String>>> {
        let source = self.require_index(&source)?;
        let target = self.require_index(&target)?;
        Ok(
            shortest_path(&self.graph, source, target)
                .map(|path| indices_to_ids(&self.graph, path)),
        )
    }

    #[napi]
    pub fn strongly_connected_components_json(&self) -> Result<String> {
        let components = strongly_connected_components(&self.graph)
            .into_iter()
            .map(|component| indices_to_ids(&self.graph, component))
            .collect::<Vec<_>>();
        serde_json::to_string(&components).map_err(json_error)
    }

    #[napi]
    pub fn topological_sort(&self) -> Option<Vec<String>> {
        topological_sort(&self.graph).map(|nodes| indices_to_ids(&self.graph, nodes))
    }

    #[napi]
    pub fn has_cycle(&self) -> bool {
        has_cycle(&self.graph)
    }

    #[napi]
    pub fn page_rank_json(&self, damping: Option<f64>, iterations: Option<u32>) -> Result<String> {
        let ranks = page_rank(
            &self.graph,
            damping.unwrap_or(0.85),
            usize::try_from(iterations.unwrap_or(20)).map_err(graph_error)?,
        )
        .map_err(graph_error)?;
        let entries = ranks
            .iter()
            .filter_map(|(index, score)| {
                self.graph.node_at(*index).map(|node| RankEntry {
                    id: node.id.as_str(),
                    score: *score,
                })
            })
            .collect::<Vec<_>>();
        serde_json::to_string(&entries).map_err(json_error)
    }

    #[napi]
    pub fn to_dot(&self) -> String {
        topology_to_dot(&self.graph)
    }

    fn require_index(&self, id: &str) -> Result<GraphNodeIndex> {
        self.graph
            .node_index(id)
            .ok_or_else(|| Error::new(Status::InvalidArg, format!("unknown graph node: {id}")))
    }
}

fn indices_to_ids(graph: &Graph, indices: Vec<GraphNodeIndex>) -> Vec<String> {
    indices
        .into_iter()
        .filter_map(|index| graph.node_at(index))
        .map(|node| node.id.to_string())
        .collect()
}

fn json_error(error: serde_json::Error) -> Error {
    Error::new(Status::InvalidArg, error.to_string())
}

fn graph_error(error: impl core::fmt::Display) -> Error {
    Error::new(Status::InvalidArg, error.to_string())
}
