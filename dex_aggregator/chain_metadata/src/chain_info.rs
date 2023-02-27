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
use ss58_registry::Ss58AddressFormat;

use privadex_common::signature_scheme::SignatureScheme;

use crate::common::{Amount, EthAddress, UniversalChainId};

// From what I have seen,
// AddressType.Ethereum corresponds to SignatureScheme.Ethereum (e.g. Moonbeam) and
// AddressType.SS58 corresponds to SignatureScheme.Sr25519 (e.g. Polkadot, Astar)
// but I don't enforce that link
#[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum AddressType {
    Ethereum,
    SS58,
}

// Not deriving Encode or Decode because
// "the trait `WrapperTypeDecode` is not implemented for `&'static str"
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ChainInfo {
    pub chain_id: UniversalChainId,
    // Can be looked up at polkadot.js.org.apps/... -> ChainState -> Constants -> system.ss58Prefix
    pub(crate) ss58_prefix_raw: Option<u16>,

    // Defines the address format (20-byte key for Ethereum or 32-byte public key for SS58)
    // used to define addresses in XCM MultiLocations
    pub xcm_address_type: AddressType,
    pub sig_scheme: SignatureScheme,

    // Used in sending EVM txns, can look up at chainlist.org
    pub evm_chain_id: Option<u64>,
    pub weth_addr: Option<EthAddress>,
    // I look at swap txns for reference
    pub avg_gas_fee_in_native_token: Amount, // hard-coded estimate
    // Cost of bridging TO this chain
    pub avg_bridge_fee_in_native_token: Amount, // hard-coded estimate

    pub rpc_url: &'static str,
    pub subsquid_graphql_archive_url: &'static str,
}

impl ChainInfo {
    // I deliberately don't store Ss58AddressFormat so that ChainInfo is
    // const-constructible
    pub fn get_ss58_prefix(&self) -> Option<Ss58AddressFormat> {
        Some(Ss58AddressFormat::custom(self.ss58_prefix_raw?))
    }
}
