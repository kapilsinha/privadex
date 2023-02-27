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

use ink_env::debug_println;
use ink_prelude::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use scale::{Compact, Decode, Encode};
use serde::Deserialize;
use sp_runtime::{
    generic::{Block as GenericBlock, Era, Header as GenericHeader},
    traits::BlakeTwo256,
    OpaqueExtrinsic,
};

use privadex_chain_metadata::common::{BlockHash, BlockNum, Nonce, SubstrateExtrinsicHash};
use privadex_common::utils::{
    general_utils::{hex_string_to_vec as hex_string_to_vec_delegate, slice_to_hex_string},
    http_request::http_post_wrapper,
};

use super::{
    common::{Result, SubstrateError},
    extrinsic_sig_config::ExtrinsicSigConfig,
};

pub struct SubstrateNodeRpcUtils {
    pub rpc_url: String,
}

#[derive(Encode, Decode, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct RuntimeVersion {
    spec_name: String,
    impl_name: String,
    authoring_version: u32,
    spec_version: u32,
    impl_version: u32,
    apis: Vec<(String, u32)>,
    transaction_version: u32,
    state_version: u32,
}

type Header = GenericHeader<BlockNum, BlakeTwo256>;
pub type Block = GenericBlock<Header, OpaqueExtrinsic>;

// Deserialize for SignedBlock is unimplemented in no_std
#[cfg(feature = "std")]
type SignedBlock = sp_runtime::generic::SignedBlock<Block>;

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct RpcResponse<'a, T> {
    jsonrpc: &'a str,
    result: T,
    id: u32,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct StrRefRpcResponse<'a> {
    jsonrpc: &'a str,
    result: &'a str,
    id: u32,
}

impl SubstrateNodeRpcUtils {
    pub fn get_next_system_nonce(&self, account_id: &str) -> Result<Nonce> {
        let data = format!(
            r#"{{"id":1,"jsonrpc":"2.0","method":"system_accountNextIndex","params":["{}"]}}"#,
            account_id
        )
        .into_bytes();

        let resp_body = self.call_rpc(data)?;
        let (next_nonce, _): (RpcResponse<u32>, usize) =
            serde_json_core::from_slice(&resp_body).or(Err(SubstrateError::InvalidBody))?;
        Ok(next_nonce.result)
    }

    pub fn get_runtime_version(&self) -> Result<RuntimeVersion> {
        #[derive(Deserialize, Debug)]
        #[allow(dead_code)]
        struct RawRuntimeVersion<'a> {
            jsonrpc: &'a str,
            #[serde(borrow)]
            result: RuntimeVersionResult<'a>,
            id: u32,
        }

        #[derive(Deserialize, Encode, Clone, Debug, PartialEq)]
        #[serde(bound(deserialize = "ink_prelude::vec::Vec<(&'a str, u32)>: Deserialize<'de>"))]
        #[allow(non_snake_case)] // camelCase allows for the derived deserialize to work out of the box
        struct RuntimeVersionResult<'a> {
            specName: &'a str,
            implName: &'a str,
            authoringVersion: u32,
            specVersion: u32,
            implVersion: u32,
            #[serde(borrow)]
            apis: Vec<(&'a str, u32)>,
            transactionVersion: u32,
            stateVersion: u32,
        }

        let data = r#"{"id":1, "jsonrpc":"2.0", "method": "state_getRuntimeVersion"}"#
            .to_string()
            .into_bytes();
        let resp_body = self.call_rpc(data)?;
        let (runtime_version, _): (RawRuntimeVersion, usize) =
            serde_json_core::from_slice(&resp_body).or(Err(SubstrateError::InvalidBody))?;
        let runtime_version_result = runtime_version.result;
        let mut api_vec: Vec<(String, u32)> = Vec::new();
        for (api_str, api_u32) in runtime_version_result.apis {
            api_vec.push((api_str.to_string().parse().unwrap(), api_u32));
        }

        let runtime_version = RuntimeVersion {
            spec_name: runtime_version_result.specName.to_string().parse().unwrap(),
            impl_name: runtime_version_result.implName.to_string().parse().unwrap(),
            authoring_version: runtime_version_result.authoringVersion,
            spec_version: runtime_version_result.specVersion,
            impl_version: runtime_version_result.implVersion,
            apis: api_vec,
            transaction_version: runtime_version_result.transactionVersion,
            state_version: runtime_version_result.stateVersion,
        };

        Ok(runtime_version)
    }

    pub fn get_block_hash(&self, block_number: u32) -> Result<BlockHash> {
        let data = format!(
            r#"{{"id":1, "jsonrpc":"2.0", "method": "chain_getBlockHash","params":[{}]}}"#,
            block_number
        )
        .into_bytes();
        let resp_body = self.call_rpc(data)?;
        let (block_hash, _): (StrRefRpcResponse, usize) =
            serde_json_core::from_slice(&resp_body).or(Err(SubstrateError::InvalidBody))?;

        let v = hex_string_to_vec(block_hash.result)?;
        Ok(BlockHash::from_slice(&v))
    }

    pub fn get_genesis_hash(&self) -> Result<BlockHash> {
        self.get_block_hash(0)
    }

    pub fn get_finalized_head_hash(&self) -> Result<BlockHash> {
        let data = r#"{"id":1, "jsonrpc":"2.0", "method": "chain_getFinalizedHead"}"#
            .to_string()
            .into_bytes();
        let resp_body = self.call_rpc(data)?;
        let (finalized_head_hash, _): (StrRefRpcResponse, usize) =
            serde_json_core::from_slice(&resp_body).or(Err(SubstrateError::InvalidBody))?;

        let v = hex_string_to_vec(finalized_head_hash.result)?;
        Ok(BlockHash::from_slice(&v))
    }

    pub fn get_finalized_block_number(&self) -> Result<BlockNum> {
        // It is critical that the module and method are upper-cased to compute the correct storage key!
        let resp_body = self.query_storage("System", "Number")?;
        // debug_println!("Json string response: {:?}", String::from_utf8(resp_body.clone()));
        // This is messy decoding, but we can clean this up later
        let (number_encoded, _): (StrRefRpcResponse, usize) =
            serde_json_core::from_slice(&resp_body).or(Err(SubstrateError::InvalidBody))?;
        let number_bytes = hex_string_to_vec(number_encoded.result)?;
        let number = <BlockNum as scale::Decode>::decode(&mut number_bytes.as_slice())
            .map_err(|_| SubstrateError::InvalidBody)?;

        Ok(number)
    }

    #[allow(dead_code)]
    #[cfg(feature = "std")]
    fn get_block_header_unsafe(&self, block_hash: BlockHash) -> Result<Header> {
        /*
         * NOTE: This is not usable in an ink contract / Phat Contract because the getBlock
         * call may (and likely will) return too much data. This causes a panic:
         * "the output buffer is too small! the decoded storage is of size ___ bytes,
         * but the output buffer has only room for 16384
         * (https://github.com/paritytech/ink/blob/e883ce5088553c93b49493e43185ce05485399d3/crates/env/src/engine/off_chain/impls.rs)
         */
        let data = format!(
            r#"{{"id":1, "jsonrpc":"2.0", "method": "chain_getBlock","params":["{}"]}}"#,
            slice_to_hex_string(&block_hash.0)
        )
        .into_bytes();
        let resp_body = self.call_rpc(data)?;

        let (header, _): (RpcResponse<Header>, usize) =
            serde_json_core::from_slice(&resp_body).or(Err(SubstrateError::InvalidBody))?;
        Ok(header.result)
    }

    #[allow(dead_code)]
    #[cfg(feature = "std")]
    fn get_block_unsafe(&self, block_hash: BlockHash) -> Result<SignedBlock> {
        /*
         * NOTE: This is not usable in an ink contract / Phat Contract because the getBlock
         * call may (and likely will) return too much data. This causes a panic:
         * "the output buffer is too small! the decoded storage is of size ___ bytes,
         * but the output buffer has only room for 16384
         * (https://github.com/paritytech/ink/blob/e883ce5088553c93b49493e43185ce05485399d3/crates/env/src/engine/off_chain/impls.rs)
         */
        let data = format!(
            r#"{{"id":1, "jsonrpc":"2.0", "method": "chain_getBlock","params":["{}"]}}"#,
            slice_to_hex_string(&block_hash.0)
        )
        .into_bytes();

        let resp_body = self.call_rpc(data)?;
        // debug_println!("Raw body: {:?}", resp_body);

        let (signed_block, _): (RpcResponse<SignedBlock>, usize) =
            serde_json_core::from_slice(&resp_body).or(Err(SubstrateError::InvalidBody))?;

        let opaque_extrinsic: &OpaqueExtrinsic = &signed_block.result.block.extrinsics[2];
        // Used https://github.com/paritytech/substrate-api-sidecar/blob/108a93b1c3a23539a5be635c918d7cffd2b8be68/src/services/blocks/BlocksService.ts#L476
        // as a reference to find how to calculate extrinsic hash
        let extrinsic_hash: [u8; 32] = sp_core_hashing::blake2_256(&opaque_extrinsic.encode());
        debug_println!("Signed block: {:?}", signed_block);
        debug_println!("Extrinsic hash: {:?}", slice_to_hex_string(&extrinsic_hash));
        Ok(signed_block.result)
    }

    fn query_storage(&self, module: &str, method: &str) -> Result<Vec<u8>> {
        let storage_key = {
            let mut vec = Vec::new();
            vec.extend(sp_core_hashing::twox_128(module.as_bytes()));
            vec.extend(sp_core_hashing::twox_128(method.as_bytes()));
            slice_to_hex_string(&vec)
        };
        // debug_println!("Storage key: {:?}", &storage_key);
        let data = format!(
            r#"{{"id":1,"jsonrpc":"2.0","method":"state_getStorage","params":["{}"]}}"#,
            storage_key
        )
        .into_bytes();
        self.call_rpc(data)
    }

    pub fn create_extrinsic<AccountId>(
        &self,
        sigconfig: ExtrinsicSigConfig<AccountId>,
        encoded_call_data: &[u8],
        account_nonce: u32,
        runtime_version: RuntimeVersion,
        genesis_hash: BlockHash,
        checkpoint_block_hash: BlockHash,
        era: Era,
        tip: u128,
    ) -> Vec<u8>
    where
        AccountId: Copy + Encode,
    {
        // Construct the extra param
        let extra = Extra {
            era,
            nonce: Compact(account_nonce),
            tip: Compact(tip),
        };

        // Construct our custom additional params.
        let additional_params = (
            runtime_version.spec_version,
            runtime_version.transaction_version,
            genesis_hash.0,
            checkpoint_block_hash.0,
        );

        let encoded_payload = {
            let mut encoded_inner = Vec::new();
            encoded_inner.extend(encoded_call_data);
            extra.encode_to(&mut encoded_inner);
            additional_params.encode_to(&mut encoded_inner);
            encoded_inner
        };
        // Construct signature
        let encoded_signature = sigconfig.get_encoded_signature(encoded_payload);

        debug_println!(
            "Extrinsic head (isSigned + extrinsic version): {:?}",
            slice_to_hex_string(&(0b10000000 + 4u8).encode())
        );
        debug_println!("Signature: {:?}", slice_to_hex_string(&encoded_signature));
        debug_println!("Extra: {:?}", slice_to_hex_string(&extra.encode()));
        debug_println!("Call data: {:?}", slice_to_hex_string(encoded_call_data));

        // Encode Extrinsic
        let extrinsic = {
            let mut encoded_inner = Vec::new();
            // "is signed" + tx protocol v4
            (0b10000000 + 4u8).encode_to(&mut encoded_inner);
            // from address for signature
            encoded_inner.extend(&sigconfig.get_encoded_signer());
            // the signature bytes
            encoded_inner.extend(&encoded_signature);
            // attach custom extra params
            extra.encode_to(&mut encoded_inner);
            // and now, call data
            encoded_inner.extend(encoded_call_data);
            // now, prefix byte length:
            let len = Compact(
                u32::try_from(encoded_inner.len()).expect("extrinsic size expected to be <4GB"),
            );
            let mut encoded = Vec::new();
            len.encode_to(&mut encoded);
            encoded.extend(encoded_inner);
            encoded
        };

        extrinsic
    }

    #[cfg(not(feature = "mock-txn-send"))]
    pub fn send_extrinsic(&self, extrinsic_hash: &[u8]) -> Result<SubstrateExtrinsicHash> {
        let hex_extrinsic = slice_to_hex_string(extrinsic_hash);
        let data = format!(
            r#"{{"id":1,"jsonrpc":"2.0","method":"author_submitExtrinsic","params":["{}"]}}"#,
            hex_extrinsic
        )
        .into_bytes();
        let resp_body = self.call_rpc(data)?;
        // ink_env::debug_println!(
        //     "Json string response: {:?}",
        //     String::from_utf8(resp_body.clone())
        // );
        let (tx, _): (StrRefRpcResponse, usize) =
            serde_json_core::from_slice(&resp_body).or(Err(SubstrateError::InvalidBody))?;
        let v = hex_string_to_vec(tx.result)?;
        Ok(SubstrateExtrinsicHash::from_slice(&v))
    }

    #[cfg(feature = "mock-txn-send")]
    pub fn send_extrinsic(&self, extrinsic_hash: &[u8]) -> Result<SubstrateExtrinsicHash> {
        ink_env::debug_println!("[Mock Substrate send_extrinsic]");
        Ok(SubstrateExtrinsicHash::zero())
    }

    #[cfg(feature = "std")]
    pub fn find_extrinsic_unsafe(
        &self,
        _extrinsic_hash: SubstrateExtrinsicHash,
        block_number: u32,
    ) -> Result<u32 /* extrinsic_index */> {
        // This was a first pass at finding extrinsic via RPC url only. Now this functionality
        // exists in indexer_utils
        let block_hash = self.get_block_hash(block_number)?;
        debug_println!("Block hash: {:?}", block_hash);
        let _ = self.get_block_unsafe(block_hash)?;
        Ok(99u32)
    }

    fn call_rpc(&self, data: Vec<u8>) -> Result<Vec<u8>> {
        http_post_wrapper(&self.rpc_url, data).map_err(|_| SubstrateError::RequestFailed)
    }
}

fn hex_string_to_vec(s: &str) -> Result<Vec<u8>> {
    hex_string_to_vec_delegate(s).map_err(|_| SubstrateError::InvalidHex)
}

#[derive(Encode, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
struct Extra {
    // 0 if Immortal, or Vec<u64, u64> for period and the phase.
    era: Era,
    // Nonce
    nonce: Compact<u32>,
    // Tip for the block producer.
    tip: Compact<u128>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;
    use privadex_chain_metadata::{chain_info::ChainInfo, registry::chain::chain_info_registry};

    fn utils(chain_info: &ChainInfo) -> SubstrateNodeRpcUtils {
        SubstrateNodeRpcUtils {
            rpc_url: chain_info.rpc_url.to_string(),
        }
    }

    #[test]
    fn moonbeam_nonce() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let address = "0x05a81d8564a3eA298660e34e03E5Eff9a29d7a2A";
        let nonce = utils(&chain_info_registry::MOONBEAM_INFO)
            .get_next_system_nonce(address)
            .expect("Expected valid nonce");
        assert!(nonce > 100);
    }

    #[test]
    fn astar_nonce() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        // This is the Substrate-mapped address of 0x05a81d8564a3eA298660e34e03E5Eff9a29d7a2A
        // Converted using https://hoonsubin.github.io/evm-substrate-address-converter/
        // (original article at https://medium.com/astar-network/using-astar-network-account-between-substrate-and-evm-656643df22a0)
        let address = "XmmrKUnJjEsupddUPCKQuBkUqEFiCm6jJPZaaYK5T25f9w7";
        let nonce = utils(&chain_info_registry::ASTAR_INFO)
            .get_next_system_nonce(address)
            .expect("Expected valid nonce");
        // ink_env::debug_println!("nonce = {}", nonce);
        assert!(nonce > 1);
    }

    #[cfg(feature = "private-rpc-endpoint")]
    #[test]
    fn polkadot_nonce() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let address = "1qnJN7FViy3HZaxZK9tGAA71zxHSBeUweirKqCaox4t8GT7";
        let nonce = utils(&chain_info_registry::POLKADOT_INFO)
            .get_next_system_nonce(address)
            .expect("Expected valid nonce");
        assert!(nonce > 100_000);
    }

    #[test]
    fn moonbeam_genesis() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let genesis_hash = utils(&chain_info_registry::MOONBEAM_INFO)
            .get_genesis_hash()
            .expect("Expected valid genesis hash");
        debug_println!("genesis hash: {}", genesis_hash);
        assert_eq!(
            genesis_hash,
            BlockHash {
                0: hex!("fe58ea77779b7abda7da4ec526d14db9b1e9cd40a217c34892af80a9b332b76d")
            }
        );
    }

    #[cfg(feature = "private-rpc-endpoint")]
    #[test]
    fn polkadot_genesis() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let genesis_hash = utils(&chain_info_registry::POLKADOT_INFO)
            .get_genesis_hash()
            .expect("Expected valid genesis hash");
        debug_println!("genesis hash: {}", genesis_hash);
        let finalized_head = utils(&chain_info_registry::POLKADOT_INFO)
            .get_finalized_head_hash()
            .expect("Expected valid finalized head hash");
        debug_println!("finalized head hash: {}", finalized_head);
        assert_eq!(
            genesis_hash,
            BlockHash {
                0: hex!("91b171bb158e2d3848fa23a9f1c25182fb8e20313b2c1eb49219da7a70ce90c3")
            }
        );
    }

    #[test]
    fn moonbeam_finalized_head_hash() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let finalized_head = utils(&chain_info_registry::MOONBEAM_INFO)
            .get_finalized_head_hash()
            .expect("Expected valid finalized head hash");
        debug_println!("finalized head hash: {}", finalized_head);
    }

    #[test]
    fn moonbeam_block_number() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let block_num = utils(&chain_info_registry::MOONBEAM_INFO)
            .get_finalized_block_number()
            .expect("Expected valid finalized block number");
        debug_println!("block num: {}", block_num);
        assert!(block_num > 2_475_364u32);
    }

    #[test]
    fn moonbeam_runtime_version() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let runtime_version = utils(&chain_info_registry::MOONBEAM_INFO)
            .get_runtime_version()
            .expect("Expected valid runtime version");
        debug_println!("Runtime version: {:?}", runtime_version);
        assert_eq!(runtime_version.impl_name, "moonbeam");
        assert!(runtime_version.apis.len() > 0);
    }

    #[cfg(feature = "private-rpc-endpoint")]
    #[test]
    fn polkadot_runtime_version() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let runtime_version = utils(&chain_info_registry::POLKADOT_INFO)
            .get_runtime_version()
            .expect("Expected valid runtime version");
        debug_println!("Runtime version: {:?}", runtime_version);
        assert_eq!(runtime_version.impl_name, "parity-polkadot");
        assert!(runtime_version.apis.len() > 0);
    }

    #[cfg(feature = "std")]
    #[test]
    fn moonbase_find_extrinsic_unsafe() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let block_num = 1000u32;
        // let block_num = 3_144_185u32; // This will cause a failure! See the comment on get_block(...) regarding buffer size
        let _ = utils(&chain_info_registry::MOONBASEALPHA_INFO)
            .find_extrinsic_unsafe(SubstrateExtrinsicHash::zero(), block_num)
            .expect("Failure");
    }
}

// Some RPC endpoints do not support author_submitExtrinsic.
// These tests check that the RPC endpoint works correctly.
// We send a dummy extrinsic and expect a SubstrateError::InvalidBody
// (i.e. response received that we submitted an invalid extrinsic)
// instead of a SubstrateError::RequestFailed
#[cfg(test)]
mod send_extrinsic_tests {
    use super::*;
    use privadex_chain_metadata::{chain_info::ChainInfo, registry::chain::chain_info_registry};

    fn utils(chain_info: &ChainInfo) -> SubstrateNodeRpcUtils {
        SubstrateNodeRpcUtils {
            rpc_url: chain_info.rpc_url.to_string(),
        }
    }

    #[test]
    fn moonbeam_send_extrinsic_bad() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let dummy_extrinsic: Vec<u8> = Vec::new();
        let bad_rpc_res = SubstrateNodeRpcUtils {
            rpc_url: "https://moonbeam.public.blastapi.io".to_string(),
        }
        .send_extrinsic(&dummy_extrinsic);
        assert_eq!(bad_rpc_res, Err(SubstrateError::RequestFailed));
    }

    #[cfg(feature = "private-rpc-endpoint")]
    #[test]
    fn moonbeam_send_extrinsic_good() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let dummy_extrinsic: Vec<u8> = Vec::new();
        // let dummy_extrinsic = hex_literal::hex!("69028405a81d8564a3ea298660e34e03e5eff9a29d7a2a44bdac274c226d2db4399608424da944d03f9609acdc06e159450d6f115211e312c4fc68bb7c145531038cb2d4d117fa34b6b7e65c749fcf489690ee41aebe170135005103006a0101000001040a0013968196396f923e0a01010200591f01005134c7f0e31c2a9e19dceddb7403b2836c69cce0b0719d2f58ec0d4da35129be0102286bee");
        let rpc_res = utils(&chain_info_registry::MOONBEAM_INFO).send_extrinsic(&dummy_extrinsic);
        assert_eq!(rpc_res, Err(SubstrateError::InvalidBody));
    }

    #[test]
    fn astar_send_extrinsic_bad() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let dummy_extrinsic: Vec<u8> = Vec::new();
        let bad_rpc_res = SubstrateNodeRpcUtils {
            rpc_url: "https://astar.public.blastapi.io".to_string(),
        }
        .send_extrinsic(&dummy_extrinsic);
        assert_eq!(bad_rpc_res, Err(SubstrateError::RequestFailed));
    }

    #[cfg(feature = "private-rpc-endpoint")]
    #[test]
    fn astar_send_extrinsic_good() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let dummy_extrinsic: Vec<u8> = Vec::new();
        let rpc_res = utils(&chain_info_registry::ASTAR_INFO).send_extrinsic(&dummy_extrinsic);
        assert_eq!(rpc_res, Err(SubstrateError::InvalidBody));
    }

    #[cfg(feature = "private-rpc-endpoint")]
    #[test]
    fn polkadot_send_extrinsic_good() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let dummy_extrinsic: Vec<u8> = Vec::new();
        let rpc_res = utils(&chain_info_registry::POLKADOT_INFO).send_extrinsic(&dummy_extrinsic);
        assert_eq!(rpc_res, Err(SubstrateError::InvalidBody));
    }
}
