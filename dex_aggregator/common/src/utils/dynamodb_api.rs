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
    vec,
    vec::Vec,
};
use scale::{Decode, Encode};

// To make HTTP requests
use pink_extension::http_post;

// To generate AWS4 Signature
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

// To format block timestamp for http request headers
use chrono::{TimeZone, Utc};

// To encrypt/decrypt HTTP payloads
use base16;

// This file uses https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Programming.LowLevelAPI.html
// as a reference (but frankly it isn't very useful). Took lots of trial and error
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct DynamoDbApi {
    access_key: String,
    secret_key: String,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum DynamoDbError {
    GenericRequestFailed,
    ConditionalCheckFailed,
}
const CONDITIONAL_CHECK_MESSAGE: &'static str = "ConditionalCheckFailedException";

pub enum DynamoDbAction {
    GetItem,
    UpdateItem,
}

impl ToString for DynamoDbAction {
    fn to_string(&self) -> String {
        match self {
            Self::GetItem => "GetItem".into(),
            Self::UpdateItem => "UpdateItem".into(),
        }
    }
}

impl DynamoDbApi {
    pub fn new(access_key: String, secret_key: String) -> Self {
        Self {
            access_key,
            secret_key,
        }
    }

    fn get_time(&self, timestamp_millis: u64) -> (String, String) {
        // Get block time (UNIX time in nano seconds)and convert to Utc datetime object
        let datetime = Utc
            .timestamp_millis_opt(timestamp_millis.try_into().unwrap())
            .unwrap();

        // Format both date and datetime for AWS4 signature
        let datestamp = datetime.format("%Y%m%d").to_string();
        let datetimestamp = datetime.format("%Y%m%dT%H%M%SZ").to_string();

        // ink_env::debug_println!("Time: {}, {}", datestamp, datetimestamp);

        (datestamp, datetimestamp)
    }

    /// Encrypts and HTTP PUTs the data to the specified storage platform as byte stream.
    /// Must seal the correct credentials before calling this function.
    pub fn dynamodb_request(
        &self,
        timestamp_millis: u64,
        payload: &[u8],
        action: DynamoDbAction,
    ) -> Result<Vec<u8>, DynamoDbError> {
        // Set request values
        let method = "POST";
        let service = "dynamodb";
        let region = "us-west-2";

        let host = format!("{}.{}.amazonaws.com", service, region);

        let content_length = format!("{}", payload.len());
        let payload_hash = format!("{:x}", Sha256::digest(payload));

        // Get datestamp (20220727) and amz_date (20220727T141618Z)
        let (datestamp, amz_date) = self.get_time(timestamp_millis);

        // 1. Create canonical request
        let canonical_uri = "/".to_string(); // / HTTP/1.1
        let canonical_querystring = "";
        let content_type = "content-type:application/x-amz-json-1.0";
        let target = format!("DynamoDB_20120810.{}", action.to_string());
        let canonical_headers = format!(
            "host:{}\nx-amz-date:{}\nx-amz-target:{}\n",
            host, amz_date, target
        );

        let signed_headers = "content-type;host;x-amz-date;x-amz-target";
        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n{}",
            method,
            canonical_uri,
            canonical_querystring,
            content_type, // added by me
            canonical_headers,
            signed_headers,
            payload_hash
        );

        // ink_env::debug_println!(" ----- Canonical request -----  \n{}\n", canonical_request);
        //  ----- Canonical request -----
        // PUT
        // /test/api-upload
        //
        // host:fat-contract-s3-sync.s3.amazonaws.com
        // x-amz-content-sha256:505f2ec6d688d6e15f718b5c91edd07c45310e08e8c221018a7c0f103515fa28
        // x-amz-date:19700101T000000Z
        //
        // host;x-amz-content-sha256;x-amz-date
        // 505f2ec6d688d6e15f718b5c91edd07c45310e08e8c221018a7c0f103515fa28

        // 2. Create string to sign
        let algorithm = "AWS4-HMAC-SHA256";
        let credential_scope = format!("{}/{}/{}/aws4_request", datestamp, region, service);
        let canonical_request_hash = format!("{:x}", Sha256::digest(&canonical_request.as_bytes()));
        let string_to_sign = format!(
            "{}\n{}\n{}\n{}",
            algorithm, amz_date, credential_scope, canonical_request_hash
        );

        // ink_env::debug_println!(" ----- String to sign ----- \n{}\n", string_to_sign);
        //  ----- String to sign -----
        // AWS4-HMAC-SHA256
        // 19700101T000000Z
        // 19700101/ap-southeast-1/s3/aws4_request
        // efd07a6d8013f3c35d4c3d6b7f52f86ae682c51a8639fe80b8f68198107e3039

        // 3. Calculate signature
        let signature_key = get_signature_key(
            self.secret_key.as_bytes(),
            &datestamp.as_bytes(),
            &region.as_bytes(),
            &service.as_bytes(),
        );
        let signature_bytes = hmac_sign(&signature_key, &string_to_sign.as_bytes());
        let signature = format!("{}", base16::encode_lower(&signature_bytes));

        // ink_env::debug_println!(" ----- Signature ----- \n{}\n", &signature);
        //  ----- Signature -----
        // 84bf2db9f7a0007f5124cf2e9c0e1b7e1cec2b1b1b209ab9458387caa3b8da52

        // 4. Create authorization header
        let authorization_header = format!(
            "{} Credential={}/{},SignedHeaders={},Signature={}",
            algorithm, self.access_key, credential_scope, signed_headers, signature
        );

        // ink_env::debug_println!(
        //     " ----- Authorization header ----- \nAuthorization: {}\n",
        //     &authorization_header
        // );
        //  ----- Authorization header -----
        // Authorization: AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/19700101/ap-southeast-1/s3/aws4_request,SignedHeaders=host;x-amz-content-sha256;x-amz-date,Signature=b9b6bcb29b1369678e3a3cfae411a5277c084c8c1796bb6e78407f402f9e3f3d
        let request_url: String = format!("https://{}.{}.amazonaws.com", service, region);

        let headers: Vec<(String, String)> = vec![
            ("Host".into(), host),
            ("Authorization".into(), authorization_header),
            ("Content-Length".into(), content_length),
            ("Content-Type".into(), "application/x-amz-json-1.0".into()), // binary/octet-stream
            ("x-amz-content-sha256".into(), payload_hash),
            ("x-amz-date".into(), amz_date),
            ("x-amz-target".into(), target),
        ];

        let response = http_post!(request_url, payload, headers);
        // ink_env::debug_println!(
        //     "Status = {}, Reason = {}, Json string response: {:?}",
        //     response.status_code,
        //     response.reason_phrase,
        //     String::from_utf8(response.body.clone())
        // );

        if response.status_code != 200 {
            if let Ok(body_str) = String::from_utf8(response.body) {
                if body_str.contains(&CONDITIONAL_CHECK_MESSAGE) {
                    return Err(DynamoDbError::ConditionalCheckFailed);
                }
            }
            return Err(DynamoDbError::GenericRequestFailed);
        }

        Ok(response.body)
    }
}

// Create alias for HMAC-SHA256
type HmacSha256 = Hmac<Sha256>;

// Returns encrypted hex bytes of key and message using SHA256
fn hmac_sign(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let mut mac =
        <HmacSha256 as Mac>::new_from_slice(key).expect("Could not instantiate HMAC instance");
    mac.update(msg);
    let result = mac.finalize().into_bytes();
    result.to_vec()
}

// Returns the signature key for the complicated version
fn get_signature_key(
    key: &[u8],
    datestamp: &[u8],
    region_name: &[u8],
    service_name: &[u8],
) -> Vec<u8> {
    let k_date = hmac_sign(&[b"AWS4", key].concat(), datestamp);
    let k_region = hmac_sign(&k_date, region_name);
    let k_service = hmac_sign(&k_region, service_name);
    let k_signing = hmac_sign(&k_service, b"aws4_request");
    return k_signing;
}

// Note that the below tests require a network connection to work! We deliberately do not
// mock the HTTP responses so we can also test the S3 database
#[cfg(feature = "dynamodb-live-test")]
#[cfg(test)]
mod dynamodb_tests {
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

    #[test]
    fn get_object_works() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let access_key =
            std::env::var("DYNAMODB_ACCESS_KEY").expect("Env var DYNAMODB_ACCESS_KEY is not set");
        let secret_key =
            std::env::var("DYNAMODB_SECRET_KEY").expect("Env var DYNAMODB_SECRET_KEY is not set");
        let api = DynamoDbApi {
            access_key,
            secret_key,
        };

        let timestamp_millis = now_millis();
        let payload = "{\"TableName\": \"privadex_phat_contract\", \"Key\": {\"id\": {\"S\": \"chainstate_astar\"}},\"ProjectionExpression\":\"ExecStepPendingNonce.execstep_0xcase3,ExecStepPendingBlockAdded.execstep_0xcase3\"}";
        let post_response = api
            .dynamodb_request(
                timestamp_millis,
                payload.as_bytes(),
                DynamoDbAction::GetItem,
            )
            .expect("Response expected");
        ink_env::debug_println!("get_object post - {:?}", String::from_utf8(post_response));
    }

    #[test]
    fn update_object_works() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let access_key =
            std::env::var("DYNAMODB_ACCESS_KEY").expect("Env var DYNAMODB_ACCESS_KEY is not set");
        let secret_key =
            std::env::var("DYNAMODB_SECRET_KEY").expect("Env var DYNAMODB_SECRET_KEY is not set");
        let api = DynamoDbApi {
            access_key,
            secret_key,
        };

        let timestamp_millis = now_millis();

        let payload = "{\"TableName\": \"privadex_phat_contract\", \"Key\": {\"id\": {\"S\": \"chainstate_astar\"}}, \"ReturnValues\": \"UPDATED_NEW\", \"UpdateExpression\": \"SET ExecStepPendingBlockAdded.execstep_0xcase4 = :curblock, ExecStepPendingNonce.execstep_0xcase4 = DroppedNonces[0] REMOVE DroppedNonces[0]\", \"ConditionExpression\": \"attribute_not_exists(ExecStepPendingNonce.execstep_0xcase4) AND size(DroppedNonces) > :zero AND size(ExecStepPendingNonce) > :zero\", \"ExpressionAttributeValues\": {\":curblock\": {\"N\": \"1001\"}, \":zero\": {\"N\": \"0\"}}}";
        let post_response = api
            .dynamodb_request(
                timestamp_millis,
                payload.as_bytes(),
                DynamoDbAction::UpdateItem,
            )
            .expect("Response expected");
        ink_env::debug_println!(
            "update_object post - {:?}",
            String::from_utf8(post_response)
        );
    }
}
