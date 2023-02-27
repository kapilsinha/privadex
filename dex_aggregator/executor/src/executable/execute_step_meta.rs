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

use ink_prelude::{
    string::{String, ToString},
    vec,
    vec::Vec,
};
use scale::{Decode, Encode};

use privadex_chain_metadata::{
    common::{BlockNum, EthTxnHash, MillisSinceEpoch, Nonce, UniversalChainId},
    get_chain_info_from_chain_id,
    registry::chain::universal_chain_id_registry,
};
use privadex_common::{utils::s3_api::S3Api, uuid::Uuid};
use privadex_execution_plan::execution_plan::ExecutionPlan;

use super::traits::{ExecutableError, ExecutableResult};
use crate::{
    concurrency_coordinator::{
        execution_plan_assigner::ExecutionPlanAssigner, nonce_manager::NonceManager,
        prestart_step_uniqueness_enforcer::PrestartStepUniquenessEnforcer,
    },
    substrate_utils::node_rpc_utils::SubstrateNodeRpcUtils,
};

/// Necessary metadata to execute a step
/// Initially I was going to make this a trait/template but it becomes
/// really messy so I just created an enum
pub enum ExecuteStepMeta {
    NoCloudStorage(DummyExecuteStepMeta),
    WithCloudStorage(LiveExecuteStepMeta),
}

pub struct DummyExecuteStepMeta {
    cur_timestamp: MillisSinceEpoch,
}

pub struct LiveExecuteStepMeta {
    cur_timestamp: MillisSinceEpoch,
    s3_api: S3Api,
    exec_plan_assigner: ExecutionPlanAssigner,
    prestart_step_uniqueness_enforcer: PrestartStepUniquenessEnforcer,
    chain_nonce_managers: Vec<(UniversalChainId, NonceManager)>,
}

impl ExecuteStepMeta {
    pub fn dummy(cur_timestamp: MillisSinceEpoch) -> Self {
        Self::NoCloudStorage(DummyExecuteStepMeta { cur_timestamp })
    }

    // Deliberately named this way so that the user knows (and I remember) these are
    // the supported chains
    pub fn new_for_astar_moonbeam_polkadot(
        cur_timestamp: MillisSinceEpoch,
        s3_access_key: String,
        s3_secret_key: String,
        dynamodb_access_key: String,
        dynamodb_secret_key: String,
    ) -> Self {
        let s3_api = S3Api::new(s3_access_key, s3_secret_key);
        let exec_plan_assigner = ExecutionPlanAssigner::new(
            dynamodb_access_key.clone(),
            dynamodb_secret_key.clone(),
            cur_timestamp,
        );
        let prestart_step_uniqueness_enforcer = PrestartStepUniquenessEnforcer::new(
            dynamodb_access_key.clone(),
            dynamodb_secret_key.clone(),
            cur_timestamp,
        );
        let chain_nonce_managers = {
            let astar_nonce_manager = NonceManager::new(
                dynamodb_access_key.clone(),
                dynamodb_secret_key.clone(),
                "astar",
                cur_timestamp,
            );
            let moonbeam_nonce_manager = NonceManager::new(
                dynamodb_access_key.clone(),
                dynamodb_secret_key.clone(),
                "moonbeam",
                cur_timestamp,
            );
            let polkadot_nonce_manager = NonceManager::new(
                dynamodb_access_key.clone(),
                dynamodb_secret_key.clone(),
                "polkadot",
                cur_timestamp,
            );
            vec![
                (universal_chain_id_registry::ASTAR, astar_nonce_manager),
                (
                    universal_chain_id_registry::MOONBEAM,
                    moonbeam_nonce_manager,
                ),
                (
                    universal_chain_id_registry::POLKADOT,
                    polkadot_nonce_manager,
                ),
            ]
        };
        Self::WithCloudStorage(LiveExecuteStepMeta {
            cur_timestamp,
            s3_api,
            exec_plan_assigner,
            prestart_step_uniqueness_enforcer,
            chain_nonce_managers,
        })
    }

    pub fn cur_timestamp(&self) -> MillisSinceEpoch {
        match self {
            Self::NoCloudStorage(dummy) => dummy.cur_timestamp,
            Self::WithCloudStorage(live) => live.cur_timestamp,
        }
    }

    pub fn save_exec_plan_to_s3(&self, exec_plan: &ExecutionPlan) -> ExecutableResult<()> {
        match self {
            Self::NoCloudStorage(_) => Ok(()),
            Self::WithCloudStorage(live) => {
                let object_key = exec_plan.uuid.to_hex_string();
                let bucket_name = "execution-plan".to_string();
                live.s3_api
                    .put_object_raw(
                        live.cur_timestamp,
                        "storj".to_string(),
                        object_key,
                        bucket_name,
                        "us-east-1".to_string(),
                        &exec_plan.encode(),
                    )
                    .map_or_else(|_| Err(ExecutableError::FailedToSaveToS3), |_| Ok(()))
            }
        }
    }

    pub fn pull_exec_plan_from_s3(&self, exec_plan_uuid: &Uuid) -> ExecutableResult<ExecutionPlan> {
        match self {
            Self::NoCloudStorage(_) => Err(ExecutableError::FailedToPullFromS3),
            Self::WithCloudStorage(live) => {
                let object_key = exec_plan_uuid.to_hex_string();
                let bucket_name = "execution-plan".to_string();
                let exec_plan_bytes = live
                    .s3_api
                    .get_object_raw(
                        live.cur_timestamp,
                        "storj".to_string(),
                        object_key,
                        bucket_name,
                        "us-east-1".to_string(),
                    )
                    .map_err(|_| ExecutableError::FailedToPullFromS3)?;
                ExecutionPlan::decode(&mut exec_plan_bytes.as_slice()).map_or_else(
                    |_| Err(ExecutableError::FailedToDeserializeFromS3),
                    |exec_plan| Ok(exec_plan),
                )
            }
        }
    }

    pub fn claim_exec_plan(&self, exec_plan_uuid: &Uuid) -> bool /* didClaimSuccessfully */ {
        match self {
            Self::NoCloudStorage(_) => true,
            Self::WithCloudStorage(live) => {
                if let Ok(true) = live
                    .exec_plan_assigner
                    .attempt_allocate_exec_plan(exec_plan_uuid)
                {
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn unclaim_exec_plan(&self, exec_plan_uuid: &Uuid) -> ExecutableResult<()> {
        match self {
            Self::NoCloudStorage(_) => Ok(()),
            Self::WithCloudStorage(live) => live
                .exec_plan_assigner
                .unallocate_exec_plan(exec_plan_uuid)
                .map_err(|_| ExecutableError::FailedToUpdateDynamoDb),
        }
    }

    pub fn register_exec_plan(&self, exec_plan_uuid: &Uuid) -> ExecutableResult<()> {
        match self {
            Self::NoCloudStorage(_) => Ok(()),
            Self::WithCloudStorage(live) => live
                .exec_plan_assigner
                .register_exec_plan(exec_plan_uuid)
                .map_err(|_| ExecutableError::FailedToUpdateDynamoDb),
        }
    }

    // We eat the error result because there is nothing the client can do (under a network issue)
    pub fn remove_completed_exec_plan(&self, exec_plan_uuid: &Uuid) -> ExecutableResult<()> {
        match self {
            Self::NoCloudStorage(_) => Ok(()),
            Self::WithCloudStorage(live) => live
                .exec_plan_assigner
                .remove_completed_execplan(exec_plan_uuid)
                .map_err(|_| ExecutableError::FailedToUpdateDynamoDb),
        }
    }

    pub fn get_nonce(
        &self,
        exec_step_uuid: &Uuid,
        src_chain: UniversalChainId,
        cur_block: BlockNum,
        system_nonce: Nonce,
    ) -> ExecutableResult<Nonce> {
        match self {
            Self::NoCloudStorage(_) => Ok(system_nonce),
            Self::WithCloudStorage(live) => {
                let nonce_man = Self::get_nonce_manager(live, src_chain)?;
                nonce_man
                    .get_nonce(exec_step_uuid, cur_block, system_nonce)
                    .map_err(|_| ExecutableError::FailedToGetNonce)
            }
        }
    }

    pub fn finalize_execstep(
        &self,
        exec_step_uuid: &Uuid,
        src_chain: UniversalChainId,
    ) -> ExecutableResult<()> {
        match self {
            Self::NoCloudStorage(_) => Ok(()),
            Self::WithCloudStorage(live) => {
                let nonce_man = Self::get_nonce_manager(live, src_chain)?;
                // We could have passed in cur_block but it makes the interface needlessly complex,
                // so we just compute it again here. Note: that may mean that we store +-1 in our
                // database, which is fine
                let cur_block = get_cur_block(&src_chain)?;
                nonce_man
                    .finalize_execstep(exec_step_uuid, cur_block)
                    .map_err(|_| ExecutableError::FailedToUpdateDynamoDb)
            }
        }
    }

    pub fn drop_execstep(
        &self,
        exec_step_uuid: &Uuid,
        src_chain: UniversalChainId,
    ) -> ExecutableResult<()> {
        match self {
            Self::NoCloudStorage(_) => Ok(()),
            Self::WithCloudStorage(live) => {
                let nonce_man = Self::get_nonce_manager(live, src_chain)?;
                nonce_man
                    .drop_execstep_from_id(exec_step_uuid)
                    .map_err(|_| ExecutableError::FailedToUpdateDynamoDb)
            }
        }
    }

    fn get_nonce_manager(
        live: &LiveExecuteStepMeta,
        chain_id: UniversalChainId,
    ) -> ExecutableResult<&NonceManager> {
        live.chain_nonce_managers
            .iter()
            .filter(|(chain, _)| *chain == chain_id)
            .next()
            .map_or(Err(ExecutableError::UnsupportedChain), |(_, nonce_man)| {
                Ok(nonce_man)
            })
    }

    pub fn register_prestart_txn_hash(&self, txn_hash: &EthTxnHash) -> bool /* is prestartTxnNew */
    {
        match self {
            Self::NoCloudStorage(_) => true,
            Self::WithCloudStorage(live) => {
                if let Ok(true) = live
                    .prestart_step_uniqueness_enforcer
                    .attempt_register_prestart_txn(txn_hash)
                {
                    true
                } else {
                    false
                }
            }
        }
    }
}

fn get_cur_block(chain_id: &UniversalChainId) -> ExecutableResult<BlockNum> {
    // We assume all ChainIds support Substrate-like extrinsics. Fine for the near future
    let chain_info =
        get_chain_info_from_chain_id(&chain_id).ok_or(ExecutableError::FailedToFindChainInfo)?;
    let subutils = SubstrateNodeRpcUtils {
        rpc_url: chain_info.rpc_url.to_string(),
    };
    subutils
        .get_finalized_block_number()
        .map_err(|_| ExecutableError::RpcRequestFailed)
}

#[cfg(test)]
mod execute_step_meta_tests {
    use super::*;

    fn now_millis() -> MillisSinceEpoch {
        use std::time::SystemTime;
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .try_into()
            .unwrap()
    }

    #[cfg(feature = "s3-live-test")]
    #[test]
    fn test_pull_exec_plan_from_s3() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let s3_access_key =
            std::env::var("S3_ACCESS_KEY").expect("Env var S3_ACCESS_KEY is not set");
        let s3_secret_key =
            std::env::var("S3_SECRET_KEY").expect("Env var S3_SECRET_KEY is not set");
        let meta = ExecuteStepMeta::new_for_astar_moonbeam_polkadot(
            now_millis(),
            s3_access_key,
            s3_secret_key,
            String::new(),
            String::new(),
        );
        let uuid = Uuid::from_str("6b9177a7f4aab43378be787cff1a25f1").unwrap();
        ink_env::debug_println!("Uuid = {:?}", uuid);
        let exec_plan = meta
            .pull_exec_plan_from_s3(&uuid)
            .expect("Failed to find exec plan");
        ink_env::debug_println!("Pulled execution plan: {:?}", exec_plan);
    }

    #[cfg(feature = "dynamodb-live-test")]
    #[test]
    fn test_remove_completed_exec_plan() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let dynamodb_access_key =
            std::env::var("DYNAMODB_ACCESS_KEY").expect("Env var DYNAMODB_ACCESS_KEY is not set");
        let dynamodb_secret_key =
            std::env::var("DYNAMODB_SECRET_KEY").expect("Env var DYNAMODB_SECRET_KEY is not set");
        let meta = ExecuteStepMeta::new_for_astar_moonbeam_polkadot(
            now_millis(),
            String::new(),
            String::new(),
            dynamodb_access_key,
            dynamodb_secret_key,
        );
        let uuid = Uuid::from_str("c7b008e74cc65d08d2f8814030c862bc").unwrap();
        ink_env::debug_println!("Uuid = {:?}", uuid);
        let removed_exec_plan = meta.remove_completed_exec_plan(&uuid);
        ink_env::debug_println!("Removed execution plan: {:?}", removed_exec_plan);
    }
}
