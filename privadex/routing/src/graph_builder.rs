use alloc::string::ToString;
use hashbrown::HashSet;
use privadex_chain_metadata::chain_info::ChainInfo;
use privadex_chain_metadata::common::{ChainTokenId, Dex, UniversalChainId, UniversalTokenId};
use privadex_chain_metadata::{
    get_chain_info_from_chain_id,
    get_dexes_from_chain_id,
    bridge::XCMBridge,
    registry::{
        bridge::xcm_bridge_registry,
        token::universal_token_id_registry,
    }
};

use crate::graph::{
    edge::{Edge, SwapEdge, ConstantProductAMMSwapEdge, BridgeEdge, XCMBridgeEdge, WrapEdge, UnwrapEdge},
    graph::{Graph, Token},
};
use crate::graphql_client::get_additional_tokens_and_edges;
use crate::{PublicError, Result};

// Set low enough so that we include the ASTR/GLMR pool in ArthSwap
const MIN_TOKEN_PAIR_RESERVE_USD: u32 = 5_000;


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
            let chain_info = get_chain_info_from_chain_id(chain_id).ok_or(PublicError::UnregisteredChainId)?;
            
            let dexes = get_dexes_from_chain_id(chain_id);
            for dex in dexes.into_iter() {
                let _ = update_graph_with_dex(dex, chain_info, &mut token_id_set, &mut graph)?;
            }
        }
    }

    // 2. Add XCMBridgeEdges (and connecting XC20 vertices)
    for xcm_bridge in xcm_bridge_registry::XCM_BRIDGES.iter() {
        update_graph_with_xcm_bridge(xcm_bridge, &mut graph);
    }

    // 3. Add WrapEdge and UnwrapEdge. We expect that the wrapped native ERC20 tokens is already
    // added to the graph, but Native tokens need not have been added (if the continue block
    // was hit in step 2)
    for chain_id in chain_ids.iter() {
        let _ = update_graph_with_wrap_edges(chain_id, &mut graph)?;
    }

    Ok(graph)
}


#[cfg(test)]
mod graph_builder_tests {
    use hex_literal::hex;
    use ink_env::debug_println;
    use super::*;
    use privadex_chain_metadata::registry::chain::universal_chain_id_registry::{
        ASTAR,
        MOONBEAM,
        POLKADOT,
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
