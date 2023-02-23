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

use ink_prelude::string::{String, ToString};

use privadex_chain_metadata::common::{EthTxnHash, MillisSinceEpoch};
use privadex_common::utils::dynamodb_api::{DynamoDbAction, DynamoDbApi, DynamoDbError};

use super::dynamodb_request_factory::DynamoDbPrestartTxnsRequestFactory;

const DYNAMODB_TABLE_EXECPLAN: &'static str = "privadex_phat_contract";
const DYNAMODB_TABLE_KEY: &'static str = "prestart_txns";

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum PrestartStepUniquenessEnforcerError {
    ConditionalCheckFailed,
    UnexpectedDeserializationError,
    UpdateFailed,
}
impl From<DynamoDbError> for PrestartStepUniquenessEnforcerError {
    fn from(e: DynamoDbError) -> Self {
        match e {
            DynamoDbError::GenericRequestFailed => Self::UpdateFailed,
            DynamoDbError::ConditionalCheckFailed => Self::ConditionalCheckFailed,
        }
    }
}

type Result<T> = core::result::Result<T, PrestartStepUniquenessEnforcerError>;

pub struct PrestartStepUniquenessEnforcer {
    api: DynamoDbApi,
    request_factory: DynamoDbPrestartTxnsRequestFactory,
    pub millis_since_epoch: MillisSinceEpoch,
}

impl PrestartStepUniquenessEnforcer {
    pub fn new(
        dynamodb_access_key: String,
        dynamodb_secret_key: String,
        millis_since_epoch: MillisSinceEpoch,
    ) -> Self {
        Self {
            api: DynamoDbApi::new(dynamodb_access_key, dynamodb_secret_key),
            request_factory: DynamoDbPrestartTxnsRequestFactory {
                table_name: DYNAMODB_TABLE_EXECPLAN,
                key: DYNAMODB_TABLE_KEY.to_string(),
            },
            millis_since_epoch,
        }
    }

    // true is good, we registered it. false is bad, it existed before (potential malicious user)
    pub fn attempt_register_prestart_txn(&self, txn_hash: &EthTxnHash) -> Result<bool> {
        let request_payload = self
            .request_factory
            .add_prestart_txn(txn_hash, self.millis_since_epoch);
        self.api
            .dynamodb_request(
                self.millis_since_epoch,
                request_payload.as_bytes(),
                DynamoDbAction::UpdateItem,
            )
            .map_or_else(
                |dynamodb_err| {
                    let err = PrestartStepUniquenessEnforcerError::from(dynamodb_err);
                    match err {
                        PrestartStepUniquenessEnforcerError::ConditionalCheckFailed => Ok(false),
                        _ => Err(err),
                    }
                },
                // We discard the response because we had set return_values to None
                |_response| Ok(true),
            )
    }
}

#[cfg(feature = "dynamodb-live-test")]
#[cfg(feature = "std")]
#[cfg(test)]
mod prestart_step_uniqueness_enforcer_tests {
    use hex_literal::hex;

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

    fn prestart_step_uniqueness_enforcer() -> PrestartStepUniquenessEnforcer {
        let dynamodb_access_key =
            std::env::var("DYNAMODB_ACCESS_KEY").expect("Env var DYNAMODB_ACCESS_KEY is not set");
        let dynamodb_secret_key =
            std::env::var("DYNAMODB_SECRET_KEY").expect("Env var DYNAMODB_SECRET_KEY is not set");
        let millis_since_epoch = now_millis();

        PrestartStepUniquenessEnforcer::new(
            dynamodb_access_key,
            dynamodb_secret_key,
            millis_since_epoch,
        )
    }

    #[test]
    fn test_register_prestart_txn() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let txn_hash = EthTxnHash {
            0: hex!("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"),
        };
        let _res1 = prestart_step_uniqueness_enforcer()
            .attempt_register_prestart_txn(&txn_hash)
            .expect("Database update 1 failed");
        // assert_eq!(res1, true); // commented out so we don't need to manually clean up the db after the test
        // The second time the transaction hash is added, we should hit a conditional check error
        let res2 = prestart_step_uniqueness_enforcer()
            .attempt_register_prestart_txn(&txn_hash)
            .expect("Database update 2 failed");
        assert_eq!(res2, false);
    }
}
