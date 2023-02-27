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
use privadex_execution_plan::execution_plan::ExecutionPath;

use crate::key_container::KeyContainer;

use super::{
    execute_step_meta::ExecuteStepMeta,
    traits::{
        Executable, ExecutableError, ExecutableResult, ExecutableSimpleStatus, StepForwardResult,
    },
};

impl Executable for ExecutionPath {
    fn get_status(&self) -> ExecutableSimpleStatus {
        let steps = &self.steps;
        if steps[0].get_status() == ExecutableSimpleStatus::NotStarted {
            ExecutableSimpleStatus::NotStarted
        } else if steps
            .last()
            .expect("Execution path has at least one execution step")
            .get_status()
            == ExecutableSimpleStatus::Succeeded
        {
            ExecutableSimpleStatus::Succeeded
        } else if steps
            .iter()
            .any(|s| s.get_status() == ExecutableSimpleStatus::Dropped)
        {
            ExecutableSimpleStatus::Dropped
        } else if steps
            .iter()
            .any(|s| s.get_status() == ExecutableSimpleStatus::Failed)
        {
            ExecutableSimpleStatus::Failed
        } else {
            ExecutableSimpleStatus::InProgress
        }
    }

    fn get_total_fee_usd(&self) -> Option<Amount> {
        if self.get_status() == ExecutableSimpleStatus::Succeeded {
            Some(self.steps.iter().fold(0, |fees_usd, step| {
                fees_usd + step.get_total_fee_usd().unwrap_or(0)
            }))
        } else {
            None
        }
    }

    // We choose to not modify self.amount_out here and instead do it in executable_plan.
    // The interface is a bit wonky
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
            return Err(ExecutableError::CalledStepForwardOnFinishedStep);
        }

        let (idx, step_to_process) = self
            .steps
            .iter_mut()
            .enumerate()
            .filter(|(_idx, step)| {
                let status = step.get_status();
                status == ExecutableSimpleStatus::NotStarted
                    || status == ExecutableSimpleStatus::InProgress
            })
            .next()
            .ok_or(ExecutableError::UnknownBadState)?; // should never hit this since status != succeeded

        let step_forward_res = step_to_process.execute_step_forward(execute_step_meta, keys)?;
        if let StepForwardResult {
            did_status_change: true,
            amount_out: Some(amount_out),
        } = step_forward_res
        {
            if step_to_process.get_status() == ExecutableSimpleStatus::Succeeded
                && idx < self.steps.len() - 1
            {
                // Propagate amount_out from one step to amount_in in the next
                let next_step = &mut self.steps[idx + 1];
                next_step.set_amount_in(amount_out);

                // We also call execute_step_forward on the next step. This isn't necessary
                // (the next invocation can do it), but it seems like an easy optimization
                // Ok(StepForwardResult {
                //     did_status_change: true,
                //     amount_out: None,
                // })
                if let StepForwardResult {
                    did_status_change: true,
                    amount_out: Some(amount_out2),
                } = next_step.execute_step_forward(execute_step_meta, keys)?
                {
                    // Realistically we never reach this because the next_next step will at best go to the
                    // InProgress
                    if next_step.get_status() == ExecutableSimpleStatus::Succeeded
                        && idx + 1 < self.steps.len() - 1
                    {
                        let next_next_step = &mut self.steps[idx + 2];
                        next_next_step.set_amount_in(amount_out2);
                        Ok(StepForwardResult {
                            did_status_change: true,
                            amount_out: None,
                        })
                    } else {
                        // We finished the last step in the path
                        self.amount_out = Some(amount_out2);
                        Ok(StepForwardResult {
                            did_status_change: true,
                            amount_out: Some(amount_out2),
                        })
                    }
                } else {
                    Ok(StepForwardResult {
                        did_status_change: true,
                        amount_out: None,
                    })
                }
            } else {
                // We finished the last step in the path
                self.amount_out = Some(amount_out);
                Ok(StepForwardResult {
                    did_status_change: true,
                    amount_out: Some(amount_out),
                })
            }
        } else {
            Ok(StepForwardResult {
                did_status_change: step_forward_res.did_status_change,
                amount_out: None,
            })
        }
    }
}

// Prerequisites for these tests: You need to have sufficient funds in your account!
// These tests do not actually send out the transaction - we use conditional compilation
// to mock the transaction sending and transaction/extrinsic/event parsing. But the
// funds are needed in the estimate_gas step.
// Mock feature is critical! Otherwise you will send an actual transaction
#[cfg(feature = "mock-txn-send")]
#[cfg(test)]
mod executable_path_tests {
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
        EthDexSwapStep, EthSendStep, EthStepStatus, EthUnwrapStep, EthWrapStep, ExecutionStep,
        ExecutionStepEnum, XCMTransferStep,
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
    fn simple_path() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let (addr, execute_step_meta, keys) = dummy_state();
        let mut exec_path = ExecutionPath {
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

        assert_eq!(exec_path.get_status(), ExecutableSimpleStatus::NotStarted);
        assert_eq!(exec_path.get_total_fee_usd(), None);

        while exec_path.get_status() == ExecutableSimpleStatus::NotStarted
            || exec_path.get_status() == ExecutableSimpleStatus::InProgress
        {
            assert_eq!(exec_path.amount_out, None);
            assert_eq!(exec_path.get_total_fee_usd(), None);
            let res = exec_path
                .execute_step_forward(&execute_step_meta, &keys)
                .expect("Step should succeed");
            debug_println!("Step forward result: {:?}", res);
            debug_println!("State: {:?}, {}\n", exec_path.get_status(), exec_path);
        }
        // assert_eq!(exec_path.get_status(), ExecutableSimpleStatus::Failed);
        // assert_eq!(exec_path.amount_out, Some(0));
        // assert!(exec_path.get_total_fee_usd().is_none());

        assert_eq!(exec_path.get_status(), ExecutableSimpleStatus::Succeeded);
        assert_eq!(exec_path.amount_out, Some(1_000_000_000));
        assert!(exec_path.get_total_fee_usd().is_some());
    }
}
