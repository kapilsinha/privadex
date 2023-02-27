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

use privadex_chain_metadata::common::Amount;
use privadex_common::utils::general_utils::mul_ratio_u128;
use privadex_execution_plan::execution_plan::{ExecutionStep, ExecutionStepEnum};

use crate::key_container::KeyContainer;

use super::{
    execute_step_meta::ExecuteStepMeta,
    traits::{Executable, ExecutableResult, ExecutableSimpleStatus, StepForwardResult},
};

// After this many blocks, we assume the txn is dropped
// 12 seconds per block * 64 block ~ 768 seconds
// This is also used for Era, which requires this to be a power of 2!
pub const TXN_NUM_BLOCKS_ALIVE: u32 = 64;

impl Executable for ExecutionStep {
    fn get_status(&self) -> ExecutableSimpleStatus {
        match &self.inner {
            ExecutionStepEnum::EthSend(step) => step.get_status(),
            ExecutionStepEnum::ERC20Transfer(step) => step.get_status(),
            ExecutionStepEnum::EthWrap(step) => step.get_status(),
            ExecutionStepEnum::EthUnwrap(step) => step.get_status(),
            ExecutionStepEnum::EthDexSwap(step) => step.get_status(),
            ExecutionStepEnum::XCMTransfer(step) => step.get_status(),
        }
    }

    fn get_total_fee_usd(&self) -> Option<Amount> {
        match &self.inner {
            ExecutionStepEnum::EthSend(step) => step.get_total_fee_usd(),
            ExecutionStepEnum::ERC20Transfer(step) => step.get_total_fee_usd(),
            ExecutionStepEnum::EthWrap(step) => step.get_total_fee_usd(),
            ExecutionStepEnum::EthUnwrap(step) => step.get_total_fee_usd(),
            ExecutionStepEnum::EthDexSwap(step) => step.get_total_fee_usd(),
            ExecutionStepEnum::XCMTransfer(step) => step.get_total_fee_usd(),
        }
    }

    fn execute_step_forward(
        &mut self,
        execute_step_meta: &ExecuteStepMeta,
        keys: &KeyContainer,
    ) -> ExecutableResult<StepForwardResult> {
        let step_forward_res = {
            if self.get_amount_in().unwrap_or(0) > 0 {
                match &mut self.inner {
                    ExecutionStepEnum::EthSend(step) => {
                        step.execute_step_forward(execute_step_meta, keys)
                    }
                    ExecutionStepEnum::ERC20Transfer(step) => {
                        step.execute_step_forward(execute_step_meta, keys)
                    }
                    ExecutionStepEnum::EthWrap(step) => {
                        step.execute_step_forward(execute_step_meta, keys)
                    }
                    ExecutionStepEnum::EthUnwrap(step) => {
                        step.execute_step_forward(execute_step_meta, keys)
                    }
                    ExecutionStepEnum::EthDexSwap(step) => {
                        step.execute_step_forward(execute_step_meta, keys)
                    }
                    ExecutionStepEnum::XCMTransfer(step) => {
                        step.execute_step_forward(execute_step_meta, keys)
                    }
                }?
            } else {
                self.drop(); // Change the status to Dropped
                StepForwardResult {
                    did_status_change: true,
                    amount_out: None,
                }
            }
        };
        let _ = terminate_exec_step_if_dropped_or_finalized(self, execute_step_meta)?;
        Ok(step_forward_res)
    }
}

fn terminate_exec_step_if_dropped_or_finalized(
    exec_step: &ExecutionStep,
    execute_step_meta: &ExecuteStepMeta,
) -> ExecutableResult<()> {
    match exec_step.get_status() {
        ExecutableSimpleStatus::Failed | ExecutableSimpleStatus::Succeeded => {
            execute_step_meta.finalize_execstep(exec_step.get_uuid(), exec_step.get_src_chain())
        }
        ExecutableSimpleStatus::Dropped => {
            execute_step_meta.drop_execstep(exec_step.get_uuid(), exec_step.get_src_chain())
        }
        ExecutableSimpleStatus::NotStarted | ExecutableSimpleStatus::InProgress => Ok(()),
    }
}

// Keep the same token-to-USD rate and update the USD value proportionally
pub fn get_updated_gas_fee_usd(
    updated_gas_fee_native: Amount,
    old_gas_fee_native: Amount,
    old_gas_fee_usd: Amount,
) -> Amount {
    mul_ratio_u128(old_gas_fee_usd, updated_gas_fee_native, old_gas_fee_native)
}

// Ensure that our new int implementation matches the output of our old float implementation
#[cfg(test)]
mod float_tests {
    use super::*;
    use ink_env::debug_println;

    fn get_updated_gas_fee_usd_float(
        updated_gas_fee_native: Amount,
        old_gas_fee_native: Amount,
        old_gas_fee_usd: Amount,
    ) -> Amount {
        ((old_gas_fee_usd as f64) * (updated_gas_fee_native as f64) / (old_gas_fee_native as f64))
            .round() as Amount
    }

    #[test]
    fn test_float_ratio_mul() {
        // Check that our int-only version of the above function (used in executable_step_helpers)
        let updated_gas_fee_native = 3_000_000_000_000_000u128;
        let old_gas_fee_native = 1_000_000_000_000u128;
        let old_gas_fee_usd = 2_000_000_000_000_000_000_000_000_000_000_000u128;
        let float_result = get_updated_gas_fee_usd_float(
            updated_gas_fee_native,
            old_gas_fee_native,
            old_gas_fee_usd,
        );
        let int_result =
            get_updated_gas_fee_usd(updated_gas_fee_native, old_gas_fee_native, old_gas_fee_usd);
        debug_println!("{}, {}", float_result, int_result);
    }
}

// Prerequisites for these tests: You need to have sufficient funds in your account!
// These tests do not actually send out the transaction - we use conditional compilation
// to mock the transaction sending and transaction/extrinsic/event parsing. But the
// funds are needed in the estimate_gas step.
// Mock feature is critical! Otherwise you will send an actual transaction
#[cfg(feature = "mock-txn-send")]
#[cfg(test)]
mod executable_step_tests {
    use core::str::FromStr;
    use hex_literal::hex;
    use ink_env::debug_println;
    use ink_prelude::{vec, vec::Vec};
    use privadex_chain_metadata::{
        common::{
            ChainTokenId, ERC20Token, EthAddress, SecretKeyContainer, SubstratePublicKey,
            UniversalAddress, UniversalChainId, UniversalTokenId,
        },
        registry::{
            bridge::xcm_bridge_registry::XCM_BRIDGES,
            chain::{chain_info_registry, universal_chain_id_registry},
            dex::dex_registry,
            token::universal_token_id_registry,
        },
    };
    use privadex_common::uuid::Uuid;
    use privadex_execution_plan::execution_plan::{
        CommonExecutionMeta, CrossChainStepStatus, DexRouterFunction, ERC20TransferStep,
        EthDexSwapStep, EthSendStep, EthStepStatus, EthUnwrapStep, EthWrapStep, XCMTransferStep,
    };

    use crate::key_container::AddressKeyPair;

    use super::super::traits::ExecutableError;
    use super::*;

    // This cfg isn't needed but exists for added safety
    #[cfg(feature = "mock-txn-send")]
    fn dummy_state() -> (UniversalAddress, ExecuteStepMeta, KeyContainer) {
        let addr = UniversalAddress::Ethereum(EthAddress {
            0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
        });
        let execute_step_meta = ExecuteStepMeta::dummy(u64::MAX);
        (addr, execute_step_meta, dummy_key_container())
    }

    fn dummy_key_container() -> KeyContainer {
        // You need to use an account with sufficient funds or you will see errors in the
        // estimate_gas function call (and end with FailedToCreateTxn)
        let kap_privkey = {
            let privkey_str =
                std::env::var("ETH_PRIVATE_KEY").expect("Env var ETH_PRIVATE_KEY is not set");
            SecretKeyContainer::from_str(&privkey_str)
                .expect("ETH_PRIVATE_KEY to_hex failed")
                .0
        };
        KeyContainer {
            0: vec![AddressKeyPair {
                address: UniversalAddress::Ethereum(EthAddress {
                    0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
                }),
                key: kap_privkey,
            }],
        }
    }

    fn execute_eth_step(mut exec_step: ExecutionStep) {
        let (_, execute_step_meta, keys) = dummy_state();
        assert_eq!(exec_step.get_status(), ExecutableSimpleStatus::NotStarted);
        assert_eq!(exec_step.get_total_fee_usd(), None);

        {
            let res = exec_step
                .execute_step_forward(&execute_step_meta, &keys)
                .expect("Step 1 should succeed");
            debug_println!("1. Step forward result: {:?}", res);
            debug_println!("State: {:?}\n", exec_step.inner);
            assert_eq!(res.did_status_change, true);
            assert_eq!(res.amount_out, None);
            assert_eq!(exec_step.get_status(), ExecutableSimpleStatus::InProgress);
        }

        {
            let res2 = exec_step
                .execute_step_forward(&execute_step_meta, &keys)
                .expect("Step 2 should succeed");
            debug_println!("2. Step forward result: {:?}", res2);
            debug_println!("State: {:?}\n", exec_step.inner);
            assert_eq!(res2.did_status_change, true);
            assert_eq!(res2.amount_out, Some(1_000_000_000));
            assert_eq!(exec_step.get_status(), ExecutableSimpleStatus::Succeeded);
        }

        {
            let res3 = exec_step.execute_step_forward(&execute_step_meta, &keys);
            debug_println!("3. Step forward result: {:?}", res3);
            debug_println!("State: {:?}\n", exec_step.inner);
            assert_eq!(res3, Err(ExecutableError::CalledStepForwardOnFinishedStep));
            assert_eq!(exec_step.get_status(), ExecutableSimpleStatus::Succeeded);
        }
    }

    #[test]
    fn test_wrap() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let addr = UniversalAddress::Ethereum(EthAddress {
            0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
        });
        let exec_step = ExecutionStep::new(ExecutionStepEnum::EthWrap(EthWrapStep {
            uuid: Uuid::new([0u8; 16]),
            chain: universal_chain_id_registry::MOONBEAM,
            amount: Some(1_000_000_000),
            common: CommonExecutionMeta {
                src_addr: addr.clone(),
                dest_addr: addr.clone(),
                gas_fee_native: 1_000_000_000,
                gas_fee_usd: 2_000_0000_000,
            },
            status: EthStepStatus::NotStarted,
        }));
        execute_eth_step(exec_step);
    }

    #[test]
    fn test_unwrap() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let addr = UniversalAddress::Ethereum(EthAddress {
            0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
        });
        let exec_step = ExecutionStep::new(ExecutionStepEnum::EthUnwrap(EthUnwrapStep {
            uuid: Uuid::new([0u8; 16]),
            chain: universal_chain_id_registry::MOONBEAM,
            amount: Some(1_000_000_000),
            common: CommonExecutionMeta {
                src_addr: addr.clone(),
                dest_addr: addr.clone(),
                gas_fee_native: 1_000_000_000,
                gas_fee_usd: 2_000_0000_000,
            },
            status: EthStepStatus::NotStarted,
        }));
        execute_eth_step(exec_step);
    }

    #[test]
    fn test_eth_send() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let addr = UniversalAddress::Ethereum(EthAddress {
            0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
        });
        let exec_step = ExecutionStep::new(ExecutionStepEnum::EthSend(EthSendStep {
            uuid: Uuid::new([0u8; 16]),
            chain: universal_chain_id_registry::MOONBEAM,
            amount: Some(1_000_000_000),
            common: CommonExecutionMeta {
                src_addr: addr.clone(),
                dest_addr: addr.clone(),
                gas_fee_native: 1_000_000_000,
                gas_fee_usd: 2_000_0000_000,
            },
            status: EthStepStatus::NotStarted,
        }));
        execute_eth_step(exec_step);
    }

    #[test]
    fn test_erc20_transfer() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let addr = UniversalAddress::Ethereum(EthAddress {
            0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
        });
        let exec_step = ExecutionStep::new(ExecutionStepEnum::ERC20Transfer(ERC20TransferStep {
            uuid: Uuid::new([0u8; 16]),
            token: universal_token_id_registry::DOT_MOONBEAM,
            amount: Some(1_000_000_000),
            common: CommonExecutionMeta {
                src_addr: addr.clone(),
                dest_addr: addr.clone(),
                gas_fee_native: 1_000_000_000,
                gas_fee_usd: 2_000_0000_000,
            },
            status: EthStepStatus::NotStarted,
        }));
        execute_eth_step(exec_step);
    }

    #[test]
    fn test_dex_swap() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let addr = UniversalAddress::Ethereum(EthAddress {
            0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
        });
        let chain_info = chain_info_registry::MOONBEAM_INFO;
        let dex = dex_registry::STELLASWAP;
        let exec_step = ExecutionStep::new(ExecutionStepEnum::EthDexSwap(EthDexSwapStep {
            uuid: Uuid::new([0u8; 16]),
            dex_router_addr: dex.eth_dex_router,
            dex_router_func: DexRouterFunction::SwapExactTokensForTokens,
            token_path: vec![
                universal_token_id_registry::DOT_MOONBEAM,
                UniversalTokenId {
                    chain: universal_chain_id_registry::MOONBEAM,
                    id: ChainTokenId::ERC20(ERC20Token {
                        addr: chain_info.weth_addr.expect("WGLMR exists"),
                    }),
                },
                UniversalTokenId {
                    chain: universal_chain_id_registry::MOONBEAM,
                    id: ChainTokenId::ERC20(ERC20Token {
                        addr: EthAddress {
                            0: hex!("931715fee2d06333043d11f658c8ce934ac61d0c"), // USDC
                        },
                    }),
                },
            ],
            amount_in: Some(1_000_000_000),
            common: CommonExecutionMeta {
                src_addr: addr.clone(),
                dest_addr: addr.clone(),
                gas_fee_native: 1_000_000_000,
                gas_fee_usd: 2_000_0000_000,
            },
            status: EthStepStatus::NotStarted,
        }));
        execute_eth_step(exec_step);
    }

    #[cfg(feature = "private-rpc-endpoint")]
    #[test]
    fn test_xcm_transfer() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let src_addr = UniversalAddress::Ethereum(EthAddress {
            0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
        });
        let dest_addr = UniversalAddress::Substrate(SubstratePublicKey {
            0: hex!("7011b670bb662eedbd60a1c4c11b7c197ec22e7cfe87df00013ca2c494f3b01a"),
        });
        let bridge = &XCM_BRIDGES[7];
        let exec_step = ExecutionStep::new(ExecutionStepEnum::XCMTransfer(XCMTransferStep {
            uuid: Uuid::new([0u8; 16]),
            src_token: universal_token_id_registry::DOT_MOONBEAM,
            dest_token: universal_token_id_registry::DOT_NATIVE,
            token_asset_multilocation: bridge.token_asset_multilocation.clone(),
            full_dest_multilocation: bridge
                .dest_multilocation_template
                .get_full_dest_multilocation(dest_addr.clone())
                .expect("Wallet template should generate MultiLocation"),
            amount_in: Some(1_000_000_000),
            bridge_fee_native: 100_000_000,
            bridge_fee_usd: 1_000_000_000_000_000,
            common: CommonExecutionMeta {
                src_addr: src_addr.clone(),
                dest_addr: dest_addr.clone(),
                gas_fee_native: 1_000_000_000,
                gas_fee_usd: 2_000_0000_000,
            },
            status: CrossChainStepStatus::NotStarted,
        }));
        // 2 internal steps happen in step 2: The state changes from Submitted to LocalConfirmed
        // and then to Confirmed
        execute_eth_step(exec_step);
    }
}
