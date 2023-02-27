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

use hashbrown::HashSet;
use privadex_chain_metadata::{
    bridge::XCMBridge,
    chain_info::ChainInfo,
    common::{ChainTokenId, Dex, UniversalChainId, UniversalTokenId, USD_AMOUNT_EXPONENT},
    get_chain_info_from_chain_id, get_dexes_from_chain_id,
    registry::{bridge::xcm_bridge_registry, token::universal_token_id_registry},
};
use privadex_common::fixed_point::DecimalFixedPoint;

use crate::graph::{
    edge::{BridgeEdge, Edge, SwapEdge, UnwrapEdge, WrapEdge, XCMBridgeEdge},
    graph::{Graph, Token},
};
use crate::graphql_client::get_additional_tokens_and_edges;
use crate::{PublicError, Result};

// Set low enough so that we include the ASTR/GLMR pool in ArthSwap
// but high enough that the largest HTTP response is less than 16KB
// (eventually we need to implement pagination of results)
const MIN_TOKEN_PAIR_RESERVE_USD: u32 = 12_000;

// This function *can* return an error if MIN_TOKEN_PAIR_RESERVE_USD filters out too many edges!
// I choose to return error instead of skipping adding those edges because I don't want silent
// unexpected behavior
pub fn create_graph_from_chain_ids(chain_ids: &[UniversalChainId]) -> Result<Graph> {
    let mut graph = Graph::new();

    // Note that ORDER MATTERS in the adding of edges below.
    // Add SwapEdges first because derived_eth and derived_usd are entirely sourced from the DEXes,
    // so we need to create those tokens first.
    // [Order hereon doesn't matter]
    // Then we add the XCMBridgeEdges, which create the native tokens. If two tokens are connected
    // over a bridge, we use the fact that the derived_usd and derived_eth must be equal to set the
    // native tokens' derived_eth and derived_usd
    // Finally we add the WrapEdges and UnwrapEdges

    // 1. Add ConstantProductAMMSwapEdges from each DEX (and connecting XC20, ERC20 vertices)
    {
        let mut token_id_set: HashSet<UniversalTokenId> = HashSet::new();
        for chain_id in chain_ids.iter() {
            let chain_info =
                get_chain_info_from_chain_id(chain_id).ok_or(PublicError::UnregisteredChainId)?;

            let dexes = get_dexes_from_chain_id(chain_id);
            for dex in dexes.into_iter() {
                let _ = update_graph_with_dex(dex, chain_info, &mut token_id_set, &mut graph)?;
            }
        }
    }

    // 2. Add XCMBridgeEdges (and connecting XC20 vertices)
    for xcm_bridge in xcm_bridge_registry::XCM_BRIDGES.iter() {
        let _ = update_graph_with_xcm_bridge(xcm_bridge, &mut graph)?;
    }

    // 3. Add WrapEdge and UnwrapEdge. We expect that the wrapped native ERC20 tokens is already
    // added to the graph, but Native tokens need not have been added (if the continue block
    // was hit in step 2)
    for chain_id in chain_ids.iter() {
        let _ = update_graph_with_wrap_edges(chain_id, &mut graph)?;
    }

    Ok(graph)
}

fn update_graph_with_dex<'a>(
    dex: &'static Dex,
    chain_info: &'static ChainInfo,
    token_id_set: &'a mut HashSet<UniversalTokenId>,
    graph: &'a mut Graph,
) -> Result<()> {
    let (tokens, edges) = get_additional_tokens_and_edges(
        dex,
        MIN_TOKEN_PAIR_RESERVE_USD,
        chain_info.avg_gas_fee_in_native_token,
        token_id_set,
    )?;
    // ink_env::debug_println!("let tokens: Vec<Token> = vec!{:?};", tokens);
    // ink_env::debug_println!("let edges: Vec<ConstantProductAMMSwapEdge> = vec!{:?};", edges);
    for token in tokens.into_iter() {
        let _ = graph.add_vertex(token);
    }
    for edge in edges.into_iter() {
        let _ = graph.add_edge(Edge::Swap(SwapEdge::CPMM(edge)))?;
    }
    Ok(())
}

/// Only should be called externally by tests!
pub fn update_graph_with_xcm_bridge<'a, 'b>(
    xcm_bridge: &'a XCMBridge,
    graph: &'b mut Graph,
) -> Result<()> {
    let (src_token_derived_eth, dest_token_derived_eth, token_derived_usd) = {
        match (
            graph.get_token(&xcm_bridge.src_token),
            xcm_bridge.src_token.id == ChainTokenId::Native,
            graph.get_token(&xcm_bridge.dest_token),
            xcm_bridge.dest_token.id == ChainTokenId::Native,
        ) {
            (Some(src), _, Some(dest), _) => (
                src.derived_eth.clone(),
                dest.derived_eth.clone(),
                dest.derived_usd.clone(),
            ),
            (Some(src), _, None, /* is_native = */ true) => {
                let dest = Token {
                    id: xcm_bridge.dest_token.clone(),
                    derived_eth: DecimalFixedPoint::from_str_and_exp("1", 0), // Native token's derived_eth is 1 by definition
                    derived_usd: src.derived_usd.clone(),
                };
                let (src_derived_eth, dest_derived_eth, derived_usd) = (
                    src.derived_eth.clone(),
                    dest.derived_eth.clone(),
                    src.derived_usd.clone(),
                );
                let _ = graph.add_vertex(dest);
                (src_derived_eth, dest_derived_eth, derived_usd)
            }
            (None, /* is_native = */ true, Some(dest), _) => {
                let src = Token {
                    id: xcm_bridge.src_token.clone(),
                    derived_eth: DecimalFixedPoint::from_str_and_exp("1", 0), // Native token's derived_eth is 1 by definition
                    derived_usd: dest.derived_usd.clone(),
                };
                let (src_derived_eth, dest_derived_eth, derived_usd) = (
                    src.derived_eth.clone(),
                    dest.derived_eth.clone(),
                    src.derived_usd.clone(),
                );
                let _ = graph.add_vertex(src);
                (src_derived_eth, dest_derived_eth, derived_usd)
            }
            // If a token in the bridge is not the Native token and was not
            // included in the SwapEdges, we just skip adding the edge (and
            // the corresponding tokens)
            _ => {
                return Ok(());
            }
        }
    };
    graph.add_edge(Edge::Bridge(BridgeEdge::Xcm(
        XCMBridgeEdge::from_bridge_and_derived_quantities(
            xcm_bridge.clone(),
            &src_token_derived_eth,
            &dest_token_derived_eth,
            &token_derived_usd,
        ),
    )))
}

/// Only should be called externally by tests!
pub fn update_graph_with_wrap_edges<'a, 'b>(
    chain_id: &'a UniversalChainId,
    graph: &'b mut Graph,
) -> Result<()> {
    let chain_info =
        get_chain_info_from_chain_id(chain_id).ok_or(PublicError::UnregisteredChainId)?;

    if let Some(weth_addr) = chain_info.weth_addr {
        // This should always be an ERC20 (not XC20) but we call this function to avoid hard-coding an ERC20
        let wrapped_native =
            universal_token_id_registry::chain_and_eth_addr_to_token(chain_id.clone(), weth_addr);
        let native_token = UniversalTokenId {
            chain: chain_id.clone(),
            id: ChainTokenId::Native,
        };
        let native_token_usd = graph
            .get_token(&wrapped_native)
            .ok_or(PublicError::VertexNotInGraph(wrapped_native.clone()))?
            .derived_usd
            .clone();
        if graph.get_token(&native_token).is_none() {
            let native = Token {
                id: native_token.clone(),
                derived_eth: DecimalFixedPoint::from_str_and_exp("1", 0), // Native token's derived_eth is 1 by definition
                derived_usd: native_token_usd.clone(),
            };
            let _ = graph.add_vertex(native);
        }
        let _ = graph.add_edge(Edge::Swap(SwapEdge::Wrap(WrapEdge {
            src_token: native_token.clone(),
            dest_token: wrapped_native.clone(),
            // Wrapped native token is 1:1 for native token so we can leave gas fee in terms of native token
            estimated_gas_fee_in_dest_token: chain_info.avg_gas_fee_in_native_token,
            estimated_gas_fee_usd: native_token_usd
                .add_exp(USD_AMOUNT_EXPONENT as i8)
                .mul_u128(chain_info.avg_gas_fee_in_native_token),
        })))?;
        let _ = graph.add_edge(Edge::Swap(SwapEdge::Unwrap(UnwrapEdge {
            src_token: wrapped_native.clone(),
            dest_token: native_token.clone(),
            estimated_gas_fee_in_dest_token: chain_info.avg_gas_fee_in_native_token,
            estimated_gas_fee_usd: native_token_usd
                .add_exp(USD_AMOUNT_EXPONENT as i8)
                .mul_u128(chain_info.avg_gas_fee_in_native_token),
        })))?;
    }
    Ok(())
}

#[cfg(test)]
mod graph_builder_tests {
    use super::*;
    use ink_env::debug_println;
    use privadex_chain_metadata::registry::chain::universal_chain_id_registry::{
        ASTAR, MOONBEAM, POLKADOT,
    };

    #[test]
    fn test() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let chain_ids: Vec<UniversalChainId> = vec![ASTAR, MOONBEAM, POLKADOT];
        let graph = create_graph_from_chain_ids(&chain_ids).unwrap();
        debug_println!("Vertex count: {}", graph.simple_graph.vertex_count());
        debug_println!("Edge count: {}", graph.simple_graph.edge_count());
        assert!(graph.simple_graph.vertex_count() > 0);
        assert!(graph.simple_graph.edge_count() > 0);
    }
}
