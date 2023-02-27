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

use ink_prelude::{vec, vec::Vec};

use privadex_chain_metadata::common::{Amount, EthAddress, UniversalTokenId};

use super::helper_graph_algos::{find_all_paths, AllPathsFinderConfig};
use crate::graph::graph::{Graph, GraphPath, GraphPathRef, GraphSolution, SplitGraphPath};
use crate::graph::traits::QuoteGetter;
use crate::{PublicError, Result};

pub struct SORConfig {
    all_paths_finder_config: AllPathsFinderConfig,
}

impl Default for SORConfig {
    fn default() -> Self {
        SORConfig {
            all_paths_finder_config: AllPathsFinderConfig::default(),
        }
    }
}

pub struct SinglePathSOR<'a> {
    graph: &'a Graph,
    src_addr: EthAddress,
    dest_addr: EthAddress,
    src_token: UniversalTokenId,
    dest_token: UniversalTokenId,
    sor_config: SORConfig,
}

impl<'a> SinglePathSOR<'a> {
    pub fn new(
        graph: &'a Graph,
        src_addr: EthAddress,
        dest_addr: EthAddress,
        src_token: UniversalTokenId,
        dest_token: UniversalTokenId,
        sor_config: SORConfig,
    ) -> Self {
        Self {
            graph,
            src_addr,
            dest_addr,
            src_token,
            dest_token,
            sor_config,
        }
    }

    pub fn compute_graph_solution(&self, amount_in: Amount) -> Result<GraphSolution> {
        let single_optimal_path = self.find_optimal_path(amount_in)?;
        let split_path = SplitGraphPath {
            path: single_optimal_path,
            fraction_amount_in: amount_in,
            fraction_bps: 10_000,
        };
        Ok(GraphSolution {
            paths: vec![split_path],
            amount_in,
            src_addr: self.src_addr,
            dest_addr: self.dest_addr,
        })
    }

    fn find_optimal_path(&self, amount_in: Amount) -> Result<GraphPath> {
        if self.src_token == self.dest_token {
            return Err(PublicError::SrcTokenDestTokenAreSame);
        }
        let src_vertex = self
            .graph
            .get_vertex(&self.src_token)
            .ok_or(PublicError::VertexNotInGraph(self.src_token.clone()))?;
        let dest_vertex = self
            .graph
            .get_vertex(&self.dest_token)
            .ok_or(PublicError::VertexNotInGraph(self.dest_token.clone()))?;

        let paths: Vec<GraphPathRef> = find_all_paths(
            &self.graph,
            src_vertex,
            dest_vertex,
            &self.sor_config.all_paths_finder_config,
        );
        let optimal_path = paths
            .into_iter()
            .max_by_key(|path| path.get_quote_with_estimated_txn_fees(amount_in))
            .ok_or(PublicError::NoPathFound)?;

        Ok(GraphPath::from(optimal_path))
    }
}

#[cfg(test)]
mod single_path_sor_tests {
    use hex_literal::hex;
    use ink_env::debug_println;

    use privadex_chain_metadata::common::{
        ChainTokenId::ERC20, ERC20Token, EthAddress, UniversalChainId::SubstrateParachain,
        UniversalTokenId,
    };
    use privadex_chain_metadata::registry::{
        chain::RelayChain::Polkadot, token::universal_token_id_registry,
    };
    use privadex_common::fixed_point::DecimalFixedPoint;

    use super::*;
    use crate::test_utilities::graph_factory;

    const DUMMY_ADDR: EthAddress = EthAddress::zero();

    fn test_graph_solution_helper(
        graph: &Graph,
        src_token_id: UniversalTokenId,
        dest_token_id: UniversalTokenId,
        amount_in: Amount,
    ) -> GraphSolution {
        let sor_config = SORConfig::default();
        let sor = SinglePathSOR::new(
            graph,
            DUMMY_ADDR,
            DUMMY_ADDR,
            src_token_id.clone(),
            dest_token_id.clone(),
            sor_config,
        );
        let graph_solution = sor
            .compute_graph_solution(amount_in)
            .expect("We expect a solution");

        {
            let src_token = graph.get_token(&src_token_id).unwrap();
            let dest_token = graph.get_token(&dest_token_id).unwrap();
            let quote = graph_solution.get_quote_with_estimated_txn_fees();
            let fee = graph_solution.get_estimated_txn_fees_in_dest_token();
            let input_usd = src_token.derived_usd.mul_u128(amount_in);
            let output_usd = dest_token.derived_usd.mul_u128(quote);
            let fee_usd = dest_token.derived_usd.mul_u128(fee);
            debug_println!(
                "Amount in = {} (${}), Amount out = {} (${}), Fee = {} (${})",
                amount_in,
                input_usd,
                quote,
                output_usd,
                fee,
                fee_usd
            );
        }

        debug_println!("Graph solution: {:?}", graph_solution);
        // debug_println!("Graph solution: {}", graph_solution.paths[0].path);
        graph_solution
    }

    #[test]
    fn test_sor_small_graph() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let graph = graph_factory::small_graph();
        debug_println!(
            "Small graph: # vertices = {}, # simple edges = {}, # multi edges = {}",
            graph.simple_graph.vertex_count(),
            graph.simple_graph.edge_count(),
            graph.edge_count()
        );

        let src_token_id = universal_token_id_registry::GLMR_NATIVE;
        let dest_token_id = universal_token_id_registry::DOT_NATIVE;
        let amount_in = 100_000_000_000_000_000_000;

        let graph_solution =
            test_graph_solution_helper(&graph, src_token_id, dest_token_id, amount_in);
        assert!(graph_solution.paths[0].path.0.len() > 0);
    }

    #[test]
    fn test_sor_medium_graph() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let graph = graph_factory::medium_graph();
        debug_println!(
            "Medium graph: # vertices = {}, # simple edges = {}, # multi edges = {}",
            graph.simple_graph.vertex_count(),
            graph.simple_graph.edge_count(),
            graph.edge_count()
        );

        let src_token_id = universal_token_id_registry::GLMR_NATIVE;
        let dest_token_id = universal_token_id_registry::DOT_NATIVE;
        let amount_in = 100_000_000_000_000_000_000;

        let graph_solution =
            test_graph_solution_helper(&graph, src_token_id, dest_token_id, amount_in);
        assert!(graph_solution.paths[0].path.0.len() > 0);
    }

    #[test]
    fn test_sor_full_graph() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let graph = graph_factory::full_graph();
        debug_println!(
            "Full graph: # vertices = {}, # simple edges = {}, # multi edges = {}",
            graph.simple_graph.vertex_count(),
            graph.simple_graph.edge_count(),
            graph.edge_count()
        );

        let src_token_id = UniversalTokenId {
            chain: SubstrateParachain(Polkadot, 2006),
            id: ERC20(ERC20Token {
                addr: EthAddress {
                    0: hex!("29F6e49c6E3397C3A84F715885F9F233A441165C"),
                },
            }),
        };
        let dest_token_id = UniversalTokenId {
            chain: SubstrateParachain(Polkadot, 2004),
            id: ERC20(ERC20Token {
                addr: EthAddress {
                    0: hex!("c9BAA8cfdDe8E328787E29b4B078abf2DaDc2055"),
                },
            }),
        };
        {
            let amount_in = 50_000_000_000_000_000_000;
            let graph_solution = test_graph_solution_helper(
                &graph,
                src_token_id.clone(),
                dest_token_id.clone(),
                amount_in,
            );
            debug_println!("\nQuote={}, Quote_w_fees={}, Txn_fees_dest_token={}, Txn_fees_usd={}, Dest_chain_gas_fee_usd={}, Solution={}",
                graph_solution.get_quote(),
                graph_solution.get_quote_with_estimated_txn_fees(),
                graph_solution.get_estimated_txn_fees_in_dest_token(),
                (graph_solution.get_estimated_txn_fees_usd() as f64) / (Amount::pow(10, 18) as f64),
                (graph_solution.get_dest_chain_estimated_gas_fee_usd() as f64) / (Amount::pow(10, 18) as f64),
                graph_solution,
            );
            assert_eq!(
                graph_solution.paths[0]
                    .path
                    .0
                    .iter()
                    .filter(|edge| edge.is_bridge())
                    .count(),
                1
            );
        }
        {
            // Bridging through Polkadot is chosen for larger amount_in to minimize price impact
            // since Astar's ASTR/GLMR liquidity pool is relatively small
            let amount_in = 1_000_000_000_000_000_000_000;
            let graph_solution = test_graph_solution_helper(
                &graph,
                src_token_id.clone(),
                dest_token_id.clone(),
                amount_in,
            );
            debug_println!("\nQuote={}, Quote_w_fees={}, Txn_fees_dest_token={}, Txn_fees_usd={}, Dest_chain_gas_fee_usd={}, Solution={}",
                graph_solution.get_quote(),
                graph_solution.get_quote_with_estimated_txn_fees(),
                graph_solution.get_estimated_txn_fees_in_dest_token(),
                (graph_solution.get_estimated_txn_fees_usd() as f64) / (Amount::pow(10, 18) as f64),
                (graph_solution.get_dest_chain_estimated_gas_fee_usd() as f64) / (Amount::pow(10, 18) as f64),
                graph_solution,
            );
            assert_eq!(
                graph_solution.paths[0]
                    .path
                    .0
                    .iter()
                    .filter(|edge| edge.is_bridge())
                    .count(),
                2
            );
        }
    }

    // This is a time-consuming test so we filter it out, but actually it loops over 3600 pairs in 11 seconds
    // - which is amazingly fast
    #[test]
    #[ignore]
    fn test_sor_full_graph_loop() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let graph = graph_factory::full_graph();
        debug_println!(
            "Full graph: # vertices = {}, # simple edges = {}, # multi edges = {}",
            graph.simple_graph.vertex_count(),
            graph.simple_graph.edge_count(),
            graph.edge_count()
        );

        // We sort tokens so this test runs in a deterministic order
        let tokens = {
            let mut v: Vec<&UniversalTokenId> = graph.vertices.keys().collect();
            v.sort();
            v
        };

        for (i, &src_token_id) in tokens.iter().enumerate() {
            for (j, &dest_token_id) in tokens.iter().enumerate() {
                if src_token_id == dest_token_id {
                    continue;
                }
                let src_derived_usd = &graph
                    .get_token(src_token_id)
                    .expect("Src token is in the graph")
                    .derived_usd;
                let dest_derived_usd = &graph
                    .get_token(dest_token_id)
                    .expect("Dest token is in the graph")
                    .derived_usd;
                // Start with $1000 of src_token
                let amount_in = DecimalFixedPoint::u128_div(1000, &src_derived_usd);

                let graph_solution = {
                    let sor_config = SORConfig::default();
                    let sor = SinglePathSOR::new(
                        &graph,
                        DUMMY_ADDR,
                        DUMMY_ADDR,
                        src_token_id.clone(),
                        dest_token_id.clone(),
                        sor_config,
                    );
                    sor.compute_graph_solution(amount_in)
                        .expect("We expect a solution")
                };
                let quote = graph_solution.get_quote_with_estimated_txn_fees();
                let path_length = graph_solution.paths[0].path.0.len();
                debug_println!(
                    "({}, {}): (In = ${:.2}, Out = ${:.2}); [{} steps]\n {:?}\n ->\n {:?}\n",
                    i,
                    j,
                    src_derived_usd.mul_u128(amount_in),
                    dest_derived_usd.mul_u128(quote),
                    path_length,
                    src_token_id,
                    dest_token_id,
                );
            }
        }
    }
}
