use pink_extension as pink;
use ink_env;
use ink_env::debug_println;
use ink_prelude::format;
use ink_prelude::vec;
use ink_prelude::{
    string::{String, ToString},
    vec::Vec,
};
use scale::{Decode, Encode};

// To make HTTP request
use pink::{http_get, http_put};
use pink::chain_extension::signing;

// To generate AWS4 Signature
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

// To format block timestamp for http request headers
use chrono::{TimeZone, Utc};

// To encrypt/decrypt HTTP payloads
use aes_gcm_siv::aead::{Aead, KeyInit, Nonce};
use aes_gcm_siv::Aes256GcmSiv;
use base16;
use cipher::{
    consts::{U12, U32},
    generic_array::GenericArray,
};

#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct S3Api {
    access_key: String,
    secret_key: String,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    RequestFailed,
    EncryptionFailed,
    DecryptionFailed,
    PlatformNotFound,
    CredentialsNotSealed,
}

impl S3Api {

    // Hacky way to populate a time in just the test environment because
    // env().block_timestamp() is 0 off chain and I can't figure out how to mock it
    // #[cfg(test)]
    // fn now_millis(&self) -> u64 {
    //     use std::time::SystemTime;
    //     SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis().try_into().unwrap()
    // }

    // #[cfg(not(test))]
    // fn now_millis(&self) -> u64 {
    //     // self.env().block_timestamp()
    //     ink_env::block_timestamp()
    // }

    fn get_time(&self, timestamp_millis: u64) -> (String, String) {
        // Get block time (UNIX time in nano seconds)and convert to Utc datetime object
        let time = timestamp_millis / 1000;
        let datetime = Utc.timestamp(time.try_into().unwrap(), 0);

        // Format both date and datetime for AWS4 signature
        let datestamp = datetime.format("%Y%m%d").to_string();
        let datetimestamp = datetime.format("%Y%m%dT%H%M%SZ").to_string();

        debug_println!("Time: {}, {}", datestamp, datetimestamp);

        (datestamp, datetimestamp)
    }

    /// HTTP GETs and decrypts the data from the specified storage platform.
    /// Must seal the correct credentials before calling this function
    pub fn get_object_raw(
        &self,
        timestamp_millis: u64,
        platform: String,
        object_key: String,
        bucket_name: String,
        region: Option<String>,
    ) -> Result<Vec<u8>, Error> {
        // Set request values
        let method = "GET";
        let service = "s3";
        let region = region.unwrap_or(String::from("us-east-1"));

        let host = if platform == "s3" {
            format!("{}.s3.amazonaws.com", bucket_name)
        } else if platform == "4everland" {
            "endpoint.4everland.co".to_string()
        } else if platform == "storj" {
            "gateway.storjshare.io".to_string()
        } else if platform == "filebase" {
            "s3.filebase.com".to_string()
        } else {
            return Err(Error::PlatformNotFound);
        };

        let payload_hash = format!("{:x}", Sha256::digest(b"")); // GET has default payload empty byte

        // Get current time: datestamp (e.g. 20220727) and amz_date (e.g. 20220727T141618Z)
        let (datestamp, amz_date) = self.get_time(timestamp_millis);

        // 1. Create canonical request
        let canonical_uri: String = if platform == "s3" {
            format!("/{}", object_key)
        } else {
            format!("/{}/{}", bucket_name, object_key)
        };
        let canonical_querystring = "";
        let canonical_headers = format!(
            "host:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n",
            host, payload_hash, amz_date
        );
        let signed_headers = "host;x-amz-content-sha256;x-amz-date";
        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method,
            canonical_uri,
            canonical_querystring,
            canonical_headers,
            signed_headers,
            payload_hash
        );

        debug_println!(" ----- Canonical request -----  \n{}\n", canonical_request);
        //  ----- Canonical request -----
        // GET
        // /test/api-upload
        //
        // host:fat-contract-s3-sync.s3.amazonaws.com
        // x-amz-content-sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        // x-amz-date:19700101T000000Z
        //
        // host;x-amz-content-sha256;x-amz-date
        // e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855

        // 2. Create "String to sign"
        let algorithm = "AWS4-HMAC-SHA256";
        let credential_scope = format!("{}/{}/{}/aws4_request", datestamp, region, service);
        let canonical_request_hash =
            format!("{:x}", Sha256::digest(&canonical_request.as_bytes()));
        let string_to_sign = format!(
            "{}\n{}\n{}\n{}",
            algorithm, amz_date, credential_scope, canonical_request_hash
        );

        debug_println!(" ----- String to sign ----- \n{}\n", string_to_sign);
        //  ----- String to sign -----
        // AWS4-HMAC-SHA256
        // 19700101T000000Z
        // 19700101/ap-southeast-1/s3/aws4_request
        // ec70fa653b4f867cda7a59007db15a7e95ed45d70bacdfb55902a2fb09b6367f

        // 3. Calculate signature
        let signature_key = get_signature_key(
            self.secret_key.as_bytes(),
            &datestamp.as_bytes(),
            &region.as_bytes(),
            &service.as_bytes(),
        );
        let signature_bytes = hmac_sign(&signature_key, &string_to_sign.as_bytes());
        let signature = format!("{}", base16::encode_lower(&signature_bytes));

        debug_println!(" ----- Signature ----- \n{}\n", &signature);
        //  ----- Signature -----
        // 485e174a7fed1691de34f116a968981709ed5a00f4975470bd3d0dd06ccd3e1d

        // 4. Create authorization header
        let authorization_header = format!(
            "{} Credential={}/{}, SignedHeaders={}, Signature={}",
            algorithm, self.access_key, credential_scope, signed_headers, signature
        );

        debug_println!(
            " ----- Authorization header ----- \nAuthorization: {}\n",
            &authorization_header
        );
        //  ----- Authorization header -----
        // Authorization: AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/19700101/ap-southeast-1/s3/aws4_request, SignedHeaders=host;x-amz-content-sha256;x-amz-date, Signature=485e174a7fed1691de34f116a968981709ed5a00f4975470bd3d0dd06ccd3e1d

        let headers: Vec<(String, String)> = vec![
            ("Host".into(), host.to_string()),
            ("Authorization".into(), authorization_header.clone()),
            ("x-amz-content-sha256".into(), payload_hash),
            ("x-amz-date".into(), amz_date),
        ];

        // Make HTTP GET request
        let request_url: String = if platform == "s3" {
            format!(
                "https://{}.s3.{}.amazonaws.com/{}",
                bucket_name, region, object_key
            )
        } else {
            format!("https://{}/{}/{}", host, bucket_name, object_key)
        };
        let response = http_get!(request_url, headers);

        debug_println!("Get response: {}", response.reason_phrase);

        if response.status_code != 200 {
            return Err(Error::RequestFailed);
        }

        // Generate key and nonce
        let key_bytes: Vec<u8> =
            signing::derive_sr25519_key(object_key.as_bytes())[..32].to_vec();
        let key: &GenericArray<u8, U32> = GenericArray::from_slice(&key_bytes);
        let nonce_bytes: Vec<u8> = self.access_key.as_bytes()[..12].to_vec();
        let nonce: &GenericArray<u8, U12> = Nonce::<Aes256GcmSiv>::from_slice(&nonce_bytes);

        // Decrypt payload
        let cipher = Aes256GcmSiv::new(key.into());
        let decrypted_byte = cipher
            .decrypt(&nonce, response.body.as_ref())
            .or(Err(Error::DecryptionFailed));
        decrypted_byte
    }

    pub fn get_object_str(
        &self,
        timestamp_millis: u64,
        platform: String,
        object_key: String,
        bucket_name: String,
        region: Option<String>,
    ) -> Result<String, Error> {
        let raw = self.get_object_raw(timestamp_millis, platform, object_key, bucket_name, region)?;
        Ok(format!("{}", String::from_utf8_lossy(&raw)))
    }

    /// Encrypts and HTTP PUTs the data to the specified storage platform as byte stream.
    /// Must seal the correct credentials before calling this function.
    pub fn put_object_raw(
        &self,
        timestamp_millis: u64,
        platform: String,
        object_key: String,
        bucket_name: String,
        region: String,
        payload: &[u8],
    ) -> Result<String, Error> {
        // Generate key and nonce
        debug_println!("Key: {:?}", object_key.as_bytes());
        let key_bytes: Vec<u8> =
            signing::derive_sr25519_key(object_key.as_bytes())[..32].to_vec();
        let key: &GenericArray<u8, U32> = GenericArray::from_slice(&key_bytes);
        let nonce_bytes: Vec<u8> = self.access_key.as_bytes()[..12].to_vec();
        let nonce: &GenericArray<u8, U12> = Nonce::<Aes256GcmSiv>::from_slice(&nonce_bytes);

        // Encrypt payload
        let cipher = Aes256GcmSiv::new(key.into());
        let encrypted_bytes: Vec<u8> =
            cipher.encrypt(nonce, payload.as_ref()).unwrap();

        // Set request values
        let method = "PUT";
        let service = "s3";
        // let region = region.unwrap_or(String::from("us-east-1"));

        let host = if platform == "s3" {
            format!("{}.s3.amazonaws.com", bucket_name)
        } else if platform == "4everland" {
            "endpoint.4everland.co".to_string()
        } else if platform == "storj" {
            "gateway.storjshare.io".to_string()
        } else if platform == "filebase" {
            "s3.filebase.com".to_string()
        } else {
            return Err(Error::PlatformNotFound);
        };

        let payload_hash = format!("{:x}", Sha256::digest(&encrypted_bytes));
        let content_length = format!("{}", encrypted_bytes.clone().len());

        // Get datestamp (20220727) and amz_date (20220727T141618Z)
        let (datestamp, amz_date) = self.get_time(timestamp_millis);

        // 1. Create canonical request
        let canonical_uri: String = if platform == "s3" {
            format!("/{}", object_key)
        } else {
            format!("/{}/{}", bucket_name, object_key)
        };
        let canonical_querystring = "";
        let canonical_headers = format!(
            "host:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n",
            host, payload_hash, amz_date
        );
        let signed_headers = "host;x-amz-content-sha256;x-amz-date";
        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method,
            canonical_uri,
            canonical_querystring,
            canonical_headers,
            signed_headers,
            payload_hash
        );

        debug_println!(" ----- Canonical request -----  \n{}\n", canonical_request);
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
        let canonical_request_hash =
            format!("{:x}", Sha256::digest(&canonical_request.as_bytes()));
        let string_to_sign = format!(
            "{}\n{}\n{}\n{}",
            algorithm, amz_date, credential_scope, canonical_request_hash
        );

        debug_println!(" ----- String to sign ----- \n{}\n", string_to_sign);
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

        debug_println!(" ----- Signature ----- \n{}\n", &signature);
        //  ----- Signature -----
        // 84bf2db9f7a0007f5124cf2e9c0e1b7e1cec2b1b1b209ab9458387caa3b8da52

        // 4. Create authorization header
        let authorization_header = format!(
            "{} Credential={}/{},SignedHeaders={},Signature={}",
            algorithm, self.access_key, credential_scope, signed_headers, signature
        );

        debug_println!(
            " ----- Authorization header ----- \nAuthorization: {}\n",
            &authorization_header
        );
        //  ----- Authorization header -----
        // Authorization: AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/19700101/ap-southeast-1/s3/aws4_request,SignedHeaders=host;x-amz-content-sha256;x-amz-date,Signature=b9b6bcb29b1369678e3a3cfae411a5277c084c8c1796bb6e78407f402f9e3f3d

        let request_url: String = if platform == "s3" {
            format!(
                "https://{}.s3.{}.amazonaws.com/{}",
                bucket_name, region, object_key
            )
        } else {
            format!("https://{}/{}/{}", host, bucket_name, object_key)
        };

        let headers: Vec<(String, String)> = vec![
            ("Host".into(), host),
            ("Authorization".into(), authorization_header),
            ("Content-Length".into(), content_length),
            ("Content-Type".into(), "binary/octet-stream".into()),
            ("x-amz-content-sha256".into(), payload_hash),
            ("x-amz-date".into(), amz_date),
        ];

        let response = http_put!(request_url, encrypted_bytes, headers);

        if response.status_code != 200 {
            return Err(Error::RequestFailed);
        }

        Ok(format!(
            "{}\n{}\n{}\n{:?}",
            response.status_code,
            response.reason_phrase,
            String::from_utf8_lossy(&response.body),
            response.headers
        ))
    }

    pub fn put_object_str(
        &self,
        timestamp_millis: u64,
        platform: String,
        object_key: String,
        bucket_name: String,
        region: String,
        payload: String,
    ) -> Result<String, Error> {
        self.put_object_raw(
            timestamp_millis,
            platform,
            object_key,
            bucket_name,
            region,
            payload.as_bytes()
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    fn now_millis() -> u64 {
        use std::time::SystemTime;
        SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis().try_into().unwrap()
    }

    #[test]
    fn put_object_works() {
        pink_extension_runtime::mock_ext::mock_all_ext();
    
        // mock::mock_http_request(|request| {
        //     if request.url == "https://s3.filebase.com/fat-contract-filebase-sync/test/api-upload" {
        //         HttpResponse::ok(b"Success".to_vec())
        //     } else {
        //         HttpResponse::not_found()
        //     }
        // });

        let access_key = std::env::var("S3_ACCESS_KEY").expect("Env var S3_ACCESS_KEY is not set");
        let secret_key = std::env::var("S3_SECRET_KEY").expect("Env var S3_SECRET_KEY is not set");
        let api = S3Api{ access_key, secret_key };
        
        let timestamp_millis = now_millis();

        let put_response = api.put_object_str(
            timestamp_millis,
            "storj".to_string(),
            "txn_0xd9ff564a3b27e41a9c59eabbec5f5564c3bf1c0bba9e54c595c3e916082ff3a8".to_string(),
            "transfer-txn".to_string(),
            "us-east-1".to_string(),
            "This is a test comment234".to_string());
        debug_println!("Put1 - {:?}", put_response);

        let get_response = api.get_object_str(
            timestamp_millis,
            "storj".to_string(),
            "txn_0xd9ff564a3b27e41a9c59eabbec5f5564c3bf1c0bba9e54c595c3e916082ff3a8".to_string(),
            "transfer-txn".to_string(),
            Some("us-east-1".to_string()),
        );
        debug_println!("Get1 - {:?}", get_response);
    }

    #[test]
    fn get_object() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let access_key = std::env::var("S3_ACCESS_KEY").expect("Env var S3_ACCESS_KEY is not set");
        let secret_key = std::env::var("S3_SECRET_KEY").expect("Env var S3_SECRET_KEY is not set");
        let api = S3Api{ access_key, secret_key };

        let timestamp_millis = now_millis();

        let get_response = api.get_object_str(
            timestamp_millis,
            "storj".to_string(),
            "txn_0xd9ff564a3b27e41a9c59eabbec5f5564c3bf1c0bba9e54c595c3e916082ff3a8".to_string(),
            "transfer-txn".to_string(),
            Some("us-east-1".to_string()),
        );
        debug_println!("Get2 - {:?}", get_response);
    }
    
    #[test]
    fn aead_works() {
    
        let payload = "test";
    
        // Generate key and nonce
        let key_bytes: Vec<u8> = vec![0; 32];
        let key: &GenericArray<u8, U32> = GenericArray::from_slice(&key_bytes);
        let nonce_bytes: Vec<u8> = vec![0; 12];
        let nonce: &GenericArray<u8, U12> = Nonce::<Aes256GcmSiv>::from_slice(&nonce_bytes);
    
        // Encrypt payload
        let cipher = Aes256GcmSiv::new(key.into());
        let encrypted_text: Vec<u8> = cipher.encrypt(nonce, payload.as_bytes().as_ref()).unwrap();
    
        // Generate key and nonce
        let key_bytes: Vec<u8> = vec![0; 32];
        let key: &GenericArray<u8, U32> = GenericArray::from_slice(&key_bytes);
        let nonce_bytes: Vec<u8> = vec![0; 12];
        let nonce: &GenericArray<u8, U12> = Nonce::<Aes256GcmSiv>::from_slice(&nonce_bytes);
    
        // Decrypt payload
        let cipher = Aes256GcmSiv::new(key.into());
        let decrypted_text = cipher.decrypt(&nonce, encrypted_text.as_ref()).unwrap();
    
        assert_eq!(payload.as_bytes(), decrypted_text);
        assert_eq!(payload, String::from_utf8_lossy(&decrypted_text));
    }
}
