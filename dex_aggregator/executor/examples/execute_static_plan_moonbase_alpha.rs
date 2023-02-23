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
        ChainTokenId, ERC20Token, EthAddress, SecretKeyContainer, UniversalAddress,
        UniversalTokenId, XC20Token,
    },
    registry::{
        chain::{chain_info_registry, universal_chain_id_registry},
        dex::dex_registry,
    },
};
use privadex_common::uuid::Uuid;
use privadex_execution_plan::execution_plan::{
    CommonExecutionMeta, CrossChainStepStatus, DexRouterFunction, ERC20TransferStep,
    EthDexSwapStep, EthPendingTxnId, EthStepStatus, EthUnwrapStep, EthWrapStep, ExecutionPath,
    ExecutionPlan, ExecutionStep, ExecutionStepEnum, XCMTransferStep,
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

// This sends out real transactions on Moonbase Alpha, so you need to use an
// account with sufficient funds!
// Note: This test actually does not terminate because there is no Subsquid archive
// URL for Moonbase Beta, and so it cannot confirm that funds have been received on
// the remote end. We can modify the test to have it terminate, but I prefer to keep
// it as-is since that is closer to the behavior of mainnet chains.
fn main() {
    pink_extension_runtime::mock_ext::mock_all_ext();

    debug_println!(
        "Will run static_plan_moonbase_alpha example in 3 seconds. This uses real (testnet) funds!"
    );
    thread::sleep(Duration::from_millis(3000));

    let (addr, execute_step_meta, keys) = get_state();
    let eth_addr = EthAddress {
        0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
    };
    let chain_info = chain_info_registry::MOONBASEALPHA_INFO;
    let initial_amount = 3_000_000_000_000_000_000;
    let mut exec_plan = ExecutionPlan {
        uuid: Uuid::new([0u8; 16]),
        prestart_user_to_escrow_transfer: ExecutionStep::new(ExecutionStepEnum::ERC20Transfer(
            ERC20TransferStep {
                uuid: Uuid::new([0u8; 16]),
                token: UniversalTokenId {
                    chain: universal_chain_id_registry::MOONBASE_ALPHA,
                    id: ChainTokenId::ERC20(ERC20Token {
                        addr: EthAddress {
                            0: hex!("CdF746C5C86Df2c2772d2D36E227B4c0203CbA25"),
                        },
                    }),
                },
                amount: Some(initial_amount), // 3 VEN
                common: CommonExecutionMeta {
                    src_addr: addr.clone(),
                    dest_addr: addr.clone(),
                    gas_fee_native: 1_000_000_000,
                    gas_fee_usd: 2_000_0000_000,
                },
                status: EthStepStatus::NotStarted,
            },
        )),
        paths: vec![ExecutionPath {
            steps: vec![
                ExecutionStep::new(ExecutionStepEnum::EthDexSwap(EthDexSwapStep {
                    uuid: Uuid::new([0u8; 16]),
                    dex_router_addr: dex_registry::MOONBASE_UNISWAP.eth_dex_router,
                    dex_router_func: DexRouterFunction::SwapExactTokensForETH,
                    token_path: vec![
                        // VEN
                        UniversalTokenId {
                            chain: universal_chain_id_registry::MOONBASE_ALPHA,
                            id: ChainTokenId::ERC20(ERC20Token {
                                addr: EthAddress {
                                    0: hex!("CdF746C5C86Df2c2772d2D36E227B4c0203CbA25"),
                                },
                            }),
                        },
                        // WDEV
                        UniversalTokenId {
                            chain: universal_chain_id_registry::MOONBASE_ALPHA,
                            id: ChainTokenId::ERC20(ERC20Token {
                                addr: chain_info.weth_addr.expect("WDEV exists"),
                            }),
                        },
                    ],
                    amount_in: Some(initial_amount),
                    common: CommonExecutionMeta {
                        src_addr: addr.clone(),
                        dest_addr: addr.clone(),
                        gas_fee_native: 1_000_000_000,
                        gas_fee_usd: 2_000_0000_000,
                    },
                    status: EthStepStatus::NotStarted,
                })),
                ExecutionStep::new(ExecutionStepEnum::EthWrap(EthWrapStep {
                    uuid: Uuid::new([0u8; 16]),
                    chain: universal_chain_id_registry::MOONBASE_ALPHA,
                    amount: None,
                    common: CommonExecutionMeta {
                        src_addr: addr.clone(),
                        dest_addr: addr.clone(),
                        gas_fee_native: 1_000_000_000,
                        gas_fee_usd: 2_000_0000_000,
                    },
                    status: EthStepStatus::NotStarted,
                })),
                ExecutionStep::new(ExecutionStepEnum::EthUnwrap(EthUnwrapStep {
                    uuid: Uuid::new([0u8; 16]),
                    chain: universal_chain_id_registry::MOONBASE_ALPHA,
                    amount: None,
                    common: CommonExecutionMeta {
                        src_addr: addr.clone(),
                        dest_addr: addr.clone(),
                        gas_fee_native: 1_000_000_000,
                        gas_fee_usd: 2_000_0000_000,
                    },
                    status: EthStepStatus::NotStarted,
                })),
                ExecutionStep::new(ExecutionStepEnum::XCMTransfer(XCMTransferStep {
                    uuid: Uuid::new([0u8; 16]),
                    // Native DEV
                    src_token: UniversalTokenId {
                        chain: universal_chain_id_registry::MOONBASE_ALPHA,
                        id: ChainTokenId::Native,
                    },
                    // xcDEV
                    dest_token: UniversalTokenId {
                        chain: universal_chain_id_registry::MOONBASE_BETA,
                        id: ChainTokenId::XC20(XC20Token::from_asset_id(
                            222_902_676_330_054_289_648_817_870_329_963_141_953,
                        )),
                    },
                    token_asset_multilocation: MultiLocation {
                        parents: 0u8,
                        interior: Junctions::X1(Junction::PalletInstance(3u8)),
                    },
                    full_dest_multilocation: MultiLocation {
                        parents: 1u8,
                        interior: Junctions::X2(
                            Junction::Parachain(888u32),
                            Junction::AccountKey20 {
                                network: NetworkId::Any,
                                key: hex!("05a81d8564a3ea298660e34e03e5eff9a29d7a2a"),
                            },
                        ),
                    },
                    amount_in: None,
                    bridge_fee_native: 100_000_000,
                    bridge_fee_usd: 3_000_000_000_000,
                    common: CommonExecutionMeta {
                        src_addr: addr.clone(),
                        dest_addr: addr.clone(),
                        gas_fee_native: 1_000_000_000,
                        gas_fee_usd: 2_000_0000_000,
                    },
                    status: CrossChainStepStatus::NotStarted,
                })),
            ],
            amount_out: None,
        }],
        postend_escrow_to_user_transfer: ExecutionStep::new(ExecutionStepEnum::ERC20Transfer(
            ERC20TransferStep {
                uuid: Uuid::new([0u8; 16]),
                // xcDEV
                token: UniversalTokenId {
                    chain: universal_chain_id_registry::MOONBASE_ALPHA,
                    id: ChainTokenId::XC20(XC20Token::from_asset_id(
                        222_902_676_330_054_289_648_817_870_329_963_141_953,
                    )),
                },
                amount: None,
                common: CommonExecutionMeta {
                    src_addr: addr.clone(),
                    dest_addr: addr.clone(),
                    gas_fee_native: 1_000_000_000,
                    gas_fee_usd: 2_000_0000_000,
                },
                status: EthStepStatus::NotStarted,
            },
        )),
    };
    assert_eq!(exec_plan.get_status(), ExecutableSimpleStatus::NotStarted);
    assert_eq!(exec_plan.get_total_fee_usd(), None);

    // Prestart step
    {
        let venus = ERC20Contract::new(
            &chain_info.rpc_url,
            EthAddress {
                0: hex!("CdF746C5C86Df2c2772d2D36E227B4c0203CbA25"),
            },
        )
        .expect("Valid ERC20Contract");
        let nonce = get_next_system_nonce(&chain_info.rpc_url, eth_addr)
            .expect("Expected successful get_next_nonce");
        let cur_block =
            block_number(&chain_info.rpc_url).expect("Expected successful block_number");
        let signed_txn = venus
            .transfer(
                eth_addr,
                initial_amount,
                keys.get_key(&addr).expect("Key must exist"),
                nonce,
            )
            .expect("Expected signed txn");
        if let ExecutionStepEnum::ERC20Transfer(ven_transfer) =
            &mut exec_plan.prestart_user_to_escrow_transfer.inner
        {
            ven_transfer.status = EthStepStatus::Submitted(EthPendingTxnId {
                txn_hash: signed_txn.transaction_hash,
                end_block_num: cur_block + 50,
            });
            let res = send_raw_transaction(&chain_info.rpc_url, signed_txn);
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
