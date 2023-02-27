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

use core::str::FromStr;
use hex_literal::hex;
use ink_env::debug_println;
use ink_prelude::vec;
use std::{thread, time::Duration};

use privadex_chain_metadata::{
    common::{
        ChainTokenId, ERC20Token, EthAddress, SecretKeyContainer, SubstratePublicKey,
        UniversalAddress, UniversalChainId, UniversalTokenId,
    },
    registry::{
        chain::{chain_info_registry, universal_chain_id_registry},
        token::universal_token_id_registry,
    },
};
use privadex_execution_plan::execution_plan::{
    EthPendingTxnId, EthStepStatus, ExecutionPlan, ExecutionStepEnum,
};
use privadex_executor::{
    eth_utils::{
        common::{block_number, get_next_system_nonce, send_raw_transaction},
        erc20_contract::ERC20Contract,
    },
    executable::{
        execute_step_meta::ExecuteStepMeta,
        traits::{Executable, ExecutableSimpleStatus, StepForwardResult},
    },
    key_container::{AddressKeyPair, KeyContainer},
};
use privadex_routing;

// This sends out real transactions on Astar, so you need to use an
// account with sufficient funds and allowance granted on the necessary
// DEXes!
fn main() {
    pink_extension_runtime::mock_ext::mock_all_ext();

    let (execute_step_meta, keys) = get_state();

    let chain_ids: Vec<UniversalChainId> = vec![
        universal_chain_id_registry::ASTAR,
        universal_chain_id_registry::MOONBEAM,
        universal_chain_id_registry::POLKADOT,
    ];
    debug_println!("Creating token graph from price feed...");
    let graph = privadex_routing::graph_builder::create_graph_from_chain_ids(&chain_ids).unwrap();
    debug_println!("Vertex count: {}", graph.simple_graph.vertex_count());
    debug_println!("Edge count: {}", graph.simple_graph.edge_count());

    let user_eth_addr = EthAddress {
        0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
    };
    let initial_amount = 1_000_000_000; // 0.1 xcDOT
    let src_token_id = universal_token_id_registry::DOT_ASTAR;
    let graph_solution = {
        // GLINT (BeamSwap token)
        let dest_token_id = UniversalTokenId {
            chain: universal_chain_id_registry::MOONBEAM,
            id: ChainTokenId::ERC20(ERC20Token {
                addr: EthAddress {
                    0: hex!("cd3b51d98478d53f4515a306be565c6eebef1d58"),
                },
            }),
        };

        let sor_config =
            privadex_routing::smart_order_router::single_path_sor::SORConfig::default();
        let sor = privadex_routing::smart_order_router::single_path_sor::SinglePathSOR::new(
            &graph,
            user_eth_addr.clone(),
            user_eth_addr.clone(),
            src_token_id.clone(),
            dest_token_id.clone(),
            sor_config,
        );
        debug_println!("Computing best route from source to dest...");
        sor.compute_graph_solution(initial_amount)
            .expect("We expect a graph solution")
    };
    let mut exec_plan =
        ExecutionPlan::try_from(graph_solution.clone()).expect("We expect an execution plan");
    debug_println!(
        "Graph Solution (quote in dest_token) = {}, total estimated fees = ${:.4}): {}\n",
        graph_solution.get_quote_with_estimated_txn_fees(),
        graph_solution.get_estimated_txn_fees_usd(),
        graph_solution,
    );
    debug_println!(
        "Generated execution plan. State: {:?}, {}\n",
        exec_plan.get_status(),
        exec_plan
    );
    debug_println!(
        "Will run e2e_plan_mainnets example in 10 seconds. This uses real (mainnet) funds!"
    );
    thread::sleep(Duration::from_millis(10_000));

    assert_eq!(exec_plan.get_status(), ExecutableSimpleStatus::NotStarted);
    assert_eq!(exec_plan.get_total_fee_usd(), None);

    // Prestart step
    {
        let astar_chain_info = chain_info_registry::ASTAR_INFO;
        let xcdot = ERC20Contract::new(
            &astar_chain_info.rpc_url,
            EthAddress {
                0: hex!("FFfFfFffFFfffFFfFFfFFFFFffFFFffffFfFFFfF"),
            },
        )
        .expect("Valid ERC20Contract");
        let nonce = get_next_system_nonce(&astar_chain_info.rpc_url, user_eth_addr)
            .expect("Expected successful get_next_nonce");
        let cur_block =
            block_number(&astar_chain_info.rpc_url).expect("Expected successful block_number");
        let escrow_eth_addr = EthAddress {
            0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
        };
        let signed_txn = xcdot
            .transfer(
                escrow_eth_addr,
                initial_amount,
                keys.get_key(&UniversalAddress::Ethereum(user_eth_addr))
                    .expect("Key must exist"),
                nonce,
            )
            .expect("Expected signed txn");
        if let ExecutionStepEnum::ERC20Transfer(xcdot_transfer) =
            &mut exec_plan.prestart_user_to_escrow_transfer.inner
        {
            xcdot_transfer.status = EthStepStatus::Submitted(EthPendingTxnId {
                txn_hash: signed_txn.transaction_hash,
                end_block_num: cur_block + 50,
            });
            let res = send_raw_transaction(&astar_chain_info.rpc_url, signed_txn);
            debug_println!("Executing prestart step: {:?}", res);
        } else {
            assert!(false); // We hard-code an ERC20 transfer at the start
        }
    }

    // Execute on ExecutionPlan
    while exec_plan.get_status() == ExecutableSimpleStatus::NotStarted
        || exec_plan.get_status() == ExecutableSimpleStatus::InProgress
    {
        let res = exec_plan.execute_step_forward(&execute_step_meta, &keys);
        debug_println!("Step forward result: {:?}", res);
        if let Ok(StepForwardResult {
            did_status_change: true,
            ..
        }) = res
        {
            let time_delta_secs = (now_millis() - execute_step_meta.cur_timestamp()) / 1000;
            debug_println!(
                "[{} secs elapsed] State: {:?}, {}\n",
                time_delta_secs,
                exec_plan.get_status(),
                exec_plan
            );
        }
        thread::sleep(Duration::from_millis(4000));
    }

    assert_eq!(exec_plan.get_status(), ExecutableSimpleStatus::Succeeded);
    assert!(exec_plan.get_total_fee_usd().is_some());
}

fn get_state() -> (ExecuteStepMeta, KeyContainer) {
    let escrow_eth_addr = UniversalAddress::Ethereum(EthAddress {
        0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
    });
    let astar_native_addr = UniversalAddress::Substrate(SubstratePublicKey {
        0: hex!("5134c7f0e31c2a9e19dceddb7403b2836c69cce0b0719d2f58ec0d4da35129be"),
    });
    let escrow_substrate_addr = UniversalAddress::Substrate(SubstratePublicKey {
        0: hex!("7011b670bb662eedbd60a1c4c11b7c197ec22e7cfe87df00013ca2c494f3b01a"),
    });
    let keys = KeyContainer {
        0: vec![
            AddressKeyPair {
                address: escrow_eth_addr.clone(),
                key: SecretKeyContainer::from_str(
                    &std::env::var("ETH_PRIVATE_KEY").expect("Env var ETH_PRIVATE_KEY is not set"),
                )
                .expect("ETH_PRIVATE_KEY to_hex failed")
                .0,
            },
            AddressKeyPair {
                address: astar_native_addr.clone(),
                key: SecretKeyContainer::from_str(
                    &std::env::var("ETH_PRIVATE_KEY").expect("Env var ETH_PRIVATE_KEY is not set"),
                )
                .expect("ETH_PRIVATE_KEY to_hex failed")
                .0,
            },
            AddressKeyPair {
                address: escrow_substrate_addr.clone(),
                key: SecretKeyContainer::from_str(
                    &std::env::var("SUBSTRATE_PRIVATE_KEY")
                        .expect("Env var SUBSTRATE_PRIVATE_KEY is not set"),
                )
                .expect("SUBSTRATE_PRIVATE_KEY to_hex failed")
                .0,
            },
        ],
    };
    let execute_step_meta = ExecuteStepMeta::dummy(now_millis());
    (execute_step_meta, keys)
}

fn now_millis() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .try_into()
        .unwrap()
}
