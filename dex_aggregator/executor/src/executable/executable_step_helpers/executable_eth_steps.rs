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

use duplicate::duplicate_item;
use ink_prelude::vec::Vec;

use pink_web3::types::SignedTransaction;
use privadex_chain_metadata::{
    chain_info::ChainInfo,
    common::{
        Amount, BlockNum, ChainTokenId, EthAddress, EthTxnHash, Nonce, UniversalAddress,
        UniversalChainId,
    },
    get_chain_info_from_chain_id,
};
use privadex_common::uuid::Uuid;
use privadex_execution_plan::execution_plan::{
    DexRouterFunction, ERC20TransferStep, EthDexSwapStep, EthPendingTxnId, EthSendStep,
    EthStepStatus, EthUnwrapStep, EthWrapStep,
};

use crate::{
    eth_utils,
    executable::{
        executable_step::{get_updated_gas_fee_usd, TXN_NUM_BLOCKS_ALIVE},
        execute_step_meta::ExecuteStepMeta,
        traits::{
            Executable, ExecutableError, ExecutableResult, ExecutableSimpleStatus,
            StepForwardResult,
        },
    },
    key_container::KeyContainer,
};

// The DEX swap deadline is this many millis in the future i.e. the txn will fail
// if it is included in a block after 8 minutes
const DEX_SWAP_LIFE_MILLIS: u64 = 480_000;

#[duplicate_item(
	exec_step;
	[EthSendStep];
	[ERC20TransferStep];
    [EthUnwrapStep];
    [EthWrapStep];
    [EthDexSwapStep];
)]
impl Executable for exec_step {
    fn get_status(&self) -> ExecutableSimpleStatus {
        (&self.status).into()
    }

    fn get_total_fee_usd(&self) -> Option<Amount> {
        if self.get_status() == ExecutableSimpleStatus::Succeeded {
            Some(self.common.gas_fee_usd)
        } else {
            None
        }
    }

    fn execute_step_forward(
        &mut self,
        execute_step_meta: &ExecuteStepMeta,
        keys: &KeyContainer,
    ) -> ExecutableResult<StepForwardResult> {
        let (opt_new_status, opt_actual_gas_fee_native, opt_amount_out) = match self.status {
            EthStepStatus::Confirmed(_) | EthStepStatus::Failed(_) | EthStepStatus::Dropped => {
                Err(ExecutableError::CalledStepForwardOnFinishedStep)
            }
            EthStepStatus::NotStarted => {
                let new_status =
                    self.execute_step_forward_if_notstarted(execute_step_meta, keys)?;
                Ok((Some(new_status), None, None))
            }
            EthStepStatus::Submitted(EthPendingTxnId {
                txn_hash,
                end_block_num,
            }) => {
                let res = self.execute_step_forward_if_inprogress(txn_hash, end_block_num)?;
                if let Some(completed_step_result) = res {
                    Ok((
                        Some(completed_step_result.new_status),
                        Some(completed_step_result.actual_gas_fee_native),
                        Some(completed_step_result.amount_out),
                    ))
                } else {
                    Ok((None, None, None))
                }
            }
        }?;
        let did_status_change = opt_new_status.is_some();
        if let Some(new_status) = opt_new_status {
            self.status = new_status;
        }
        if let Some(updated_gas_fee_native) = opt_actual_gas_fee_native {
            self.common.gas_fee_usd = get_updated_gas_fee_usd(
                updated_gas_fee_native,
                self.common.gas_fee_native,
                self.common.gas_fee_usd,
            );
            self.common.gas_fee_native = updated_gas_fee_native;
        }
        Ok(StepForwardResult {
            did_status_change,
            amount_out: opt_amount_out,
        })
    }
}

// Returned data from a failed or confirmed step
struct CompletedStepResult {
    pub new_status: EthStepStatus,
    pub actual_gas_fee_native: Amount,
    pub amount_out: Amount,
}

trait EthExecutableHelper {
    // Ok(new status, Some(updated gas fee)) if the step was updated and the gas
    //   fee was updated e.g. txn was dropped, or
    // Ok(new status, None) if the step was updated and the gas fee was not updated, or
    // Err(_) if we encountered an error
    fn execute_step_forward_if_notstarted(
        &self,
        execute_step_meta: &ExecuteStepMeta,
        keys: &KeyContainer,
    ) -> ExecutableResult<EthStepStatus /* new status */> {
        let chain_info = get_chain_info_from_chain_id(&self.get_chain())
            .ok_or(ExecutableError::FailedToFindChainInfo)?;
        let cur_block = eth_utils::common::block_number(chain_info.rpc_url)
            .map_err(|_| ExecutableError::RpcRequestFailed)?;

        // Using NonceManager to get the nonce in a concurrent-safe way
        let nonce = {
            let system_nonce = {
                if let UniversalAddress::Ethereum(src_addr) = self.src_addr() {
                    eth_utils::common::get_next_system_nonce(chain_info.rpc_url, src_addr.clone())
                        .map_err(|_| ExecutableError::RpcRequestFailed)
                } else {
                    Err(ExecutableError::UnexpectedNonEthAddress)
                }
            }?;
            execute_step_meta.get_nonce(
                self.get_exec_step_uuid(),
                self.get_chain(),
                cur_block,
                system_nonce,
            )
        }?;
        let signed_txn = self.create_raw_txn(execute_step_meta, keys, chain_info, nonce)?;

        let txn_hash = self.send_raw_txn(chain_info.rpc_url, signed_txn)?;

        Ok(EthStepStatus::Submitted(EthPendingTxnId {
            txn_hash,
            end_block_num: cur_block + TXN_NUM_BLOCKS_ALIVE,
        }))
    }

    // Ok(Some(_)) if the step was completed (failed or confirmed or dropped), or
    // Ok(None) if the step was not completed, or
    // Err(_) if we encountered an error
    fn execute_step_forward_if_inprogress(
        &self,
        txn_hash: EthTxnHash,
        end_block_num: BlockNum,
    ) -> ExecutableResult<Option<CompletedStepResult>> {
        let chain_info = get_chain_info_from_chain_id(&self.get_chain())
            .ok_or(ExecutableError::FailedToFindChainInfo)?;
        let cur_block = eth_utils::common::block_number(chain_info.rpc_url)
            .map_err(|_| ExecutableError::RpcRequestFailed)?;

        if cur_block > end_block_num {
            Ok(Some(CompletedStepResult {
                new_status: EthStepStatus::Dropped,
                actual_gas_fee_native: 0,
                amount_out: 0,
            }))
        } else {
            Ok(self.get_completed_step_result(chain_info.rpc_url, txn_hash))
        }
    }

    fn create_raw_txn(
        &self,
        execute_step_meta: &ExecuteStepMeta,
        keys: &KeyContainer,
        chain_info: &ChainInfo,
        nonce: Nonce,
    ) -> ExecutableResult<SignedTransaction>;

    fn send_raw_txn(
        &self,
        rpc_url: &str,
        raw_txn: SignedTransaction,
    ) -> ExecutableResult<EthTxnHash> {
        eth_utils::common::send_raw_transaction(rpc_url, raw_txn)
            .map_err(|_| ExecutableError::RpcRequestFailed)
    }

    fn get_completed_step_result(
        &self,
        rpc_url: &str,
        txn_hash: EthTxnHash,
    ) -> Option<CompletedStepResult>;

    fn src_addr(&self) -> &UniversalAddress;

    fn get_chain(&self) -> UniversalChainId;

    fn get_exec_step_uuid(&self) -> &Uuid;
}

impl EthExecutableHelper for EthSendStep {
    fn create_raw_txn(
        &self,
        _execute_step_meta: &ExecuteStepMeta,
        keys: &KeyContainer,
        chain_info: &ChainInfo,
        nonce: Nonce,
    ) -> ExecutableResult<SignedTransaction> {
        let to_addr = {
            if let UniversalAddress::Ethereum(eth_addr) = self.common.dest_addr.clone() {
                Ok(eth_addr)
            } else {
                Err(ExecutableError::UnexpectedNonEthAddress)
            }
        }?;
        let amount = self.amount.ok_or(ExecutableError::UnexpectedNullAmount)?;
        let key = keys
            .get_key(self.src_addr())
            .ok_or(ExecutableError::SecretNotFound)?;
        let evm_chain_id = chain_info
            .evm_chain_id
            .ok_or(ExecutableError::UnexpectedNullEvmChainId)?;

        eth_utils::common::create_send_eth_raw_txn(
            chain_info.rpc_url,
            to_addr,
            amount,
            key,
            evm_chain_id,
            nonce,
        )
        .map_err(|_| ExecutableError::FailedToCreateTxn)
    }

    fn get_completed_step_result(
        &self,
        rpc_url: &str,
        txn_hash: EthTxnHash,
    ) -> Option<CompletedStepResult> {
        helpers::verified_get_completed_step_result_for_eth_transfer(
            rpc_url,
            txn_hash,
            self.amount
                .expect("Should have checked for erroneously null amount in create_raw_txn"),
        )
    }

    fn src_addr(&self) -> &UniversalAddress {
        &self.common.src_addr
    }

    fn get_chain(&self) -> UniversalChainId {
        self.chain
    }

    fn get_exec_step_uuid(&self) -> &Uuid {
        &self.uuid
    }
}

impl EthExecutableHelper for ERC20TransferStep {
    fn create_raw_txn(
        &self,
        _execute_step_meta: &ExecuteStepMeta,
        keys: &KeyContainer,
        chain_info: &ChainInfo,
        nonce: Nonce,
    ) -> ExecutableResult<SignedTransaction> {
        let to_addr = {
            if let UniversalAddress::Ethereum(eth_addr) = self.common.dest_addr.clone() {
                Ok(eth_addr)
            } else {
                Err(ExecutableError::UnexpectedNonEthAddress)
            }
        }?;
        let amount = self.amount.ok_or(ExecutableError::UnexpectedNullAmount)?;
        let key = keys
            .get_key(self.src_addr())
            .ok_or(ExecutableError::SecretNotFound)?;

        let token_eth_addr = {
            match &self.token.id {
                ChainTokenId::Native => Err(ExecutableError::UnexpectedNonEthAddress),
                ChainTokenId::ERC20(erc20_token) => Ok(erc20_token.addr),
                ChainTokenId::XC20(xc20_token) => Ok(xc20_token.get_eth_address()),
            }
        }?;

        let erc20_contract =
            eth_utils::erc20_contract::ERC20Contract::new(chain_info.rpc_url, token_eth_addr)
                .map_err(|_| ExecutableError::FailedToLoadWethContract)?;
        erc20_contract
            .transfer(to_addr, amount, key, nonce)
            .map_err(|_| ExecutableError::FailedToCreateTxn)
    }

    fn get_completed_step_result(
        &self,
        rpc_url: &str,
        txn_hash: EthTxnHash,
    ) -> Option<CompletedStepResult> {
        let token_addr = match &self.token.id {
            ChainTokenId::ERC20(erc20_token) => erc20_token.addr.clone(),
            ChainTokenId::XC20(xc20_token) => xc20_token.get_eth_address().clone(),
            _ => panic!(
                "Expected ERC20-compatible token in ERC20TransferStep get_completed_step_result"
            ),
        };
        helpers::verified_get_completed_step_result_for_erc20_transfer(
            rpc_url,
            txn_hash,
            &token_addr,
            self.amount
                .expect("Should have checked for erroneously null amount in create_raw_txn"),
        )
    }

    fn src_addr(&self) -> &UniversalAddress {
        &self.common.src_addr
    }

    fn get_chain(&self) -> UniversalChainId {
        self.token.chain
    }

    fn get_exec_step_uuid(&self) -> &Uuid {
        &self.uuid
    }
}

impl EthExecutableHelper for EthWrapStep {
    fn create_raw_txn(
        &self,
        _execute_step_meta: &ExecuteStepMeta,
        keys: &KeyContainer,
        chain_info: &ChainInfo,
        nonce: Nonce,
    ) -> ExecutableResult<SignedTransaction> {
        let amount = self.amount.ok_or(ExecutableError::UnexpectedNullAmount)?;
        let key = keys
            .get_key(self.src_addr())
            .ok_or(ExecutableError::SecretNotFound)?;

        let weth_contract = eth_utils::weth_contract::WethContract::new(
            chain_info.rpc_url,
            chain_info
                .weth_addr
                .ok_or(ExecutableError::FailedToLoadWethContract)?,
        )
        .map_err(|_| ExecutableError::FailedToLoadWethContract)?;
        weth_contract
            .deposit(amount, key, nonce)
            .map_err(|_| ExecutableError::FailedToCreateTxn)
    }

    fn get_completed_step_result(
        &self,
        rpc_url: &str,
        txn_hash: EthTxnHash,
    ) -> Option<CompletedStepResult> {
        helpers::get_completed_step_result_for_known_amount(
            rpc_url,
            txn_hash,
            self.amount
                .expect("Should have checked for erroneously null amount in create_raw_txn"),
        )
    }

    fn src_addr(&self) -> &UniversalAddress {
        &self.common.src_addr
    }

    fn get_chain(&self) -> UniversalChainId {
        self.chain
    }

    fn get_exec_step_uuid(&self) -> &Uuid {
        &self.uuid
    }
}

impl EthExecutableHelper for EthUnwrapStep {
    fn create_raw_txn(
        &self,
        _execute_step_meta: &ExecuteStepMeta,
        keys: &KeyContainer,
        chain_info: &ChainInfo,
        nonce: Nonce,
    ) -> ExecutableResult<SignedTransaction> {
        let amount = self.amount.ok_or(ExecutableError::UnexpectedNullAmount)?;
        let key = keys
            .get_key(self.src_addr())
            .ok_or(ExecutableError::SecretNotFound)?;

        let weth_contract = eth_utils::weth_contract::WethContract::new(
            chain_info.rpc_url,
            chain_info
                .weth_addr
                .ok_or(ExecutableError::FailedToLoadWethContract)?,
        )
        .map_err(|_| ExecutableError::FailedToLoadWethContract)?;
        weth_contract
            .withdraw(amount, key, nonce)
            .map_err(|_| ExecutableError::FailedToCreateTxn)
    }

    fn get_completed_step_result(
        &self,
        rpc_url: &str,
        txn_hash: EthTxnHash,
    ) -> Option<CompletedStepResult> {
        helpers::get_completed_step_result_for_known_amount(
            rpc_url,
            txn_hash,
            self.amount
                .expect("Should have checked for erroneously null amount in create_raw_txn"),
        )
    }

    fn src_addr(&self) -> &UniversalAddress {
        &self.common.src_addr
    }

    fn get_chain(&self) -> UniversalChainId {
        self.chain
    }

    fn get_exec_step_uuid(&self) -> &Uuid {
        &self.uuid
    }
}

impl EthExecutableHelper for EthDexSwapStep {
    fn create_raw_txn(
        &self,
        execute_step_meta: &ExecuteStepMeta,
        keys: &KeyContainer,
        chain_info: &ChainInfo,
        nonce: Nonce,
    ) -> ExecutableResult<SignedTransaction> {
        let amount_in = self
            .amount_in
            .ok_or(ExecutableError::UnexpectedNullAmount)?;
        // TODO_lowpriority: We should definitely add a 'limit price' in the future,
        // but doing so means we need to handle failed transactions if the limit
        // price is exceeded. For simplicity, we exclude this feature in the MVP.
        let amount_out_min = 0;
        let path = {
            let swap_path: Result<Vec<EthAddress>, ExecutableError> = self
                .token_path
                .iter()
                .map(|universal_token_id| match &universal_token_id.id {
                    ChainTokenId::Native => Err(ExecutableError::UnexpectedNonEthAddress),
                    ChainTokenId::ERC20(erc20_token) => Ok(erc20_token.addr),
                    ChainTokenId::XC20(xc20_token) => Ok(xc20_token.get_eth_address()),
                })
                .collect();
            swap_path?
        };
        let to_addr = {
            if let UniversalAddress::Ethereum(eth_addr) = self.common.dest_addr.clone() {
                Ok(eth_addr)
            } else {
                Err(ExecutableError::UnexpectedNonEthAddress)
            }
        }?;
        let deadline = {
            if execute_step_meta.cur_timestamp() > u64::MAX - DEX_SWAP_LIFE_MILLIS {
                u64::MAX
            } else {
                execute_step_meta.cur_timestamp() + DEX_SWAP_LIFE_MILLIS
            }
        };
        let key = keys
            .get_key(self.src_addr())
            .ok_or(ExecutableError::SecretNotFound)?;

        let dex_router_contract = eth_utils::dex_router_contract::DEXRouterContract::new(
            chain_info.rpc_url,
            self.dex_router_addr,
        )
        .map_err(|_| ExecutableError::FailedToLoadWethContract)?;
        let router_func = match self.dex_router_func {
            DexRouterFunction::SwapExactETHForTokens => {
                eth_utils::dex_router_contract::DEXRouterContract::swap_exact_eth_for_tokens
            }
            DexRouterFunction::SwapExactTokensForETH => {
                eth_utils::dex_router_contract::DEXRouterContract::swap_exact_tokens_for_eth
            }
            DexRouterFunction::SwapExactTokensForTokens => {
                eth_utils::dex_router_contract::DEXRouterContract::swap_exact_tokens_for_tokens
            }
        };
        router_func(
            &dex_router_contract,
            amount_in,
            amount_out_min,
            path,
            to_addr,
            deadline,
            key,
            nonce,
        )
        .map_err(|_| ExecutableError::FailedToCreateTxn)
    }

    fn get_completed_step_result(
        &self,
        rpc_url: &str,
        txn_hash: EthTxnHash,
    ) -> Option<CompletedStepResult> {
        let parse_response =
            eth_utils::parse_txn_helper::parse_transfer_from_dex_swap_txn(rpc_url, txn_hash);
        if let Ok(erc20_transfer) = parse_response {
            if erc20_transfer.is_txn_success {
                Some(CompletedStepResult {
                    new_status: EthStepStatus::Confirmed(txn_hash),
                    actual_gas_fee_native: erc20_transfer.gas_fee_native,
                    amount_out: erc20_transfer.amount,
                })
            } else {
                Some(CompletedStepResult {
                    new_status: EthStepStatus::Failed(txn_hash),
                    actual_gas_fee_native: erc20_transfer.gas_fee_native,
                    amount_out: 0,
                })
            }
        } else {
            None
        }
    }

    fn src_addr(&self) -> &UniversalAddress {
        &self.common.src_addr
    }

    fn get_chain(&self) -> UniversalChainId {
        self.token_path[0].chain // token path must be non-empty
    }

    fn get_exec_step_uuid(&self) -> &Uuid {
        &self.uuid
    }
}

mod helpers {
    use super::*;

    // For ETH send, ERC20 transfer, we know that amount_out SHOULD be the same as amount_in but
    // we check anyway. This is important! For the prestart step, a user could otherwise cheat the
    // system by passing in a different value of amount_in (or different token ID) and sending a txn
    // of lower value (or of different token)
    pub(super) fn verified_get_completed_step_result_for_eth_transfer(
        rpc_url: &str,
        eth_send_txn: EthTxnHash,
        expected_amount: Amount,
    ) -> Option<CompletedStepResult> {
        if let Ok(eth_transfer) =
            eth_utils::parse_txn_helper::parse_transfer_from_eth_send_txn(rpc_url, eth_send_txn)
        {
            if is_eth_transfer_invalid(&eth_transfer, expected_amount) {
                ink_env::debug_println!("Unexpected! Amount received from Eth transfer ({}) does not match expected amount ({})",
                    eth_transfer.amount, expected_amount);
                // Treat this like a fail
                Some(CompletedStepResult {
                    new_status: EthStepStatus::Failed(eth_send_txn),
                    actual_gas_fee_native: eth_transfer.gas_fee_native,
                    amount_out: 0,
                })
            } else {
                if eth_transfer.is_txn_success {
                    Some(CompletedStepResult {
                        new_status: EthStepStatus::Confirmed(eth_send_txn),
                        actual_gas_fee_native: eth_transfer.gas_fee_native,
                        amount_out: expected_amount,
                    })
                } else {
                    Some(CompletedStepResult {
                        new_status: EthStepStatus::Failed(eth_send_txn),
                        actual_gas_fee_native: eth_transfer.gas_fee_native,
                        amount_out: 0,
                    })
                }
            }
        } else {
            None
        }
    }

    #[cfg(not(feature = "mock-txn-send"))]
    fn is_eth_transfer_invalid(
        eth_transfer: &eth_utils::common::EthTransfer,
        expected_amount: Amount,
    ) -> bool {
        eth_transfer.amount != expected_amount
    }

    #[cfg(feature = "mock-txn-send")]
    fn is_eth_transfer_invalid(
        eth_transfer: &eth_utils::common::EthTransfer,
        expected_amount: Amount,
    ) -> bool {
        false
    }

    pub(super) fn verified_get_completed_step_result_for_erc20_transfer(
        rpc_url: &str,
        erc20_txn_hash: EthTxnHash,
        expected_token: &EthAddress,
        expected_amount: Amount,
    ) -> Option<CompletedStepResult> {
        if let Ok(erc20_transfer) =
            eth_utils::parse_txn_helper::parse_transfer_from_erc20_txn(rpc_url, erc20_txn_hash)
        {
            if is_erc20_transfer_invalid(&erc20_transfer, expected_token, expected_amount) {
                ink_env::debug_println!("Unexpected! Amount/token received from Eth transfer ({} {:?}) does not match expected amount ({} {:?})",
                    erc20_transfer.amount, erc20_transfer.token, expected_amount, expected_token);
                // Treat this like a fail
                Some(CompletedStepResult {
                    new_status: EthStepStatus::Failed(erc20_txn_hash),
                    actual_gas_fee_native: erc20_transfer.gas_fee_native,
                    amount_out: 0,
                })
            } else {
                if erc20_transfer.is_txn_success {
                    Some(CompletedStepResult {
                        new_status: EthStepStatus::Confirmed(erc20_txn_hash),
                        actual_gas_fee_native: erc20_transfer.gas_fee_native,
                        amount_out: expected_amount,
                    })
                } else {
                    Some(CompletedStepResult {
                        new_status: EthStepStatus::Failed(erc20_txn_hash),
                        actual_gas_fee_native: erc20_transfer.gas_fee_native,
                        amount_out: 0,
                    })
                }
            }
        } else {
            None
        }
    }

    #[cfg(not(feature = "mock-txn-send"))]
    fn is_erc20_transfer_invalid(
        erc20_transfer: &eth_utils::common::ERC20Transfer,
        expected_token: &EthAddress,
        expected_amount: Amount,
    ) -> bool {
        erc20_transfer.amount != expected_amount || erc20_transfer.token != *expected_token
    }

    #[cfg(feature = "mock-txn-send")]
    fn is_erc20_transfer_invalid(
        erc20_transfer: &eth_utils::common::ERC20Transfer,
        expected_token: &EthAddress,
        expected_amount: Amount,
    ) -> bool {
        false
    }

    // For wrap, unwrap:
    // we know that amount_out == amount_in (by definition),
    // so we do not need to parse out the amount_out
    pub(super) fn get_completed_step_result_for_known_amount(
        rpc_url: &str,
        txn_hash: EthTxnHash,
        amount: Amount,
    ) -> Option<CompletedStepResult> {
        if let Ok(txn_summary) = eth_utils::parse_txn_helper::get_txn_summary(rpc_url, txn_hash) {
            if txn_summary.is_txn_success {
                Some(CompletedStepResult {
                    new_status: EthStepStatus::Confirmed(txn_hash),
                    actual_gas_fee_native: txn_summary.gas_fee_native,
                    amount_out: amount,
                })
            } else {
                Some(CompletedStepResult {
                    new_status: EthStepStatus::Failed(txn_hash),
                    actual_gas_fee_native: txn_summary.gas_fee_native,
                    amount_out: 0,
                })
            }
        } else {
            None
        }
    }
}
