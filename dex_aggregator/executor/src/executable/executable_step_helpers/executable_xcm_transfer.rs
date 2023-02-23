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

use ink_prelude::{string::ToString, vec::Vec};
use sp_runtime::{generic::Era, AccountId32};

use privadex_chain_metadata::{
    common::{Amount, BlockNum, Nonce, SecretKey, UniversalAddress},
    get_chain_info_from_chain_id,
    registry::chain::universal_chain_id_registry,
};
use privadex_common::{signature_scheme::SignatureScheme, utils::ss58_utils::Ss58Codec};
use privadex_execution_plan::execution_plan::{
    CrossChainStepStatus, EthPendingTxnId, FinalizedTxnId, PendingTxnId, SubstrateEventId,
    SubstrateFinalizedExtrinsicId, SubstratePendingEventId, SubstratePendingExtrinsicId,
    XCMTransferStep,
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
    extrinsic_call_factory::{
        moonbase_alpha_xtokens_transfer_multiasset, moonbeam_xtokens_transfer_multiasset,
        polkadot_xcm_limited_reserve_transfer_assets,
    },
    key_container::KeyContainer,
    substrate_utils::{
        extrinsic_sig_config::ExtrinsicSigConfig,
        indexer_utils::subsquid_utils::SubstrateSubsquidUtils,
        node_rpc_utils::SubstrateNodeRpcUtils,
    },
};

impl Executable for XCMTransferStep {
    fn get_status(&self) -> ExecutableSimpleStatus {
        (&self.status).into()
    }

    fn get_total_fee_usd(&self) -> Option<Amount> {
        if self.get_status() == ExecutableSimpleStatus::Succeeded {
            Some(self.common.gas_fee_usd + self.bridge_fee_usd)
        } else {
            None
        }
    }

    fn execute_step_forward(
        &mut self,
        execute_step_meta: &ExecuteStepMeta,
        keys: &KeyContainer,
    ) -> ExecutableResult<StepForwardResult> {
        let optional_intermediate_result = match &self.status {
            CrossChainStepStatus::Dropped
            | CrossChainStepStatus::Failed(_)
            | CrossChainStepStatus::Confirmed(_, _) => {
                Err(ExecutableError::CalledStepForwardOnFinishedStep)
            }
            CrossChainStepStatus::NotStarted => self
                .execute_step_forward_if_notstarted(execute_step_meta, keys)
                .map(|res| Some(res)),
            CrossChainStepStatus::Submitted(pending_txn_id, pending_event_id) => {
                self.execute_step_forward_if_submitted(pending_txn_id, pending_event_id)
            }
            CrossChainStepStatus::LocalConfirmed(txn_id, pending_event_id) => {
                self.execute_step_forward_if_local_confirmed(txn_id, pending_event_id)
            }
        }?;

        if let Some(intermediate_step_res) = optional_intermediate_result {
            self.status = intermediate_step_res.new_status;
            if let Some(updated_gas_fee_native) = intermediate_step_res.updated_gas_fee_native {
                self.common.gas_fee_usd = get_updated_gas_fee_usd(
                    updated_gas_fee_native,
                    self.common.gas_fee_native,
                    self.common.gas_fee_usd,
                );
                self.common.gas_fee_native = updated_gas_fee_native;
            }
            Ok(StepForwardResult {
                did_status_change: true,
                amount_out: intermediate_step_res.amount_out,
            })
        } else {
            Ok(StepForwardResult {
                did_status_change: false,
                amount_out: None,
            })
        }
    }
}

struct IntermediateStepResult {
    pub new_status: CrossChainStepStatus,
    // For the MVP we do not parse fees from Substrate extrinsics and update them in our state,
    // so we keep all our estimates for bridge fees and for extrinsic-based gas fees.
    // We only update the gas fee if we sent it via an Ethereum transaction (e.g. Astar XCM precompile)
    pub updated_gas_fee_native: Option<Amount>,
    // amount_out is null if LocalConfirmed, 0 if Failed or Dropped, and a real value if Confirmed
    pub amount_out: Option<Amount>,
}

trait XCMTransferExecutableHelper {
    fn execute_step_forward_if_notstarted(
        &self,
        execute_step_meta: &ExecuteStepMeta,
        keys: &KeyContainer,
    ) -> ExecutableResult<IntermediateStepResult>;

    fn execute_step_forward_if_notstarted_astar_precompile(
        &self,
        src_chain_rpc_url: &str,
        src_cur_block: BlockNum,
        dest_cur_block: BlockNum,
        nonce: Nonce,
        amount: Amount,
        key: &SecretKey,
    ) -> ExecutableResult<IntermediateStepResult>;

    fn execute_step_forward_if_notstarted_substrate_extrinsic(
        &self,
        src_subutils: SubstrateNodeRpcUtils,
        src_cur_block: BlockNum,
        dest_cur_block: BlockNum,
        encoded_call_data: Vec<u8>,
        nonce: Nonce,
        key: &SecretKey,
    ) -> ExecutableResult<IntermediateStepResult>;

    fn execute_step_forward_if_submitted(
        &self,
        pending_txn_id: &PendingTxnId,
        pending_event_id: &SubstratePendingEventId,
    ) -> ExecutableResult<Option<IntermediateStepResult>>;

    fn execute_step_forward_if_submitted_eth_helper(
        &self,
        pending_txn_id: &EthPendingTxnId,
        pending_event_id: &SubstratePendingEventId,
    ) -> ExecutableResult<Option<IntermediateStepResult>>;

    fn execute_step_forward_if_submitted_substrate_helper(
        &self,
        pending_txn_id: &SubstratePendingExtrinsicId,
        pending_event_id: &SubstratePendingEventId,
    ) -> ExecutableResult<Option<IntermediateStepResult>>;

    fn execute_step_forward_if_local_confirmed(
        &self,
        txn_id: &FinalizedTxnId,
        pending_event_id: &SubstratePendingEventId,
    ) -> ExecutableResult<Option<IntermediateStepResult>>;
}

impl XCMTransferExecutableHelper for XCMTransferStep {
    fn execute_step_forward_if_notstarted(
        &self,
        execute_step_meta: &ExecuteStepMeta,
        keys: &KeyContainer,
    ) -> ExecutableResult<IntermediateStepResult> {
        let (src_chain_info, src_subutils, src_cur_block, _) =
            helpers::get_chain_utils(&self.src_token.chain)?;
        let (_, _, dest_cur_block, _) = helpers::get_chain_utils(&self.dest_token.chain)?;

        // Using NonceManager to get the nonce in a concurrent-safe way
        let nonce = {
            let system_nonce = {
                match self.common.src_addr {
                    UniversalAddress::Ethereum(eth_addr) => {
                        eth_utils::common::get_next_system_nonce(
                            src_chain_info.rpc_url,
                            eth_addr.clone(),
                        )
                        .map_err(|_| ExecutableError::RpcRequestFailed)
                    }
                    UniversalAddress::Substrate(substrate_addr) => {
                        let ss58_prefix = src_chain_info
                            .get_ss58_prefix()
                            .ok_or(ExecutableError::Ss58AddressFormatNotFound)?;
                        let ss58_address = AccountId32::new(substrate_addr.0)
                            .to_ss58check_with_version(ss58_prefix);
                        src_subutils
                            .get_next_system_nonce(&ss58_address)
                            .map_err(|_| ExecutableError::RpcRequestFailed)
                    }
                }
            }?;
            execute_step_meta.get_nonce(
                &self.uuid,
                self.src_token.chain,
                src_cur_block,
                system_nonce,
            )
        }?;
        let amount = self
            .amount_in
            .ok_or(ExecutableError::UnexpectedNullAmount)?;
        let key = keys
            .get_key(&self.common.src_addr)
            .ok_or(ExecutableError::SecretNotFound)?;

        // Special handling for Astar's precompile
        if self.src_token.chain == universal_chain_id_registry::ASTAR {
            return self.execute_step_forward_if_notstarted_astar_precompile(
                src_chain_info.rpc_url,
                src_cur_block,
                dest_cur_block,
                nonce,
                amount,
                key,
            );
        }

        // General handling for Substrate extrinsics
        let asset = xcm::prelude::MultiAsset {
            id: xcm::prelude::AssetId::Concrete(self.token_asset_multilocation.clone()),
            fun: xcm::prelude::Fungible(amount),
        };
        let encoded_call_data = match &self.src_token.chain {
            &universal_chain_id_registry::MOONBEAM => {
                moonbeam_xtokens_transfer_multiasset(asset, self.full_dest_multilocation.clone())
                    .map_err(|_| ExecutableError::FailedToCreateTxn)
            }
            &universal_chain_id_registry::POLKADOT => polkadot_xcm_limited_reserve_transfer_assets(
                asset,
                self.full_dest_multilocation.clone(),
            )
            .map_err(|_| ExecutableError::FailedToCreateTxn),
            &universal_chain_id_registry::MOONBASE_ALPHA => {
                moonbase_alpha_xtokens_transfer_multiasset(
                    asset,
                    self.full_dest_multilocation.clone(),
                )
                .map_err(|_| ExecutableError::FailedToCreateTxn)
            }
            _ => Err(ExecutableError::UnsupportedChain),
        }?;
        self.execute_step_forward_if_notstarted_substrate_extrinsic(
            src_subutils,
            src_cur_block,
            dest_cur_block,
            encoded_call_data,
            nonce,
            key,
        )
    }

    fn execute_step_forward_if_notstarted_astar_precompile(
        &self,
        src_chain_rpc_url: &str,
        src_cur_block: BlockNum,
        dest_cur_block: BlockNum,
        nonce: Nonce,
        amount: Amount,
        key: &SecretKey,
    ) -> ExecutableResult<IntermediateStepResult> {
        let astar_xcm_precompile =
            eth_utils::astar_xcm_precompile_contract::AstarXcmContract::new(src_chain_rpc_url)
                .map_err(|_| ExecutableError::FailedToLoadAstarPrecompileContract)?;
        let signed_txn = astar_xcm_precompile
            .assets_xcm_transfer(
                &self.src_token,
                amount,
                self.dest_token.chain,
                self.common.dest_addr.clone(),
                key,
                nonce,
            )
            .map_err(|_| ExecutableError::FailedToCreateTxn)?;

        let txn_hash = eth_utils::common::send_raw_transaction(src_chain_rpc_url, signed_txn)
            .map_err(|_| ExecutableError::RpcRequestFailed)?;

        Ok(IntermediateStepResult {
            new_status: CrossChainStepStatus::Submitted(
                PendingTxnId::Ethereum(EthPendingTxnId {
                    txn_hash,
                    end_block_num: src_cur_block + TXN_NUM_BLOCKS_ALIVE,
                }),
                SubstratePendingEventId {
                    start_block_num: dest_cur_block,
                },
            ),
            updated_gas_fee_native: None,
            amount_out: None,
        })
    }

    fn execute_step_forward_if_notstarted_substrate_extrinsic(
        &self,
        src_subutils: SubstrateNodeRpcUtils,
        src_cur_block: BlockNum,
        dest_cur_block: BlockNum,
        encoded_call_data: Vec<u8>,
        nonce: Nonce,
        key: &SecretKey,
    ) -> ExecutableResult<IntermediateStepResult> {
        let runtime_version = src_subutils
            .get_runtime_version()
            .map_err(|_| ExecutableError::RpcRequestFailed)?;
        let genesis_hash = src_subutils
            .get_genesis_hash()
            .map_err(|_| ExecutableError::RpcRequestFailed)?;
        let era = Era::Immortal;
        // TODO: Using a mortal error causes bad extrinsic signatures (at least on Moonbeam).
        // Need to investigate late on how to resolve that
        // let era = Era::mortal(TXN_NUM_BLOCKS_ALIVE.into(), src_cur_block.into());
        let finalized_head = if era != Era::Immortal {
            src_subutils
                .get_finalized_head_hash()
                .map_err(|_| ExecutableError::RpcRequestFailed)?
        } else {
            genesis_hash.clone()
        };

        let tx_raw = match self.common.src_addr {
            UniversalAddress::Ethereum(eth_addr) => {
                let sigconfig = ExtrinsicSigConfig::<[u8; 20]> {
                    sig_scheme: SignatureScheme::Ethereum,
                    signer: eth_addr.0,
                    privkey: key.to_vec(),
                };
                src_subutils.create_extrinsic::<[u8; 20]>(
                    sigconfig,
                    &encoded_call_data,
                    nonce,
                    runtime_version,
                    genesis_hash,
                    finalized_head, // checkpoint block hash
                    era,
                    0, // tip
                )
            }
            UniversalAddress::Substrate(substrate_addr) => {
                let sigconfig = ExtrinsicSigConfig::<[u8; 32]> {
                    sig_scheme: SignatureScheme::Sr25519,
                    signer: substrate_addr.0,
                    privkey: key.to_vec(),
                };
                src_subutils.create_extrinsic::<[u8; 32]>(
                    sigconfig,
                    &encoded_call_data,
                    nonce,
                    runtime_version,
                    genesis_hash,
                    finalized_head, // checkpoint block hash
                    era,
                    0, // tip
                )
            }
        };

        ink_env::debug_println!(
            "Tx: {:?}",
            privadex_common::utils::general_utils::slice_to_hex_string(&tx_raw)
        );

        let res = src_subutils.send_extrinsic(&tx_raw);

        ink_env::debug_println!("XCM transfer send_extrinsic: {:?}", res);

        let extrinsic_hash = res.map_err(|_| ExecutableError::RpcRequestFailed)?;

        Ok(IntermediateStepResult {
            new_status: CrossChainStepStatus::Submitted(
                PendingTxnId::Substrate(SubstratePendingExtrinsicId {
                    start_block_num: src_cur_block,
                    // synced with transaction mortality
                    end_block_num: src_cur_block + TXN_NUM_BLOCKS_ALIVE,
                    extrinsic_hash,
                }),
                SubstratePendingEventId {
                    start_block_num: dest_cur_block,
                },
            ),
            updated_gas_fee_native: None,
            amount_out: None,
        })
    }

    fn execute_step_forward_if_submitted(
        &self,
        pending_txn_id: &PendingTxnId,
        pending_event_id: &SubstratePendingEventId,
    ) -> ExecutableResult<Option<IntermediateStepResult>> {
        let intermediate_step_result = match pending_txn_id {
            PendingTxnId::Ethereum(eth_pending_txn_id) => self
                .execute_step_forward_if_submitted_eth_helper(
                    &eth_pending_txn_id,
                    pending_event_id,
                ),
            PendingTxnId::Substrate(substrate_pending_extrinsic_id) => self
                .execute_step_forward_if_submitted_substrate_helper(
                    &substrate_pending_extrinsic_id,
                    pending_event_id,
                ),
        }?;
        match &intermediate_step_result {
            // If the previous step returned LocalConfirmed status, we check if we are Confirmed on the remote chain also
            Some(IntermediateStepResult {
                new_status: CrossChainStepStatus::LocalConfirmed(txn_id, pending_event_id),
                updated_gas_fee_native,
                amount_out: _,
            }) => {
                if let Ok(Some(confirmed_step_result)) =
                    self.execute_step_forward_if_local_confirmed(txn_id, pending_event_id)
                {
                    Ok(Some(IntermediateStepResult {
                        new_status: confirmed_step_result.new_status,
                        // We use the updated_gas_fee_native from the LocalConfirmed
                        // step since Confirmed never sets gas fee (as it looks up
                        // the remote chain whereas gas fees are on the local chain)
                        updated_gas_fee_native: *updated_gas_fee_native,
                        amount_out: confirmed_step_result.amount_out,
                    }))
                } else {
                    Ok(intermediate_step_result)
                }
            }
            // Below captures Failed or Dropped
            Some(_) => Ok(intermediate_step_result),
            None => Ok(None),
        }
    }

    fn execute_step_forward_if_submitted_eth_helper(
        &self,
        pending_txn_id: &EthPendingTxnId,
        pending_event_id: &SubstratePendingEventId,
    ) -> ExecutableResult<Option<IntermediateStepResult>> {
        let (src_chain_info, _, src_cur_block, _) =
            helpers::get_chain_utils(&self.src_token.chain)?;

        if src_cur_block > pending_txn_id.end_block_num {
            Ok(Some(IntermediateStepResult {
                new_status: CrossChainStepStatus::Dropped,
                updated_gas_fee_native: Some(0),
                amount_out: Some(0),
            }))
        } else if let Ok(txn_summary) = eth_utils::parse_txn_helper::get_txn_summary(
            src_chain_info.rpc_url,
            pending_txn_id.txn_hash,
        ) {
            let finalized_txn_id = FinalizedTxnId::Ethereum(pending_txn_id.txn_hash);
            if txn_summary.is_txn_success {
                Ok(Some(IntermediateStepResult {
                    new_status: CrossChainStepStatus::LocalConfirmed(
                        finalized_txn_id,
                        pending_event_id.clone(),
                    ),
                    updated_gas_fee_native: Some(txn_summary.gas_fee_native),
                    amount_out: None,
                }))
            } else {
                Ok(Some(IntermediateStepResult {
                    new_status: CrossChainStepStatus::Failed(finalized_txn_id),
                    updated_gas_fee_native: Some(txn_summary.gas_fee_native),
                    amount_out: Some(0),
                }))
            }
        } else {
            Ok(None)
        }
    }

    fn execute_step_forward_if_submitted_substrate_helper(
        &self,
        pending_txn_id: &SubstratePendingExtrinsicId,
        pending_event_id: &SubstratePendingEventId,
    ) -> ExecutableResult<Option<IntermediateStepResult>> {
        let (_, _, src_cur_block, src_subsquid_utils) =
            helpers::get_chain_utils(&self.src_token.chain)?;
        if src_cur_block > pending_txn_id.end_block_num {
            Ok(Some(IntermediateStepResult {
                new_status: CrossChainStepStatus::Dropped,
                updated_gas_fee_native: Some(0),
                amount_out: Some(0),
            }))
        } else if let Ok(extrinsic_summary) = src_subsquid_utils.lookup_extrinsic_by_hash(
            pending_txn_id.start_block_num,
            src_cur_block,
            &pending_txn_id.extrinsic_hash,
        ) {
            let finalized_txn_id = FinalizedTxnId::Substrate(SubstrateFinalizedExtrinsicId {
                block_num: extrinsic_summary.block_num,
                extrinsic_index: extrinsic_summary.extrinsic_index,
            });
            if extrinsic_summary.is_extrinsic_success {
                Ok(Some(IntermediateStepResult {
                    new_status: CrossChainStepStatus::LocalConfirmed(
                        finalized_txn_id,
                        pending_event_id.clone(),
                    ),
                    updated_gas_fee_native: None, // we do not update fees for parsed extrinsics
                    amount_out: None,
                }))
            } else {
                Ok(Some(IntermediateStepResult {
                    new_status: CrossChainStepStatus::Failed(finalized_txn_id),
                    updated_gas_fee_native: None,
                    amount_out: Some(0),
                }))
            }
        } else {
            Ok(None)
        }
    }

    fn execute_step_forward_if_local_confirmed(
        &self,
        txn_id: &FinalizedTxnId,
        pending_event_id: &SubstratePendingEventId,
    ) -> ExecutableResult<Option<IntermediateStepResult>> {
        let amount = self
            .amount_in
            .ok_or(ExecutableError::UnexpectedNullAmount)?;
        let (_, _, dest_cur_block, dest_subsquid_utils) =
            helpers::get_chain_utils(&self.dest_token.chain)?;

        if let Ok(xcm_transfer_event_summary) = dest_subsquid_utils.lookup_xcm_event_transfer(
            pending_event_id.start_block_num,
            dest_cur_block,
            self.src_token.clone(),
            self.dest_token.clone(),
            amount,
            self.common.dest_addr.clone(),
        ) {
            Ok(Some(IntermediateStepResult {
                new_status: CrossChainStepStatus::Confirmed(
                    txn_id.clone(),
                    SubstrateEventId {
                        block_num: xcm_transfer_event_summary.block_num,
                        event_index: xcm_transfer_event_summary.event_index,
                    },
                ),
                updated_gas_fee_native: None,
                amount_out: Some(xcm_transfer_event_summary.amount_out),
            }))
        } else {
            Ok(None)
        }
    }
}

mod helpers {
    use privadex_chain_metadata::{chain_info::ChainInfo, common::UniversalChainId};

    use super::*;

    pub(super) fn get_chain_utils(
        chain_id: &UniversalChainId,
    ) -> ExecutableResult<(
        &ChainInfo,
        SubstrateNodeRpcUtils,
        BlockNum,
        SubstrateSubsquidUtils,
    )> {
        let chain_info = get_chain_info_from_chain_id(&chain_id)
            .ok_or(ExecutableError::FailedToFindChainInfo)?;
        let subutils = SubstrateNodeRpcUtils {
            rpc_url: chain_info.rpc_url.to_string(),
        };
        let cur_block = subutils
            .get_finalized_block_number()
            .map_err(|_| ExecutableError::RpcRequestFailed)?;
        let subsquid_utils = SubstrateSubsquidUtils {
            subsquid_graphql_archive_url: chain_info.subsquid_graphql_archive_url.to_string(),
        };
        Ok((chain_info, subutils, cur_block, subsquid_utils))
    }
}
