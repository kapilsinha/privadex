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
    format,
    string::{String, ToString},
};

use privadex_chain_metadata::common::{BlockNum, MillisSinceEpoch, Nonce};
use privadex_common::{
    utils::dynamodb_api::{DynamoDbAction, DynamoDbApi, DynamoDbError},
    uuid::Uuid,
};

use super::{
    deserialize_helper::{
        AttributesWrapper, Empty, ItemWrapper, PendingNonceBlockNextResponse,
        PendingNonceBlockResponse,
    },
    dynamodb_request_factory::DynamoDbNonceRequestFactory,
};

const DYNAMODB_TABLE_NONCE: &'static str = "privadex_phat_contract";

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum NonceManagerError {
    ConditionalCheckFailed,
    UnexpectedDeserializationError,
    UnlikelyAllNonceGettersFailed,
    UpdateFailed,
}
impl From<DynamoDbError> for NonceManagerError {
    fn from(e: DynamoDbError) -> Self {
        match e {
            DynamoDbError::GenericRequestFailed => Self::UpdateFailed,
            DynamoDbError::ConditionalCheckFailed => Self::ConditionalCheckFailed,
        }
    }
}

type Result<T> = core::result::Result<T, NonceManagerError>;

pub struct NonceManager {
    api: DynamoDbApi,
    request_factory: DynamoDbNonceRequestFactory,
    pub millis_since_epoch: MillisSinceEpoch,
}

impl NonceManager {
    pub fn new(
        dynamodb_access_key: String,
        dynamodb_secret_key: String,
        chain_name: &str,
        millis_since_epoch: MillisSinceEpoch,
    ) -> Self {
        let key = format!("chainstate_{chain_name}");
        Self {
            api: DynamoDbApi::new(dynamodb_access_key, dynamodb_secret_key),
            request_factory: DynamoDbNonceRequestFactory {
                table_name: DYNAMODB_TABLE_NONCE,
                key: key.to_string(),
            },
            millis_since_epoch,
        }
    }

    // Tries each of our 'cases' in sequence (roughly from most to least likely).
    // It is theoretically possible for no nonce to be found (since different cases
    // are non-atomic though an individual case is an atomic transaction), but
    // very unlikely
    pub fn get_nonce(
        &self,
        exec_step_uuid: &Uuid,
        cur_block: BlockNum,
        system_nonce: Nonce,
    ) -> Result<Nonce> {
        if let Ok(nonce) = self.attempt_cold_start(exec_step_uuid, cur_block, system_nonce) {
            ink_env::debug_println!("Nonce retrieved from cold start");
            Ok(nonce)
        } else if let Ok(nonce) = self.attempt_next_nonce(exec_step_uuid, cur_block) {
            ink_env::debug_println!("Nonce retrieved from NextNonce");
            Ok(nonce)
        } else if let Ok(nonce) = self.attempt_existing_assignment(exec_step_uuid) {
            ink_env::debug_println!("Nonce retrieved from existing assignment");
            Ok(nonce)
        } else if let Ok(nonce) = self.attempt_reclaim_dropped_nonce(exec_step_uuid, cur_block) {
            ink_env::debug_println!("Nonce retrieved from dropped nonce");
            Ok(nonce)
        } else {
            Err(NonceManagerError::UnlikelyAllNonceGettersFailed)
        }
    }

    pub fn finalize_execstep(&self, exec_step_uuid: &Uuid, cur_block: BlockNum) -> Result<()> {
        let request_payload = self
            .request_factory
            .process_finalized_step_request(exec_step_uuid, cur_block);
        self.api
            .dynamodb_request(
                self.millis_since_epoch,
                request_payload.as_bytes(),
                DynamoDbAction::UpdateItem,
            )
            .map_or_else(
                |dynamodb_err| Err(NonceManagerError::from(dynamodb_err)),
                // We discard the response because we had set return_values to None
                |_response| Ok(()),
            )
    }

    pub fn drop_execstep(&self, exec_step_uuid: &Uuid, dropped_nonce: Nonce) -> Result<()> {
        let request_payload = self
            .request_factory
            .process_dropped_step_request(exec_step_uuid, dropped_nonce);
        self.api
            .dynamodb_request(
                self.millis_since_epoch,
                request_payload.as_bytes(),
                DynamoDbAction::UpdateItem,
            )
            .map_or_else(
                |dynamodb_err| Err(NonceManagerError::from(dynamodb_err)),
                // We discard the response because we had set return_values to None
                |_response| Ok(()),
            )
    }

    // More convenient interface than the above but it requires two lookups
    pub fn drop_execstep_from_id(&self, exec_step_uuid: &Uuid) -> Result<()> {
        let dropped_nonce = self.attempt_existing_assignment(exec_step_uuid)?;
        self.drop_execstep(exec_step_uuid, dropped_nonce)
    }

    fn attempt_cold_start(
        &self,
        exec_step_uuid: &Uuid,
        cur_block: BlockNum,
        system_nonce: Nonce,
    ) -> Result<Nonce> {
        let request_payload =
            self.request_factory
                .cold_start_request(exec_step_uuid, cur_block, system_nonce);
        self.api
            .dynamodb_request(
                self.millis_since_epoch,
                request_payload.as_bytes(),
                DynamoDbAction::UpdateItem,
            )
            .map_or_else(
                |dynamodb_err| Err(NonceManagerError::from(dynamodb_err)),
                // We discard the response because we had set return_values to None
                |_response| Ok(system_nonce),
            )
    }

    fn attempt_next_nonce(&self, exec_step_uuid: &Uuid, cur_block: BlockNum) -> Result<Nonce> {
        let request_payload = self
            .request_factory
            .next_nonce_request(exec_step_uuid, cur_block);
        let updated_block_nonce_next_response = self
            .api
            .dynamodb_request(
                self.millis_since_epoch,
                request_payload.as_bytes(),
                DynamoDbAction::UpdateItem,
            )
            .map_err(|dynamodb_err| NonceManagerError::from(dynamodb_err))?;

        let (decoded, _): (AttributesWrapper<PendingNonceBlockNextResponse>, usize) =
            serde_json_core::from_slice(&updated_block_nonce_next_response)
                .map_err(|_| NonceManagerError::UnexpectedDeserializationError)?;
        Ok(decoded.Attributes.ExecStepPendingNonce.M.num.N)
    }

    fn attempt_existing_assignment(&self, exec_step_uuid: &Uuid) -> Result<Nonce> {
        let request_payload = self
            .request_factory
            .existing_assignment_request(exec_step_uuid);
        let get_block_nonce_response = self
            .api
            .dynamodb_request(
                self.millis_since_epoch,
                request_payload.as_bytes(),
                DynamoDbAction::GetItem,
            )
            .map_err(|dynamodb_err| NonceManagerError::from(dynamodb_err))?;

        // We return a ConditionCheckFail for uniformity even though this is a GET_ITEM request
        // Sort of hacky but we try the item deserialization and then the empty one. ORDER MATTERS
        // The empty deserialization works on a valid object also
        if let Ok((decoded, _)) = serde_json_core::from_slice::<
            ItemWrapper<PendingNonceBlockResponse>,
        >(&get_block_nonce_response)
        {
            Ok(decoded.Item.ExecStepPendingNonce.M.num.N)
        } else {
            let (_decoded, _): (ItemWrapper<Empty>, usize) =
                serde_json_core::from_slice(&get_block_nonce_response)
                    .map_err(|_| NonceManagerError::UnexpectedDeserializationError)?;
            Err(NonceManagerError::ConditionalCheckFailed)
        }
    }

    fn attempt_reclaim_dropped_nonce(
        &self,
        exec_step_uuid: &Uuid,
        cur_block: BlockNum,
    ) -> Result<Nonce> {
        let request_payload = self
            .request_factory
            .reclaim_dropped_nonce_request(exec_step_uuid, cur_block);
        let reclaim_dropped_nonce_response = self
            .api
            .dynamodb_request(
                self.millis_since_epoch,
                request_payload.as_bytes(),
                DynamoDbAction::UpdateItem,
            )
            .map_err(|dynamodb_err| NonceManagerError::from(dynamodb_err))?;

        let (decoded, _): (AttributesWrapper<PendingNonceBlockResponse>, usize) =
            serde_json_core::from_slice(&reclaim_dropped_nonce_response)
                .map_err(|_| NonceManagerError::UnexpectedDeserializationError)?;
        Ok(decoded.Attributes.ExecStepPendingNonce.M.num.N)
    }
}

#[cfg(feature = "dynamodb-live-test")]
#[cfg(feature = "std")]
#[cfg(test)]
mod nonce_manager_tests {
    use ink_env::debug_println;

    use super::*;

    fn now_millis() -> u64 {
        use std::time::SystemTime;
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .try_into()
            .unwrap()
    }

    fn nonce_manager() -> NonceManager {
        let dynamodb_access_key =
            std::env::var("DYNAMODB_ACCESS_KEY").expect("Env var DYNAMODB_ACCESS_KEY is not set");
        let dynamodb_secret_key =
            std::env::var("DYNAMODB_SECRET_KEY").expect("Env var DYNAMODB_SECRET_KEY is not set");
        let chain_name = "astar";
        let millis_since_epoch = now_millis();

        NonceManager::new(
            dynamodb_access_key,
            dynamodb_secret_key,
            chain_name,
            millis_since_epoch,
        )
    }

    #[test]
    fn test_cold_start() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let res = nonce_manager().attempt_cold_start(&Uuid::new([1u8; 16]), 10_000, 40);
        assert!(res.is_ok() || res == Err(NonceManagerError::ConditionalCheckFailed));
        debug_println!("[Expected] Cold start attempt: {:?}", res);
    }

    #[test]
    fn test_next_nonce() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let res = nonce_manager().attempt_next_nonce(&Uuid::new([2u8; 16]), 10_000);
        assert!(res.is_ok() || res == Err(NonceManagerError::ConditionalCheckFailed));
        debug_println!("[Expected] Next nonce attempt: {:?}", res);
    }

    #[test]
    fn test_existing_assignment() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let res = nonce_manager().attempt_existing_assignment(&Uuid::new([2u8; 16]));
        assert!(res.is_ok() || res == Err(NonceManagerError::ConditionalCheckFailed));
        debug_println!("[Expected] Existing assignment attempt: {:?}", res);
    }

    #[test]
    fn test_reclaim_dropped_nonce() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let res = nonce_manager().attempt_reclaim_dropped_nonce(&Uuid::new([4u8; 16]), 10_000);
        assert!(res.is_ok() || res == Err(NonceManagerError::ConditionalCheckFailed));
        debug_println!("[Expected] Reclaim dropped nonce attempt: {:?}", res);
    }

    // This is a time-consuming test (takes 3-4 seconds) so we filter it out
    #[test]
    #[ignore]
    fn test_get_nonce() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let nonce_manager = nonce_manager();
        for uuid in 1u8..11u8 {
            let res = nonce_manager
                .get_nonce(
                    &Uuid::new([uuid; 16]),
                    10_000 + BlockNum::from(uuid),
                    50, // system nonce
                )
                .expect("get nonce must succeed in a single-worker setting");
            debug_println!("Get nonce: {:?}", res);
        }
    }

    #[test]
    fn test_finalize_execstep() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let res = nonce_manager()
            .finalize_execstep(&Uuid::new([4u8; 16]), 10_000)
            .expect("Unconditional update should succeed");
        debug_println!("Finalize execution step: {:?}", res);
    }

    #[test]
    fn test_drop_execstep() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let res = nonce_manager().drop_execstep(&Uuid::new([6u8; 16]), 55);
        assert!(res.is_ok() || res == Err(NonceManagerError::ConditionalCheckFailed));
        debug_println!("Drop execution step: {:?}", res);
    }
}
