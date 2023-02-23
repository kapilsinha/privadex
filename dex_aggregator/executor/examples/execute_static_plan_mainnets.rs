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
use xcm::prelude::{Junction, Junctions, MultiLocation, NetworkId};

use privadex_chain_metadata::{
    common::{
        ChainTokenId, ERC20Token, EthAddress, SecretKeyContainer, SubstratePublicKey,
        UniversalAddress, UniversalTokenId,
    },
    registry::{
        chain::{chain_info_registry, universal_chain_id_registry},
        dex::dex_registry,
        token::universal_token_id_registry,
    },
};
use privadex_common::uuid::Uuid;
use privadex_execution_plan::execution_plan::{
    CommonExecutionMeta, CrossChainStepStatus, DexRouterFunction, ERC20TransferStep,
    EthDexSwapStep, EthPendingTxnId, EthSendStep, EthStepStatus, ExecutionPath, ExecutionPlan,
    ExecutionStep, ExecutionStepEnum, XCMTransferStep,
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

// This sends out real transactions on Moonbeam, so you need to use an
// account with sufficient funds and allowance granted on the necessary
// DEXes!
// 0.05 xcDOT on Moonbeam -> native GLMR -> xcGLMR on Astar -> WASTR
fn main() {
    pink_extension_runtime::mock_ext::mock_all_ext();

    let (addr, execute_step_meta, keys) = get_state();
    let astar_substrate_address = UniversalAddress::Substrate(SubstratePublicKey {
        0: hex!("5134c7f0e31c2a9e19dceddb7403b2836c69cce0b0719d2f58ec0d4da35129be"),
    });
    let eth_addr = EthAddress {
        0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
    };
    let raw_astar_substrate_addr =
        hex!("5134c7f0e31c2a9e19dceddb7403b2836c69cce0b0719d2f58ec0d4da35129be");
    let moonbeam_chain_info = chain_info_registry::MOONBEAM_INFO;
    let astar_chain_info = chain_info_registry::ASTAR_INFO;
    let initial_amount = 500_000_000;
    let mut exec_plan = ExecutionPlan {
        uuid: Uuid::new([1u8; 16]),
        prestart_user_to_escrow_transfer: ExecutionStep::new(ExecutionStepEnum::ERC20Transfer(
            ERC20TransferStep {
                uuid: Uuid::new([2u8; 16]),
                token: universal_token_id_registry::DOT_MOONBEAM,
                amount: Some(initial_amount), // 0.05 xcDOT
                common: CommonExecutionMeta {
                    src_addr: addr.clone(),
                    dest_addr: addr.clone(),
                    gas_fee_native: 10_000_000_000_000_000,
                    gas_fee_usd: 3_000_000_000_000_000,
                },
                status: EthStepStatus::NotStarted,
            },
        )),
        paths: vec![ExecutionPath {
            steps: vec![
                ExecutionStep::new(ExecutionStepEnum::EthDexSwap(EthDexSwapStep {
                    uuid: Uuid::new([3u8; 16]),
                    dex_router_addr: dex_registry::STELLASWAP.eth_dex_router,
                    dex_router_func: DexRouterFunction::SwapExactTokensForETH,
                    token_path: vec![
                        // xcDOT
                        universal_token_id_registry::DOT_MOONBEAM,
                        // WGLMR
                        UniversalTokenId {
                            chain: universal_chain_id_registry::MOONBEAM,
                            id: ChainTokenId::ERC20(ERC20Token {
                                addr: moonbeam_chain_info.weth_addr.expect("WGLMR exists"),
                            }),
                        },
                    ],
                    amount_in: Some(initial_amount),
                    common: CommonExecutionMeta {
                        src_addr: addr.clone(),
                        dest_addr: addr.clone(),
                        gas_fee_native: 10_000_000_000_000_000,
                        gas_fee_usd: 3_000_000_000_000_000,
                    },
                    status: EthStepStatus::NotStarted,
                })),
                ExecutionStep::new(ExecutionStepEnum::XCMTransfer(XCMTransferStep {
                    uuid: Uuid::new([4u8; 16]),
                    // Native GLMR
                    src_token: universal_token_id_registry::GLMR_NATIVE,
                    // xcGLMR on Astar
                    dest_token: universal_token_id_registry::GLMR_ASTAR,
                    token_asset_multilocation: MultiLocation {
                        parents: 0u8,
                        interior: Junctions::X1(Junction::PalletInstance(10)),
                    },
                    full_dest_multilocation: MultiLocation {
                        parents: 1u8,
                        interior: Junctions::X2(
                            Junction::Parachain(2006u32),
                            Junction::AccountId32 {
                                network: NetworkId::Any,
                                id: raw_astar_substrate_addr,
                            },
                        ),
                    },
                    amount_in: None,
                    bridge_fee_native: 200_000_000_000_000,
                    bridge_fee_usd: 10_000_000_000_000,
                    common: CommonExecutionMeta {
                        src_addr: addr.clone(),
                        dest_addr: astar_substrate_address.clone(),
                        gas_fee_native: 1_000_000_000,
                        gas_fee_usd: 50_000_000,
                    },
                    status: CrossChainStepStatus::NotStarted,
                })),
                ExecutionStep::new(ExecutionStepEnum::EthDexSwap(EthDexSwapStep {
                    uuid: Uuid::new([5u8; 16]),
                    dex_router_addr: dex_registry::ARTHSWAP.eth_dex_router,
                    dex_router_func: DexRouterFunction::SwapExactTokensForETH,
                    token_path: vec![
                        // xcGLMR
                        universal_token_id_registry::GLMR_ASTAR,
                        // WASTR
                        UniversalTokenId {
                            chain: universal_chain_id_registry::ASTAR,
                            id: ChainTokenId::ERC20(ERC20Token {
                                addr: astar_chain_info.weth_addr.expect("WASTR exists"),
                            }),
                        },
                    ],
                    amount_in: None,
                    common: CommonExecutionMeta {
                        src_addr: addr.clone(),
                        dest_addr: addr.clone(),
                        gas_fee_native: 1_000_000_000,
                        gas_fee_usd: 50_000_000,
                    },
                    status: EthStepStatus::NotStarted,
                })),
            ],
            amount_out: None,
        }],
        postend_escrow_to_user_transfer: ExecutionStep::new(ExecutionStepEnum::EthSend(
            EthSendStep {
                uuid: Uuid::new([6u8; 16]),
                chain: universal_chain_id_registry::ASTAR,
                amount: None,
                common: CommonExecutionMeta {
                    src_addr: addr.clone(),
                    dest_addr: addr.clone(),
                    gas_fee_native: 1_000_000_000,
                    gas_fee_usd: 50_000_000,
                },
                status: EthStepStatus::NotStarted,
            },
        )),
    };
    debug_println!("State: {:?}, {}\n", exec_plan.get_status(), exec_plan);
    debug_println!(
        "Will run static_plan_mainnets example in 5 seconds. This uses real (mainnet) funds!"
    );
    thread::sleep(Duration::from_millis(5000));

    assert_eq!(exec_plan.get_status(), ExecutableSimpleStatus::NotStarted);
    assert_eq!(exec_plan.get_total_fee_usd(), None);

    // Prestart step
    {
        let xcdot = ERC20Contract::new(
            &moonbeam_chain_info.rpc_url,
            EthAddress {
                0: hex!("FfFFfFff1FcaCBd218EDc0EbA20Fc2308C778080"),
            },
        )
        .expect("Valid ERC20Contract");
        let nonce = get_next_system_nonce(&moonbeam_chain_info.rpc_url, eth_addr)
            .expect("Expected successful get_next_nonce");
        let cur_block =
            block_number(&moonbeam_chain_info.rpc_url).expect("Expected successful block_number");
        let signed_txn = xcdot
            .transfer(
                eth_addr,
                initial_amount,
                keys.get_key(&addr).expect("Key must exist"),
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
            let res = send_raw_transaction(&moonbeam_chain_info.rpc_url, signed_txn);
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

fn get_state() -> (UniversalAddress, ExecuteStepMeta, KeyContainer) {
    let escrow_addr = UniversalAddress::Ethereum(EthAddress {
        0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
    });
    let keys = KeyContainer {
        0: vec![AddressKeyPair {
            address: escrow_addr.clone(),
            key: SecretKeyContainer::from_str(
                &std::env::var("ETH_PRIVATE_KEY").expect("Env var ETH_PRIVATE_KEY is not set"),
            )
            .expect("ETH_PRIVATE_KEY to_hex failed")
            .0,
        }],
    };
    let execute_step_meta = ExecuteStepMeta::dummy(now_millis());
    (escrow_addr, execute_step_meta, keys)
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
