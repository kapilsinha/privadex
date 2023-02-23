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

use scale::{Decode, Encode};

use privadex_chain_metadata::common::Amount;
use privadex_execution_plan::execution_plan::{CrossChainStepStatus, EthStepStatus};

use super::execute_step_meta::ExecuteStepMeta;
use crate::key_container::KeyContainer;

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ExecutableSimpleStatus {
    NotStarted,
    InProgress,
    Failed,
    Dropped,
    Succeeded,
}

#[derive(Decode, Encode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ExecutableError {
    UnknownBadState,
    CalledStepForwardOnFinishedStep,
    CalledStepForwardOnFinishedPlan,
    EthTxnDropped,
    FailedToCreateTxn,
    FailedToDeserializeFromS3,
    FailedToFindChainInfo,
    FailedToGetNonce,
    FailedToLoadAstarPrecompileContract,
    FailedToLoadWethContract,
    FailedToPullFromS3,
    FailedToSaveToS3,
    FailedToUpdateDynamoDb,
    PrestartStepNotStarted,
    RpcRequestFailed,
    SecretNotFound,
    Ss58AddressFormatNotFound,
    SubstrateIndexerLookupFailed,
    UnexpectedNonEthAddress,
    UnexpectedNullAmount,
    UnexpectedNullEvmChainId,
    UnexpectedStepStatus,
    UnsupportedChain,
}
pub type ExecutableResult<T> = core::result::Result<T, ExecutableError>;

// Implement for ExecutionPlan, ExecutionPath, ExecutionStep
pub trait Executable {
    fn get_status(&self) -> ExecutableSimpleStatus;
    // Some($ * 10^USD_AMOUNT_EXPONENT) if and only if status == Succeeded, else None
    fn get_total_fee_usd(&self) -> Option<Amount>;
    fn execute_step_forward(
        &mut self,
        execute_step_meta: &ExecuteStepMeta,
        keys: &KeyContainer,
    ) -> ExecutableResult<StepForwardResult>;
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct StepForwardResult {
    // True if status changed, False otherwise
    // (i.e. true if we need to update S3 with the new contents)
    pub did_status_change: bool,
    // Some(x) if the step succeeded, else None
    pub amount_out: Option<Amount>,
}

impl From<&EthStepStatus> for ExecutableSimpleStatus {
    fn from(status: &EthStepStatus) -> Self {
        match status {
            EthStepStatus::NotStarted => Self::NotStarted,
            EthStepStatus::Submitted(_) => Self::InProgress,
            EthStepStatus::Dropped => Self::Dropped,
            EthStepStatus::Failed(_) => Self::Failed,
            EthStepStatus::Confirmed(_) => Self::Succeeded,
        }
    }
}

impl From<&CrossChainStepStatus> for ExecutableSimpleStatus {
    fn from(status: &CrossChainStepStatus) -> Self {
        match status {
            CrossChainStepStatus::NotStarted => Self::NotStarted,
            CrossChainStepStatus::Dropped => Self::Dropped,
            CrossChainStepStatus::Failed(_) => Self::Failed,
            CrossChainStepStatus::Submitted(_, _) => Self::InProgress,
            CrossChainStepStatus::LocalConfirmed(_, _) => Self::InProgress,
            CrossChainStepStatus::Confirmed(_, _) => Self::Succeeded,
        }
    }
}
