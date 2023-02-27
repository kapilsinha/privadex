/*
 * Copyright (C) 2023-present Kapil Sinha
 * Company: PrivaDEX
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the Server Side Public License, version 1,
 * as published by MongoDB, Inc.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * Server Side Public License for more details.
 *
 * You should have received a copy of the Server Side Public License
 * along with this program. If not, see
 * <http://www.mongodb.com/licensing/server-side-public-license>.
 */

use core::fmt::{self, Debug};
use core::hash::Hash;
use duplicate::duplicate_item;
use graphlib::{Graph as SimpleGraph, VertexId};
// This is Rust's new std HashMap implementation,
// but this crate allows for no_std and is used in graphlib
use hashbrown::HashMap;
use ink_prelude::{vec, vec::Vec};
use scale::Encode;

use privadex_chain_metadata::common::{Amount, EthAddress, UniversalTokenId};
use privadex_common::fixed_point::DecimalFixedPoint;

use crate::{PublicError, Result};

use super::edge::Edge;
use super::traits::QuoteGetter;

// Note: Really this data is best represented with a multigraph because there can be
// several edges between two tokens (e.g. via multiple DEX liquidity pools). But
// 1. I can't find any existing no_std-supported Rust library that supports multigraphs and
// 2. Our SOR algorithms can be adjusted easily to work with a normal graph representation
// Thus we will represent with a normal graph and maintain a Vec of Edges for each vertex pair
pub struct Graph {
    pub simple_graph: SimpleGraph<Token>,
    pub vertices: HashMap<UniversalTokenId, VertexId>,
    edges: HashMap<VertexPair, Vec<Edge>>,
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            simple_graph: SimpleGraph::new(),
            vertices: HashMap::new(),
            edges: HashMap::new(),
        }
    }

    #[cfg(not(feature = "dot"))]
    pub fn add_vertex(&mut self, token: Token) -> VertexId {
        self.add_vertex_helper(token)
    }

    #[cfg(feature = "dot")]
    pub fn add_vertex(&mut self, token: Token) -> VertexId {
        let label = token.to_string();
        let vertex_id = self.add_vertex_helper(token);
        let _ = self.simple_graph.add_vertex_label(&vertex_id, &label);
        vertex_id
    }

    fn add_vertex_helper(&mut self, token: Token) -> VertexId {
        let token_id = token.id.clone();
        let vertex_id = self.simple_graph.add_vertex(token);
        self.vertices.insert(token_id, vertex_id);
        vertex_id
    }

    pub fn get_token(&self, token_id: &UniversalTokenId) -> Option<&Token> {
        let vertex_id = self.get_vertex(token_id)?;
        self.simple_graph.fetch(vertex_id)
    }

    pub fn get_vertex(&self, token_id: &UniversalTokenId) -> Option<&VertexId> {
        self.vertices.get(token_id)
    }

    pub fn get_edges(
        &self,
        src_vertex_id: VertexId,
        dest_vertex_id: VertexId,
    ) -> Option<&Vec<Edge>> {
        let vpair = VertexPair {
            src: src_vertex_id,
            dest: dest_vertex_id,
        };
        self.edges.get(&vpair)
    }

    #[cfg(not(feature = "dot"))]
    pub fn add_edge(&mut self, edge: Edge) -> Result<()> {
        let (src_id, dest_id) = edge.get_src_dest_token();
        // We expect that the edge's endpoints already exist in the graph
        let src = self
            .vertices
            .get(src_id)
            .ok_or(PublicError::VertexNotInGraph(src_id.clone()))?;
        let dest = self
            .vertices
            .get(dest_id)
            .ok_or(PublicError::VertexNotInGraph(dest_id.clone()))?;
        if let Some(edges) = self.edges.get_mut(&VertexPair {
            src: *src,
            dest: *dest,
        }) {
            edges.push(edge);
        } else {
            let _ = self
                .simple_graph
                .add_edge(src, dest)
                .map_err(|_| PublicError::AddEdgeFailed)?;
            self.edges.insert(
                VertexPair {
                    src: src.clone(),
                    dest: dest.clone(),
                },
                vec![edge],
            );
        }
        Ok(())
    }

    #[cfg(feature = "dot")]
    pub fn add_edge(&mut self, edge: Edge) -> Result<()> {
        let (src_id, dest_id) = edge.get_src_dest_token();
        // We expect that the edge's endpoints already exist in the graph
        let src = self
            .vertices
            .get(src_id)
            .ok_or(PublicError::VertexNotInGraph(src_id.clone()))?;
        let dest = self
            .vertices
            .get(dest_id)
            .ok_or(PublicError::VertexNotInGraph(dest_id.clone()))?;
        if let Some(edges) = self.edges.get_mut(&VertexPair {
            src: *src,
            dest: *dest,
        }) {
            edges.push(edge);
        } else {
            let _ = self
                .simple_graph
                .add_edge(src, dest)
                .map_err(|_| PublicError::AddEdgeFailed)?;
            let label = edge.to_string();
            self.edges.insert(
                VertexPair {
                    src: src.clone(),
                    dest: dest.clone(),
                },
                vec![edge],
            );
            let _ = self.simple_graph.add_edge_label(src, dest, &label);
        }
        Ok(())
    }

    // Note this is an expensive operation, just for test purposes. If this functionality is needed
    // in prod, we should just store a variable for the count and increment it in add_edge
    pub fn edge_count(&self) -> usize {
        self.edges
            .values()
            .fold(0, |a, multiedge_vec| a + multiedge_vec.len())
    }
}

// Node in the graph
#[derive(Debug, Clone)]
pub struct Token {
    pub id: UniversalTokenId,
    // # of native token unit per this token unit
    // For example, if 1 USDC (6 decimals) = 3 GLMR (18 decimals), then
    // USDC.derived_eth = (3 * 10^18) / 10^6
    pub derived_eth: DecimalFixedPoint,
    // # of USD per this token unit
    // For example, if 1 USDC (6 decimals) = 1 USD,  then
    // USDC.derived_usd = 1 / 10^6
    pub derived_usd: DecimalFixedPoint,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}(eth={:?}, usd={:?})",
            self.id, self.derived_eth, self.derived_usd
        )
    }
}

// This is otherwise hidden in the graphlib internals
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct VertexPair {
    pub src: VertexId,
    pub dest: VertexId,
}

#[derive(Debug, Clone, Encode)]
pub struct GraphSolution {
    pub paths: Vec<SplitGraphPath>,
    pub amount_in: Amount,
    pub src_addr: EthAddress, // wallet src, we only support Eth addresses for now
    pub dest_addr: EthAddress, // wallet dest, we only support Eth addresses for now
}

impl fmt::Display for GraphSolution {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let _ = write!(
            f,
            "GraphSolution: amount_in = {:?}, src_addr = {:?}, dest_addr = {:?}\n ",
            self.amount_in, self.src_addr, self.dest_addr
        );
        for (i, p) in self.paths.iter().enumerate() {
            let _ = write!(f, "SplitGraphPath {}: {}", i + 1, p.path);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Encode)]
pub struct SplitGraphPath {
    pub path: GraphPath,
    pub fraction_amount_in: Amount,
    pub fraction_bps: u16, // e.g. 500 means that 5% of the amount_in goes down this path
}

// Note that these function signatures are different from those in QuoteGetter since amount_in
// is not passed in (the GraphSolution has a fixed amount_in)
impl GraphSolution {
    pub fn get_src_dest_token(&self) -> (&UniversalTokenId, &UniversalTokenId) {
        // paths must be non-empty and all the paths must have the same src and dest,
        // so we arbitrarily use the first path's src and dest
        self.paths
            .first()
            .expect("GraphSolution paths must be non-empty")
            .path
            .get_src_dest_token()
    }

    pub fn get_quote(&self) -> Amount {
        // Note that there will be some rounding error / truncation since we don't add back the remainder
        // after multiplying by the fraction
        self.paths.iter().fold(0, |amount_out, split_path| {
            amount_out + split_path.path.get_quote(split_path.fraction_amount_in)
        })
    }

    pub fn get_quote_with_estimated_txn_fees(&self) -> Amount {
        self.paths.iter().fold(0, |amount_out, split_path| {
            amount_out
                + split_path
                    .path
                    .get_quote_with_estimated_txn_fees(split_path.fraction_amount_in)
        })
    }

    pub fn get_estimated_txn_fees_in_dest_token(&self) -> Amount {
        self.paths.iter().fold(0, |fees, split_path| {
            fees + split_path.path.get_estimated_txn_fees_in_dest_token()
        })
    }

    pub fn get_estimated_txn_fees_usd(&self) -> Amount {
        self.paths.iter().fold(0, |fees, split_path| {
            fees + split_path.path.get_estimated_txn_fees_usd()
        })
    }

    pub fn get_dest_chain_estimated_gas_fee_usd(&self) -> Amount {
        self.paths.iter().fold(0, |fees, split_path| {
            fees + split_path.path.get_dest_chain_estimated_gas_fee_usd()
        })
    }
}

#[derive(Debug)]
pub struct GraphPathRef<'a>(pub Vec<&'a Edge>);

#[derive(Debug, Clone, Encode)]
pub struct GraphPath(pub Vec<Edge>);

// GraphPath and GraphPathRef have the same trait impl, so we use the duplicate macro
// to avoid repeating it
#[duplicate_item(
	lifetime	struct_name;
	['a]	[GraphPathRef<'a>];
	[]		[GraphPath];
)]
impl<lifetime> QuoteGetter for struct_name {
    fn get_src_dest_token(&self) -> (&UniversalTokenId, &UniversalTokenId) {
        if self.0.len() == 0 {
            panic!("GraphPath must have at least one edge");
        }
        let (src, _) = self.0.first().unwrap().get_src_dest_token();
        let (_, dest) = self.0.last().unwrap().get_src_dest_token();
        (src, dest)
    }

    fn get_quote(&self, amount_in: Amount) -> Amount {
        let mut amount_out = amount_in;
        for edge in self.0.iter() {
            amount_out = edge.get_quote(amount_out)
        }
        amount_out
    }

    fn get_quote_with_estimated_txn_fees(&self, amount_in: Amount) -> Amount {
        let mut amount_out = amount_in;
        for edge in self.0.iter() {
            amount_out = edge.get_quote_with_estimated_txn_fees(amount_out)
        }
        amount_out
    }

    fn get_estimated_txn_fees_in_dest_token(&self) -> Amount {
        // We cannot just add txn fees from each edge because they are all in
        // terms of their respective dest_tokens. We thus use the get_quote
        // function - but note that this means that CPMM edge fees are
        // underestimated (0.997 - .9975x). Close enough though so we ignore this
        let mut fee = 0;
        for edge in self.0.iter() {
            let additional_fee = edge.get_estimated_txn_fees_in_dest_token();
            fee = edge.get_quote(fee) + additional_fee;
        }
        fee
    }

    fn get_estimated_txn_fees_usd(&self) -> Amount {
        self.0.iter().fold(0, |fees_usd, edge| {
            fees_usd + edge.get_estimated_txn_fees_usd()
        })
    }

    fn get_dest_chain_estimated_gas_fee_usd(&self) -> Amount {
        self.0.iter().fold(0, |fees_usd, edge| {
            fees_usd + edge.get_dest_chain_estimated_gas_fee_usd()
        })
    }
}

impl From<GraphPathRef<'_>> for GraphPath {
    fn from(graph_path_ref: GraphPathRef) -> Self {
        Self {
            0: graph_path_ref
                .0
                .into_iter()
                .map(|edge_ref| edge_ref.clone())
                .collect(),
        }
    }
}

#[duplicate_item(
	lifetime	struct_name;
	['a]	[GraphPathRef<'a>];
	[]		[GraphPath];
)]
impl<lifetime> fmt::Display for struct_name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GraphPath ({} edges):", self.0.len())?;
        for edge in self.0.iter() {
            write!(f, "\n  {:?}", edge)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod graph_tests {
    use super::super::edge::{BridgeEdge, XCMBridgeEdge};
    use super::*;
    use ink_env::debug_println;
    use privadex_chain_metadata::registry::bridge::xcm_bridge_registry;

    fn create_token(id: UniversalTokenId) -> Token {
        Token {
            id,
            derived_eth: DecimalFixedPoint::from_str_and_exp("10", 3),
            derived_usd: DecimalFixedPoint::from_str_and_exp("10", 3),
        }
    }

    #[test]
    fn test_create_bridge_graph() {
        let mut graph = Graph::new();
        for xcm_bridge in xcm_bridge_registry::XCM_BRIDGES.iter() {
            if graph.get_token(&xcm_bridge.src_token).is_none() {
                let token = create_token(xcm_bridge.src_token.clone());
                graph.add_vertex(token);
            }
            let (src_token_derived_eth, derived_usd) = {
                if let Some(token) = graph.get_token(&xcm_bridge.src_token) {
                    (token.derived_eth.clone(), token.derived_usd.clone())
                } else {
                    let token = create_token(xcm_bridge.dest_token.clone());
                    let (derived_eth, derived_usd) =
                        (token.derived_eth.clone(), token.derived_usd.clone());
                    graph.add_vertex(token);
                    (derived_eth, derived_usd)
                }
            };
            let dest_token_derived_eth = {
                if let Some(token) = graph.get_token(&xcm_bridge.dest_token) {
                    token.derived_eth.clone()
                } else {
                    let token = create_token(xcm_bridge.dest_token.clone());
                    let derived_eth = token.derived_eth.clone();
                    graph.add_vertex(token);
                    derived_eth
                }
            };
            let edge = Edge::Bridge(BridgeEdge::Xcm(
                XCMBridgeEdge::from_bridge_and_derived_quantities(
                    xcm_bridge.clone(),
                    &src_token_derived_eth,
                    &dest_token_derived_eth,
                    &derived_usd,
                ),
            ));
            let _ = graph.add_edge(edge).unwrap();
        }
        debug_println!("Vertex count: {}", graph.simple_graph.vertex_count());
        debug_println!("Edge count: {}", graph.simple_graph.edge_count());
        assert_eq!(true, true);
    }
}
