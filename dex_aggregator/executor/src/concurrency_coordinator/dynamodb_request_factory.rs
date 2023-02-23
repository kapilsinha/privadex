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
use privadex_chain_metadata::common::{BlockNum, EthTxnHash, MillisSinceEpoch, Nonce};
use privadex_common::{utils::general_utils::slice_to_hex_string, uuid::Uuid};

// One per chain
pub(super) struct DynamoDbNonceRequestFactory {
    pub table_name: &'static str,
    pub key: String,
}

// One overall (across all chains)
pub(super) struct DynamoDbExecPlanRequestFactory {
    pub table_name: &'static str,
    pub key: String,
}

// One overall (across all chains)
pub(super) struct DynamoDbPrestartTxnsRequestFactory {
    pub table_name: &'static str,
    pub key: String,
}

impl DynamoDbNonceRequestFactory {
    // Case 1: Cold start / cleanup
    // When: IsPendingTxnsEmpty (and thus !IsExecutionStepAssigned)
    pub fn cold_start_request(
        &self,
        exec_step_uuid: &Uuid,
        cur_block: BlockNum,
        system_nonce: Nonce,
    ) -> String {
        let exec_step_attr = self.get_exec_step_attribute(exec_step_uuid);
        let self_assigned_nonce = system_nonce;
        let next_nonce = system_nonce + 1;
        format!(r#"{{"TableName": "{}", "Key": {{"id": {{"S": "{}"}}}}, "ReturnValues": "NONE", "UpdateExpression": "SET BlockAtLastConfirmedNonce = :curblock, DroppedNonces = :emptylist, ExecStepPendingNonce = :pendingnonce, ExecStepPendingBlockAdded = :pendingblockadded, NextNonce = :nextnonce", "ConditionExpression": "size(ExecStepPendingNonce) = :zero", "ExpressionAttributeValues": {{":curblock": {{"N": "{cur_block}"}}, ":emptylist": {{"L": []}}, ":pendingnonce": {{"M": {{"{exec_step_attr}": {{"N": "{self_assigned_nonce}"}}}}}}, ":pendingblockadded": {{"M": {{"{exec_step_attr}": {{"N": "{cur_block}"}}}}}}, ":nextnonce": {{"N": "{next_nonce}"}}, ":zero": {{"N": "0"}}}}}}"#, self.table_name, self.key).to_string()
    }

    // Case 2: Assign the next nonce
    // When: !IsExecutionStepAssigned AND IsDroppedNoncesEmpty AND !IsPendingTxnsEmpty
    pub fn next_nonce_request(&self, exec_step_uuid: &Uuid, cur_block: BlockNum) -> String {
        let exec_step_attr = self.get_exec_step_attribute(exec_step_uuid);
        format!(r#"{{"TableName": "{}", "Key": {{"id": {{"S": "{}"}}}}, "ReturnValues": "UPDATED_NEW", "UpdateExpression": "SET ExecStepPendingBlockAdded.{exec_step_attr} = :curblock, ExecStepPendingNonce.{exec_step_attr} = NextNonce, NextNonce = NextNonce + :one", "ConditionExpression": "attribute_not_exists(ExecStepPendingNonce.{exec_step_attr}) AND size(DroppedNonces) = :zero AND size(ExecStepPendingNonce) > :zero", "ExpressionAttributeValues": {{":curblock": {{"N": "{cur_block}"}}, ":one": {{"N": "1"}}, ":zero": {{"N": "0"}}}}}}"#, self.table_name, self.key,).to_string()
    }

    // Case 3: Pull the existing assignment for the ExecutionStep
    // When: IsExecutionStepAssigned (and thus !IsPendingTxnsEmpty)
    pub fn existing_assignment_request(&self, exec_step_uuid: &Uuid) -> String {
        let exec_step_attr = self.get_exec_step_attribute(exec_step_uuid);
        format!(r#"{{"TableName": "{}", "Key": {{"id": {{"S": "{}"}}}}, "ProjectionExpression": "ExecStepPendingNonce.{exec_step_attr}, ExecStepPendingBlockAdded.{exec_step_attr}"}}"#, self.table_name, self.key,).to_string()
    }

    // Case 4: Reclaim a dropped transaction's nonce
    // When: !IsExecutionStepAssigned AND !IsDroppedNoncesEmpty AND !IsPendingTxnsEmpty
    pub fn reclaim_dropped_nonce_request(
        &self,
        exec_step_uuid: &Uuid,
        cur_block: BlockNum,
    ) -> String {
        let exec_step_attr = self.get_exec_step_attribute(exec_step_uuid);
        format!(r#"{{"TableName": "{}", "Key": {{"id": {{"S": "{}"}}}}, "ReturnValues": "UPDATED_NEW", "UpdateExpression": "SET ExecStepPendingBlockAdded.{exec_step_attr} = :curblock, ExecStepPendingNonce.{exec_step_attr} = DroppedNonces[0] REMOVE DroppedNonces[0]", "ConditionExpression": "attribute_not_exists(ExecStepPendingNonce.{exec_step_attr}) AND size(DroppedNonces) > :zero AND size(ExecStepPendingNonce) > :zero", "ExpressionAttributeValues": {{":curblock": {{"N": "{cur_block}"}}, ":zero": {{"N": "0"}}}}}}"#, self.table_name, self.key,).to_string()
    }

    // For every case: A transaction has been finalized
    pub fn process_finalized_step_request(
        &self,
        exec_step_uuid: &Uuid,
        cur_block: BlockNum,
    ) -> String {
        let exec_step_attr = self.get_exec_step_attribute(exec_step_uuid);
        format!(r#"{{"TableName": "{}", "Key": {{"id": {{"S": "{}"}}}}, "ReturnValues": "NONE", "UpdateExpression": "SET BlockAtLastConfirmedNonce = :curblock REMOVE ExecStepPendingBlockAdded.{exec_step_attr}, ExecStepPendingNonce.{exec_step_attr}", "ExpressionAttributeValues": {{":curblock": {{"N": "{cur_block}"}}}}}}"#, self.table_name, self.key,).to_string()
    }

    // For every case: A transaction has been dropped
    pub fn process_dropped_step_request(
        &self,
        exec_step_uuid: &Uuid,
        dropped_nonce: Nonce,
    ) -> String {
        let exec_step_attr = self.get_exec_step_attribute(exec_step_uuid);
        format!(r#"{{"TableName": "{}", "Key": {{"id": {{"S": "{}"}}}}, "ReturnValues": "NONE", "UpdateExpression": "SET DroppedNonces = list_append(DroppedNonces, :droppednonce) REMOVE ExecStepPendingBlockAdded.{exec_step_attr}, ExecStepPendingNonce.{exec_step_attr}", "ConditionExpression": "attribute_exists(ExecStepPendingBlockAdded.{exec_step_attr})", "ExpressionAttributeValues": {{":droppednonce": {{"L": [{{"N": "{dropped_nonce}"}}]}}}}}}"#,
        self.table_name, self.key,).to_string()
    }

    fn get_exec_step_attribute(&self, exec_step_uuid: &Uuid) -> String {
        format!("execstep_{}", exec_step_uuid.to_hex_string())
    }
}

impl DynamoDbExecPlanRequestFactory {
    // Allocate a plan to a worker
    // When: isallocated = false OR updateepochmillis is old (1 minute)
    pub fn allocate_execplan_request(
        &self,
        exec_plan_uuid: &Uuid,
        now_epoch_millis: MillisSinceEpoch,
    ) -> String {
        let execplan_hex_str = exec_plan_uuid.to_hex_string();
        let exec_plan_attr = self.get_exec_plan_attribute(exec_plan_uuid);
        // If the ExecutionPlan is still allocateed but its timestamp is over a minute ago, then we allocate to it
        // (we assume the worker that it was allocated to has died)
        let min_epoch_millis = now_epoch_millis - 60_000;
        format!(r#"{{"TableName": "{}", "Key": {{"id": {{"S": "{}"}}}}, "ReturnValues": "NONE", "UpdateExpression": "SET WorkerIsAllocated.{exec_plan_attr} = :true, WorkerAssignmentUpdateEpochMillis.{exec_plan_attr} = :epochmillis ADD Plans :plan", "ConditionExpression": "WorkerIsAllocated.{exec_plan_attr} <> :true OR WorkerAssignmentUpdateEpochMillis.{exec_plan_attr} < :minepochmillis", "ExpressionAttributeValues": {{":true": {{"BOOL": true}}, ":epochmillis": {{"N": "{now_epoch_millis}"}}, ":plan": {{"SS": ["{execplan_hex_str}"]}}, ":minepochmillis": {{"N": "{min_epoch_millis}"}}}}}}"#, self.table_name, self.key,).to_string()
    }

    // Unallocate a plan from a worker
    // When: Unconditional update. No other worker can allocate before this
    // (and only a worker that has allocated last should unallocate)
    pub fn unallocate_execplan_request(
        &self,
        exec_plan_uuid: &Uuid,
        now_epoch_millis: MillisSinceEpoch,
    ) -> String {
        let execplan_hex_str = exec_plan_uuid.to_hex_string();
        let exec_plan_attr = self.get_exec_plan_attribute(exec_plan_uuid);
        format!(r#"{{"TableName": "{}", "Key": {{"id": {{"S": "{}"}}}}, "ReturnValues": "NONE", "UpdateExpression": "SET WorkerIsAllocated.{exec_plan_attr} = :false, WorkerAssignmentUpdateEpochMillis.{exec_plan_attr} = :epochmillis ADD Plans :plan", "ExpressionAttributeValues": {{":false": {{"BOOL": false}}, ":epochmillis": {{"N": "{now_epoch_millis}"}}, ":plan": {{"SS": ["{execplan_hex_str}"]}}}}}}"#, self.table_name, self.key,).to_string()
    }

    // Remove exec plan from processing queue
    pub fn remove_completed_execplan_request(&self, exec_plan_uuid: &Uuid) -> String {
        let execplan_hex_str = exec_plan_uuid.to_hex_string();
        let exec_plan_attr = self.get_exec_plan_attribute(exec_plan_uuid);
        format!(r#"{{"TableName": "{}", "Key": {{"id": {{"S": "{}"}}}}, "ReturnValues": "NONE", "UpdateExpression": "REMOVE WorkerIsAllocated.{exec_plan_attr}, WorkerAssignmentUpdateEpochMillis.{exec_plan_attr} DELETE Plans :plan", "ExpressionAttributeValues": {{":plan": {{"SS": ["{execplan_hex_str}"]}}}}}}"#, self.table_name, self.key,).to_string()
    }

    pub fn get_execplan_ids(&self) -> String {
        format!(r#"{{"TableName": "{}", "Key": {{"id": {{"S": "{}"}}}}, "ProjectionExpression": "Plans"}}"#,
        self.table_name, self.key,).to_string()
    }

    fn get_exec_plan_attribute(&self, exec_plan_uuid: &Uuid) -> String {
        format!("execplan_{}", exec_plan_uuid.to_hex_string())
    }
}

impl DynamoDbPrestartTxnsRequestFactory {
    // Add a prestart txn hash to an ever-growing set if it doesn't exist (Note that this really should per chain but the odds
    // of identical txn hashes on different chains is virtually zero)
    pub fn add_prestart_txn(
        &self,
        txn_hash: &EthTxnHash,
        now_epoch_millis: MillisSinceEpoch,
    ) -> String {
        let txn_hash_str = slice_to_hex_string(&txn_hash.0);
        format!(r#"{{"TableName": "{}", "Key": {{"id": {{"S": "{}"}}}}, "ReturnValues": "NONE", "UpdateExpression": "SET LastUpdateEpochMillis = :epochmillis ADD TxnHash :txnhashset", "ConditionExpression": "NOT contains(TxnHash, :txnhash)", "ExpressionAttributeValues": {{":epochmillis": {{"N": "{now_epoch_millis}"}}, ":txnhashset": {{"SS": ["{txn_hash_str}"]}}, ":txnhash": {{"S": "{txn_hash_str}"}}}}}}"#, self.table_name, self.key,).to_string()
    }
}

#[cfg(test)]
mod request_factory_tests {
    use ink_env::debug_println;

    use super::*;

    #[test]
    fn test_print_query() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let nonce_factory = DynamoDbNonceRequestFactory {
            table_name: "privadex_phat_contract",
            key: "chainstate_astar".into(),
        };
        let cur_block = 1_000_000;
        let system_nonce = 50;
        let x = nonce_factory.cold_start_request(&Uuid::new([1u8; 16]), cur_block, system_nonce);
        debug_println!("{}", x);
    }
}
