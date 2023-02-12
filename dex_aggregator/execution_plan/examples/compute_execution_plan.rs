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

use hex_literal::hex;
use ink_env::debug_println;

use privadex_chain_metadata::{
    common::{
        ChainTokenId, ERC20Token, EthAddress,
        UniversalChainId::{self, SubstrateParachain},
        UniversalTokenId,
    },
    registry::chain::{
        universal_chain_id_registry::{ASTAR, MOONBEAM, POLKADOT},
        RelayChain::Polkadot,
    },
};
use privadex_execution_plan::execution_plan::ExecutionPlan;
use privadex_routing::graph_builder::create_graph_from_chain_ids;

const DUMMY_SRC_ADDR: EthAddress = EthAddress {
    0: hex!("fedcba98765432100123456789abcdef00010203"),
};
const DUMMY_DEST_ADDR: EthAddress = EthAddress {
    0: hex!("000102030405060708090a0b0c0d0e0f10111213"),
};

fn main() {
    pink_extension_runtime::mock_ext::mock_all_ext();

    let chain_ids: Vec<UniversalChainId> = vec![ASTAR, MOONBEAM, POLKADOT];
    let graph = create_graph_from_chain_ids(&chain_ids).unwrap();
    debug_println!("Vertex count: {}", graph.simple_graph.vertex_count());
    debug_println!("Edge count: {}", graph.simple_graph.edge_count());

    let graph_solution = {
        let amount_in = 100_000_000_000_000_000;
        let src_token_id = UniversalTokenId {
            chain: SubstrateParachain(Polkadot, 2006),
            id: ChainTokenId::ERC20(ERC20Token {
                addr: EthAddress {
                    0: hex!("29f6e49c6e3397c3a84f715885f9f233a441165c"),
                },
            }),
        };
        let dest_token_id = UniversalTokenId {
            chain: SubstrateParachain(Polkadot, 2004),
            id: ChainTokenId::ERC20(ERC20Token {
                addr: EthAddress {
                    0: hex!("c9baa8cfdde8e328787e29b4b078abf2dadc2055"),
                },
            }),
        };

        let sor_config =
            privadex_routing::smart_order_router::single_path_sor::SORConfig::default();
        let sor = privadex_routing::smart_order_router::single_path_sor::SinglePathSOR::new(
            &graph,
            DUMMY_SRC_ADDR,
            DUMMY_DEST_ADDR,
            src_token_id.clone(),
            dest_token_id.clone(),
            sor_config,
        );
        sor.compute_graph_solution(amount_in)
            .expect("We expect a graph solution")
    };
    let exec_plan =
        ExecutionPlan::try_from(graph_solution.clone()).expect("We expect an execution plan");
    debug_println!("{}\n\n{}", graph_solution, exec_plan);
    debug_println!("Generated execution plan!");
}
