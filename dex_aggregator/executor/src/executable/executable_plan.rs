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
use privadex_execution_plan::execution_plan::{ExecutionPath, ExecutionPlan};

use crate::key_container::KeyContainer;

use super::{
    execute_step_meta::ExecuteStepMeta,
    traits::{
        Executable, ExecutableError, ExecutableResult, ExecutableSimpleStatus, StepForwardResult,
    },
};

impl Executable for ExecutionPlan {
    fn get_status(&self) -> ExecutableSimpleStatus {
        if self.prestart_user_to_escrow_transfer.get_status() == ExecutableSimpleStatus::NotStarted
        {
            ExecutableSimpleStatus::NotStarted
        } else if self.postend_escrow_to_user_transfer.get_status()
            == ExecutableSimpleStatus::Succeeded
        {
            ExecutableSimpleStatus::Succeeded
        } else if self.prestart_user_to_escrow_transfer.get_status()
            == ExecutableSimpleStatus::Dropped
            || self.postend_escrow_to_user_transfer.get_status() == ExecutableSimpleStatus::Dropped
            || self
                .paths
                .iter()
                .any(|path| path.get_status() == ExecutableSimpleStatus::Dropped)
        {
            ExecutableSimpleStatus::Dropped
        } else if self.prestart_user_to_escrow_transfer.get_status()
            == ExecutableSimpleStatus::Failed
            || self.postend_escrow_to_user_transfer.get_status() == ExecutableSimpleStatus::Failed
            || self
                .paths
                .iter()
                .any(|path| path.get_status() == ExecutableSimpleStatus::Failed)
        {
            ExecutableSimpleStatus::Failed
        } else {
            ExecutableSimpleStatus::InProgress
        }
    }

    fn get_total_fee_usd(&self) -> Option<Amount> {
        // We want to return a value when all the subpaths are completed i.e.
        // it is fine if the postend step is not yet complete!
        if have_all_exec_paths_succeeded(self) {
            Some(
                self.paths.iter().fold(0, |fees_usd, path| {
                    fees_usd + path.get_total_fee_usd().unwrap_or(0)
                }) + self
                    .postend_escrow_to_user_transfer
                    .get_total_fee_usd()
                    .unwrap_or(0),
            )
        } else {
            None
        }
    }

    // The caller (worker) should save to S3 and unallocate itself from the ExecutionPlan
    fn execute_step_forward(
        &mut self,
        execute_step_meta: &ExecuteStepMeta,
        keys: &KeyContainer,
    ) -> ExecutableResult<StepForwardResult> {
        let status = self.get_status();
        if status == ExecutableSimpleStatus::Dropped
            || status == ExecutableSimpleStatus::Failed
            || status == ExecutableSimpleStatus::Succeeded
        {
            return Err(ExecutableError::CalledStepForwardOnFinishedPlan);
        }
        let (mut did_plan_status_change, should_process_paths) =
            match self.prestart_user_to_escrow_transfer.get_status() {
                ExecutableSimpleStatus::NotStarted => Err(ExecutableError::PrestartStepNotStarted),
                // The Failed check above captures this below state
                ExecutableSimpleStatus::Failed | ExecutableSimpleStatus::Dropped => {
                    Err(ExecutableError::UnknownBadState)
                }
                ExecutableSimpleStatus::InProgress => {
                    let prestart_step_result = self
                        .prestart_user_to_escrow_transfer
                        .execute_step_forward(execute_step_meta, keys)?;
                    Ok((
                        prestart_step_result.did_status_change,
                        self.prestart_user_to_escrow_transfer.get_status()
                            == ExecutableSimpleStatus::Succeeded,
                    ))
                }
                ExecutableSimpleStatus::Succeeded => Ok((false, true)),
            }?;
        if !should_process_paths {
            Ok(StepForwardResult {
                did_status_change: did_plan_status_change,
                amount_out: None,
            })
        } else if !have_all_exec_paths_succeeded(self) {
            for exec_path in self.paths.iter_mut() {
                if exec_path.get_status() == ExecutableSimpleStatus::NotStarted
                    || exec_path.get_status() == ExecutableSimpleStatus::InProgress
                {
                    let StepForwardResult {
                        did_status_change: did_path_status_change,
                        amount_out: _,
                    } = exec_path.execute_step_forward(execute_step_meta, keys)?;
                    did_plan_status_change = did_plan_status_change | did_path_status_change;
                }
                if exec_path.get_status() == ExecutableSimpleStatus::Dropped
                    || exec_path.get_status() == ExecutableSimpleStatus::Failed
                {
                    // Stop processing other paths and exit early if any have failed
                    break;
                }
            }
            Ok(StepForwardResult {
                did_status_change: did_plan_status_change,
                amount_out: None,
            })
        } else {
            let total_amount = sum_exec_paths_amounts_out(&self.paths);
            let amount_in_after_fee = calc_amount_after_simple_fee(total_amount);
            self.postend_escrow_to_user_transfer
                .set_amount_in(amount_in_after_fee);
            let postend_res = self
                .postend_escrow_to_user_transfer
                .execute_step_forward(execute_step_meta, keys)?;
            did_plan_status_change = did_plan_status_change | postend_res.did_status_change;
            Ok(StepForwardResult {
                did_status_change: did_plan_status_change,
                // We only set amount_out when exec plan succeeds
                amount_out: postend_res.amount_out,
            })
        }
    }
}

fn have_all_exec_paths_succeeded(exec_plan: &ExecutionPlan) -> bool {
    exec_plan
        .paths
        .iter()
        .all(|path| path.get_status() == ExecutableSimpleStatus::Succeeded)
}

fn sum_exec_paths_amounts_out(exec_paths: &[ExecutionPath]) -> Amount {
    exec_paths.iter().fold(0, |amount_out, exec_path| {
        // All the amount outs should be non-null!
        amount_out + exec_path.amount_out.unwrap_or(0)
    })
}

// TODO_lowpriority: Can make this fee as sophisticated as possible (e.g. depend on the
// complexity of the execution plan, etc.). Simple % fee for now.
fn calc_amount_after_simple_fee(amount_no_fee: Amount) -> Amount {
    // Simple 0.05% fee
    // TODO: This needs to account for gas fees before true go-live
    mul_ratio_u128(amount_no_fee, 9_995, 10_000)
}

// Prerequisites for these tests: You need to have sufficient funds in your account!
// These tests do not actually send out the transaction - we use conditional compilation
// to mock the transaction sending and transaction/extrinsic/event parsing. But the
// funds are needed in the estimate_gas step.
// Mock feature is critical! Otherwise you will send an actual transaction
#[cfg(feature = "mock-txn-send")]
#[cfg(test)]
mod executable_plan_tests {
    use core::str::FromStr;
    use hex_literal::hex;
    use ink_env::debug_println;
    use ink_prelude::{vec, vec::Vec};
    use privadex_chain_metadata::{
        common::{
            BlockNum, ChainTokenId, ERC20Token, EthAddress, EthTxnHash, SecretKeyContainer,
            SubstratePublicKey, UniversalAddress, UniversalChainId, UniversalTokenId,
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
        EthDexSwapStep, EthPendingTxnId, EthSendStep, EthStepStatus, EthUnwrapStep, EthWrapStep,
        ExecutionPath, ExecutionStep, ExecutionStepEnum, XCMTransferStep,
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

    #[test]
    fn simple_plan() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let (addr, execute_step_meta, keys) = dummy_state();
        let exec_path1 = ExecutionPath {
            steps: vec![
                ExecutionStep::new(ExecutionStepEnum::EthWrap(EthWrapStep {
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
                })),
                ExecutionStep::new(ExecutionStepEnum::EthUnwrap(EthUnwrapStep {
                    uuid: Uuid::new([0u8; 16]),
                    chain: universal_chain_id_registry::MOONBEAM,
                    amount: None,
                    common: CommonExecutionMeta {
                        src_addr: addr.clone(),
                        dest_addr: addr.clone(),
                        gas_fee_native: 1_000_000_000,
                        gas_fee_usd: 2_000_0000_000,
                    },
                    status: EthStepStatus::NotStarted,
                })),
                ExecutionStep::new(ExecutionStepEnum::EthSend(EthSendStep {
                    uuid: Uuid::new([0u8; 16]),
                    chain: universal_chain_id_registry::MOONBEAM,
                    amount: None,
                    common: CommonExecutionMeta {
                        src_addr: addr.clone(),
                        dest_addr: addr.clone(),
                        gas_fee_native: 1_000_000_000,
                        gas_fee_usd: 2_000_0000_000,
                    },
                    status: EthStepStatus::NotStarted,
                })),
            ],
            amount_out: None,
        };
        let exec_path2 = ExecutionPath {
            steps: vec![
                ExecutionStep::new(ExecutionStepEnum::EthUnwrap(EthUnwrapStep {
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
                })),
                ExecutionStep::new(ExecutionStepEnum::EthSend(EthSendStep {
                    uuid: Uuid::new([0u8; 16]),
                    chain: universal_chain_id_registry::MOONBEAM,
                    amount: None,
                    common: CommonExecutionMeta {
                        src_addr: addr.clone(),
                        dest_addr: addr.clone(),
                        gas_fee_native: 1_000_000_000,
                        gas_fee_usd: 2_000_0000_000,
                    },
                    status: EthStepStatus::NotStarted,
                })),
            ],
            amount_out: None,
        };
        let mut exec_plan = ExecutionPlan {
            uuid: Uuid::new([0u8; 16]),
            paths: vec![exec_path1, exec_path2],
            prestart_user_to_escrow_transfer: ExecutionStep::new(ExecutionStepEnum::EthSend(
                EthSendStep {
                    uuid: Uuid::new([0u8; 16]),
                    chain: universal_chain_id_registry::MOONBEAM,
                    amount: Some(1_000_000_000),
                    common: CommonExecutionMeta {
                        src_addr: addr.clone(),
                        dest_addr: addr.clone(),
                        gas_fee_native: 1_000_000_000,
                        gas_fee_usd: 2_000_0000_000,
                    },
                    status: EthStepStatus::Submitted(EthPendingTxnId {
                        txn_hash: EthTxnHash::zero(),
                        end_block_num: BlockNum::MAX,
                    }),
                },
            )),
            postend_escrow_to_user_transfer: ExecutionStep::new(ExecutionStepEnum::EthSend(
                EthSendStep {
                    uuid: Uuid::new([0u8; 16]),
                    chain: universal_chain_id_registry::MOONBEAM,
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

        // Prestart step is in progress
        assert_eq!(exec_plan.get_status(), ExecutableSimpleStatus::InProgress);
        assert_eq!(exec_plan.get_total_fee_usd(), None);

        while exec_plan.get_status() == ExecutableSimpleStatus::NotStarted
            || exec_plan.get_status() == ExecutableSimpleStatus::InProgress
        {
            if !have_all_exec_paths_succeeded(&exec_plan) {
                assert_eq!(exec_plan.get_total_fee_usd(), None);
            } else {
                debug_println!("All exec paths finished. Postend step remaining...")
            }
            let res = exec_plan
                .execute_step_forward(&execute_step_meta, &keys)
                .expect("Step should succeed");
            debug_println!("Step forward result: {:?}", res);
            debug_println!("State: {:?}, {}\n", exec_plan.get_status(), exec_plan);
        }
        // assert_eq!(exec_plan.get_status(), ExecutableSimpleStatus::Failed);
        // assert!(exec_plan.get_total_fee_usd().is_none());

        assert_eq!(exec_plan.get_status(), ExecutableSimpleStatus::Succeeded);
        assert!(exec_plan.get_total_fee_usd().is_some());
    }
}
