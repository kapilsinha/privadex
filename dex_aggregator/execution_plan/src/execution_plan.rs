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

use core::fmt::{self, Debug};
use ink_prelude::vec::Vec;
use scale::{Decode, Encode};
use xcm::latest::MultiLocation;

use privadex_common::uuid::Uuid;

use privadex_chain_metadata::common::{
    Amount, BlockNum, EthAddress, EthTxnHash, Nonce, SubstrateExtrinsicHash, UniversalAddress,
    UniversalChainId, UniversalTokenId,
};

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ExecutionPlan {
    pub uuid: Uuid,
    pub paths: Vec<ExecutionPath>,
    pub prestart_user_to_escrow_transfer: ExecutionStep, // EthSend/ERC20Transfer from user to escrow
    pub postend_escrow_to_user_transfer: ExecutionStep, // EthSend/ERC20Transfer from escrow to user
}

impl fmt::Display for ExecutionPlan {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let _ = write!(
            f,
            "ExecutionPlan [{:?}]: \nprestart_user_to_escrow_transfer = {:?}, \
			 \npostend_escrow_to_user_transfer = {:?}",
            self.uuid, self.prestart_user_to_escrow_transfer, self.postend_escrow_to_user_transfer
        );
        for (i, p) in self.paths.iter().enumerate() {
            let _ = write!(f, "\nExecutionPath {}: {}", i + 1, p);
        }
        Ok(())
    }
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ExecutionPath {
    pub steps: Vec<ExecutionStep>,
    pub amount_out: Option<Amount>,
    // TODO: Should also add amount_out_usd so that we can properly
    // compute the fee when all ExecutionPaths finish
}

impl fmt::Display for ExecutionPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ExecutionPath ({} steps, amount_out = {:?}):",
            self.steps.len(),
            self.amount_out
        )?;
        for edge in self.steps.iter() {
            write!(f, "\n  {:?}", edge)?;
        }
        Ok(())
    }
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ExecutionStep {
    // There used to be other stuff in this outer struct. I have kept it as a
    // singleton struct instead of collapsing it down for ease of adding items in
    // the future
    pub inner: ExecutionStepEnum,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ExecutionStepEnum {
    // Substrate extrinsic to the balances pallet - avoid supporting for now
    // SubstrateNativeTokenTransfer(SubstrateNativeTokenTransferStep),

    // Sends the chain's native token using Ethereum send interface
    EthSend(EthSendStep),
    // ERC20 contract.transfer
    ERC20Transfer(ERC20TransferStep),

    // Converts the chain's native token to/from its wrapped version e.g.
    // wrap GLMR to WGLMR, unwrap WGLMR for GLMR
    EthWrap(EthWrapStep),
    EthUnwrap(EthUnwrapStep),

    // DEX router function call to swapExactTokensForTokens/swapExactETHForTokens
    // Might become more complex later if we handle stableswap or concentrated liquidity
    EthDexSwap(EthDexSwapStep),

    // Substrate XCM transfer extrinsic e.g.
    // xcmPallet.limitedReserveTransferAssets from Polkadot
    // polkadotXcm.limitedReserveTransferAssets from Astar
    //  (identical to Polkadot's XCM interface)
    // xTokens.transferMultiasset from Moonbeam
    // xTransfer.transfer from Phala
    XCMTransfer(XCMTransferStep),
    // FYI Batch will be inelegant since I insert status into the ExecutionStep
    // struct MoonbeamBatchStep { substeps: Vec<ExecutionStep>, ... }
    // MoonbeamBatch(MoonbeamBatchStep),
}

impl ExecutionStep {
    pub fn new(inner: ExecutionStepEnum) -> Self {
        Self { inner }
    }

    pub fn get_amount_in(&self) -> Option<Amount> {
        match &self.inner {
            ExecutionStepEnum::EthSend(step) => step.amount,
            ExecutionStepEnum::ERC20Transfer(step) => step.amount,
            ExecutionStepEnum::EthWrap(step) => step.amount,
            ExecutionStepEnum::EthUnwrap(step) => step.amount,
            ExecutionStepEnum::EthDexSwap(step) => step.amount_in,
            ExecutionStepEnum::XCMTransfer(step) => step.amount_in,
        }
    }

    pub fn set_amount_in(&mut self, amount_in: Amount) {
        match &mut self.inner {
            ExecutionStepEnum::EthSend(step) => step.amount = Some(amount_in),
            ExecutionStepEnum::ERC20Transfer(step) => step.amount = Some(amount_in),
            ExecutionStepEnum::EthWrap(step) => step.amount = Some(amount_in),
            ExecutionStepEnum::EthUnwrap(step) => step.amount = Some(amount_in),
            ExecutionStepEnum::EthDexSwap(step) => step.amount_in = Some(amount_in),
            ExecutionStepEnum::XCMTransfer(step) => step.amount_in = Some(amount_in),
        }
    }

    pub fn drop(&mut self) {
        match &mut self.inner {
            ExecutionStepEnum::EthSend(step) => step.status = EthStepStatus::Dropped,
            ExecutionStepEnum::ERC20Transfer(step) => step.status = EthStepStatus::Dropped,
            ExecutionStepEnum::EthWrap(step) => step.status = EthStepStatus::Dropped,
            ExecutionStepEnum::EthUnwrap(step) => step.status = EthStepStatus::Dropped,
            ExecutionStepEnum::EthDexSwap(step) => step.status = EthStepStatus::Dropped,
            ExecutionStepEnum::XCMTransfer(step) => step.status = CrossChainStepStatus::Dropped,
        }
    }

    pub fn get_src_chain(&self) -> UniversalChainId {
        match &self.inner {
            ExecutionStepEnum::EthSend(step) => step.chain,
            ExecutionStepEnum::ERC20Transfer(step) => step.token.chain,
            ExecutionStepEnum::EthWrap(step) => step.chain,
            ExecutionStepEnum::EthUnwrap(step) => step.chain,
            ExecutionStepEnum::EthDexSwap(step) => step.token_path[0].chain,
            ExecutionStepEnum::XCMTransfer(step) => step.src_token.chain,
        }
    }

    pub fn get_uuid(&self) -> &Uuid {
        match &self.inner {
            ExecutionStepEnum::EthSend(step) => &step.uuid,
            ExecutionStepEnum::ERC20Transfer(step) => &step.uuid,
            ExecutionStepEnum::EthWrap(step) => &step.uuid,
            ExecutionStepEnum::EthUnwrap(step) => &step.uuid,
            ExecutionStepEnum::EthDexSwap(step) => &step.uuid,
            ExecutionStepEnum::XCMTransfer(step) => &step.uuid,
        }
    }
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct CommonExecutionMeta {
    pub src_addr: UniversalAddress,  // wallet src
    pub dest_addr: UniversalAddress, // wallet dest
    pub gas_fee_native: Amount,      // native token
    // Below is derived from the price feed's token.derived_usd
    pub gas_fee_usd: Amount, // in $ * USD_DECIMALS
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct EthSendStep {
    pub uuid: Uuid,
    pub chain: UniversalChainId,
    // Null if we rely on the previous step's output for this, else non-null
    pub amount: Option<Amount>,
    pub common: CommonExecutionMeta,
    pub status: EthStepStatus,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct EthWrapStep {
    pub uuid: Uuid,
    pub chain: UniversalChainId,
    pub amount: Option<Amount>,
    pub common: CommonExecutionMeta,
    pub status: EthStepStatus,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct EthUnwrapStep {
    pub uuid: Uuid,
    pub chain: UniversalChainId,
    pub amount: Option<Amount>,
    pub common: CommonExecutionMeta,
    pub status: EthStepStatus,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ERC20TransferStep {
    pub uuid: Uuid,
    pub token: UniversalTokenId,
    pub amount: Option<Amount>,
    pub common: CommonExecutionMeta,
    pub status: EthStepStatus,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum DexRouterFunction {
    SwapExactETHForTokens,
    SwapExactTokensForTokens,
    SwapExactTokensForETH,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct EthDexSwapStep {
    pub uuid: Uuid,
    // This will become more complex later (perhaps make the addr an enum
    // e.g enabling us to not use the DEX router for stableswap) but keep it simple for now
    pub dex_router_addr: EthAddress,
    pub dex_router_func: DexRouterFunction,
    pub token_path: Vec<UniversalTokenId>, // token.chain are all the same of course
    pub amount_in: Option<Amount>,
    // Eventually will add amount_out_min
    pub common: CommonExecutionMeta,
    pub status: EthStepStatus,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct XCMTransferStep {
    pub uuid: Uuid,
    pub src_token: UniversalTokenId,
    pub dest_token: UniversalTokenId,
    pub token_asset_multilocation: MultiLocation,
    pub full_dest_multilocation: MultiLocation,
    pub amount_in: Option<Amount>,
    pub bridge_fee_native: Amount,
    pub bridge_fee_usd: Amount,
    pub common: CommonExecutionMeta,
    pub status: CrossChainStepStatus,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum EthStepStatus {
    // Haven't started executing this step yet, which is the default status.
    NotStarted,
    // Transaction has been sent with transaction hash returned.
    // end_block_number = cur_block + CONSTANT (set CONSTANT to something
    // generous like 50; we should send a cancel instruction then but won't
    // do that for simplicity in v1)
    Submitted(EthPendingTxnId),
    // Transaction has been sent but was dropped.
    // We detect this if we (a) never received a txn hash or
    // (b) are in Submitted state past end_block_number
    Dropped,
    // Transaction has been sent but failed to execute by the node.
    Failed(EthTxnHash),
    // Transaction has been sent and included in a specific block
    Confirmed(EthTxnHash),
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum PendingTxnId {
    Ethereum(EthPendingTxnId),
    Substrate(SubstratePendingExtrinsicId),
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct EthPendingTxnId {
    // Fields used to look up the txn
    pub txn_hash: EthTxnHash,
    pub end_block_num: BlockNum,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct SubstratePendingExtrinsicId {
    // Fields used to look up the extrinsic
    pub start_block_num: BlockNum,
    pub end_block_num: BlockNum,
    pub extrinsic_hash: SubstrateExtrinsicHash,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum FinalizedTxnId {
    Ethereum(EthTxnHash),
    Substrate(SubstrateFinalizedExtrinsicId),
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct SubstrateFinalizedExtrinsicId {
    pub block_num: BlockNum,
    pub extrinsic_index: Nonce,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct SubstratePendingEventId {
    // To be used to find the event on a remote chain
    pub start_block_num: BlockNum,
    // Use fields on the ExecutionStep (e.g. XCMTransferStep) as the identifier
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct SubstrateEventId {
    pub block_num: BlockNum,
    pub event_index: Nonce,
}

// Will need to implement this when we handle intra-chain extrinsics
// e.g. SubstrateNativeTokenTransfer
// enum SubstrateStepStatus { ... }

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum CrossChainStepStatus {
    // Haven't started executing this step yet, which is the default status.
    NotStarted,
    // Transaction has been sent to the local chain
    Submitted(PendingTxnId, SubstratePendingEventId),
    // Transaction has been sent but was dropped accidentally by the node.
    // Detected if we don't see the txn and we are past end_block_num
    Dropped,
    // Transaction has been included in a block but failed (locally).
    Failed(FinalizedTxnId),
    // Transaction has been included in a block (and succeeded) on the
    // local chain but not the remote chain
    LocalConfirmed(FinalizedTxnId, SubstratePendingEventId),
    // Transaction has been included in a block on the local chain and
    // produced an event on the remote chain
    Confirmed(FinalizedTxnId, SubstrateEventId),
}
