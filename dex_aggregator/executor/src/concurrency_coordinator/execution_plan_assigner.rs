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
    vec::Vec,
};

use privadex_chain_metadata::common::MillisSinceEpoch;
use privadex_common::{
    utils::dynamodb_api::{DynamoDbAction, DynamoDbApi, DynamoDbError},
    uuid::Uuid,
};

use super::{
    deserialize_helper::{ExecPlanIdsWrapper, ItemWrapper},
    dynamodb_request_factory::DynamoDbExecPlanRequestFactory,
};

const DYNAMODB_TABLE_EXECPLAN: &'static str = "privadex_phat_contract";
const DYNAMODB_TABLE_KEY: &'static str = "execplans";

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ExecutionPlanAssignerError {
    ConditionalCheckFailed,
    UnexpectedDeserializationError,
    UpdateFailed,
}
impl From<DynamoDbError> for ExecutionPlanAssignerError {
    fn from(e: DynamoDbError) -> Self {
        match e {
            DynamoDbError::GenericRequestFailed => Self::UpdateFailed,
            DynamoDbError::ConditionalCheckFailed => Self::ConditionalCheckFailed,
        }
    }
}

type Result<T> = core::result::Result<T, ExecutionPlanAssignerError>;

pub struct ExecutionPlanAssigner {
    api: DynamoDbApi,
    request_factory: DynamoDbExecPlanRequestFactory,
    pub millis_since_epoch: MillisSinceEpoch,
}

impl ExecutionPlanAssigner {
    pub fn new(
        dynamodb_access_key: String,
        dynamodb_secret_key: String,
        millis_since_epoch: MillisSinceEpoch,
    ) -> Self {
        Self {
            api: DynamoDbApi::new(dynamodb_access_key, dynamodb_secret_key),
            request_factory: DynamoDbExecPlanRequestFactory {
                table_name: DYNAMODB_TABLE_EXECPLAN,
                key: DYNAMODB_TABLE_KEY.to_string(),
            },
            millis_since_epoch,
        }
    }

    pub fn attempt_allocate_exec_plan(&self, exec_plan_uuid: &Uuid) -> Result<bool> {
        let request_payload = self
            .request_factory
            .allocate_execplan_request(exec_plan_uuid, self.millis_since_epoch);
        self.api
            .dynamodb_request(
                self.millis_since_epoch,
                request_payload.as_bytes(),
                DynamoDbAction::UpdateItem,
            )
            .map_or_else(
                |dynamodb_err| {
                    let err = ExecutionPlanAssignerError::from(dynamodb_err);
                    match err {
                        ExecutionPlanAssignerError::ConditionalCheckFailed => Ok(false),
                        _ => Err(err),
                    }
                },
                // We discard the response because we had set return_values to None
                |_response| Ok(true),
            )
    }

    pub fn unallocate_exec_plan(&self, exec_plan_uuid: &Uuid) -> Result<()> {
        let request_payload = self
            .request_factory
            .unallocate_execplan_request(exec_plan_uuid, self.millis_since_epoch);
        self.api
            .dynamodb_request(
                self.millis_since_epoch,
                request_payload.as_bytes(),
                DynamoDbAction::UpdateItem,
            )
            .map_or_else(
                |dynamodb_err| Err(ExecutionPlanAssignerError::from(dynamodb_err)),
                // We discard the response because we had set return_values to None
                |_response| Ok(()),
            )
    }

    pub fn remove_completed_execplan(&self, exec_plan_uuid: &Uuid) -> Result<()> {
        let request_payload = self
            .request_factory
            .remove_completed_execplan_request(exec_plan_uuid);
        self.api
            .dynamodb_request(
                self.millis_since_epoch,
                request_payload.as_bytes(),
                DynamoDbAction::UpdateItem,
            )
            .map_or_else(
                |dynamodb_err| Err(ExecutionPlanAssignerError::from(dynamodb_err)),
                // We discard the response because we had set return_values to None
                |_response| Ok(()),
            )
    }

    // Below functions are more useful for the driver/scheduler

    pub fn register_exec_plan(&self, exec_plan_uuid: &Uuid) -> Result<()> {
        self.unallocate_exec_plan(exec_plan_uuid)
    }

    pub fn get_execplan_ids(&self) -> Result<Vec<Uuid>> {
        let request_payload = self.request_factory.get_execplan_ids();
        let get_exec_plan_ids_response = self
            .api
            .dynamodb_request(
                self.millis_since_epoch,
                request_payload.as_bytes(),
                DynamoDbAction::GetItem,
            )
            .map_err(|dynamodb_err| ExecutionPlanAssignerError::from(dynamodb_err))?;

        let (decoded, _): (ItemWrapper<ExecPlanIdsWrapper>, usize) =
            serde_json_core::from_slice(&get_exec_plan_ids_response)
                .map_err(|_| ExecutionPlanAssignerError::UnexpectedDeserializationError)?;

        Ok(decoded
            .Item
            .Plans
            .SS
            .into_iter()
            .map(|uuid_container| uuid_container.0)
            .collect())
    }
}

#[cfg(feature = "dynamodb-live-test")]
#[cfg(feature = "std")]
#[cfg(test)]
mod execution_plan_assigner_tests {
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

    fn exec_plan_assigner() -> ExecutionPlanAssigner {
        let dynamodb_access_key =
            std::env::var("DYNAMODB_ACCESS_KEY").expect("Env var DYNAMODB_ACCESS_KEY is not set");
        let dynamodb_secret_key =
            std::env::var("DYNAMODB_SECRET_KEY").expect("Env var DYNAMODB_SECRET_KEY is not set");
        let millis_since_epoch = now_millis();

        ExecutionPlanAssigner::new(dynamodb_access_key, dynamodb_secret_key, millis_since_epoch)
    }

    #[test]
    fn test_allocate_execplan() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let res = exec_plan_assigner()
            .attempt_allocate_exec_plan(&Uuid::new([1u8; 16]))
            .expect("Database write error");
        debug_println!("Allocate ExecutionPlan attempt: success = {:?}", res);
    }

    #[test]
    fn test_unallocate_execplan() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let _ = exec_plan_assigner()
            .unallocate_exec_plan(&Uuid::new([1u8; 16]))
            .expect("Database write error");
        debug_println!("Unallocated ExecutionPlan");
    }

    #[test]
    fn test_remove_completed_assignment() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let _ = exec_plan_assigner()
            .remove_completed_execplan(&Uuid::new([1u8; 16]))
            .expect("Database write error");
        debug_println!("Removed ExecutionPlan");
    }

    #[test]
    fn test_get_exec_plan_ids() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let exec_plan_uuids = exec_plan_assigner()
            .get_execplan_ids()
            .expect("Database access/connection error");
        debug_println!("Active ExecutionPlans: {:?}", exec_plan_uuids);
    }
}
