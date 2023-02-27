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

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub mod bridge;
pub mod chain_info;
pub mod common;
pub mod registry;

use chain_info::{AddressType, ChainInfo};
use common::{
    Dex, EthAddress, PublicError, Result, SubstratePublicKey, UniversalAddress, UniversalChainId,
};
use ink_prelude::{vec, vec::Vec};
use registry::{
    chain::{chain_info_registry, universal_chain_id_registry},
    dex::dex_registry,
};
use scale::Encode;

pub fn get_chain_id_from_network_name(network_name: &str) -> Option<UniversalChainId> {
    match network_name {
        "astar" => Some(universal_chain_id_registry::ASTAR),
        "moonbeam" => Some(universal_chain_id_registry::MOONBEAM),
        "polkadot" => Some(universal_chain_id_registry::POLKADOT),

        "moonbase-alpha" => Some(universal_chain_id_registry::MOONBASE_ALPHA),
        "moonbase-beta" => Some(universal_chain_id_registry::MOONBASE_BETA),
        _ => None,
    }
}

pub fn get_chain_info_from_chain_id(chain_id: &UniversalChainId) -> Option<&'static ChainInfo> {
    match chain_id {
        &universal_chain_id_registry::ASTAR => Some(&chain_info_registry::ASTAR_INFO),
        &universal_chain_id_registry::MOONBEAM => Some(&chain_info_registry::MOONBEAM_INFO),
        &universal_chain_id_registry::POLKADOT => Some(&chain_info_registry::POLKADOT_INFO),

        &universal_chain_id_registry::MOONBASE_ALPHA => {
            Some(&chain_info_registry::MOONBASEALPHA_INFO)
        }
        &universal_chain_id_registry::MOONBASE_BETA => {
            Some(&chain_info_registry::MOONBASEBETA_INFO)
        }
        _ => None,
    }
}

pub fn get_dexes_from_chain_id(chain_id: &UniversalChainId) -> Vec<&'static Dex> {
    match chain_id {
        &universal_chain_id_registry::ASTAR => vec![&dex_registry::ARTHSWAP],
        &universal_chain_id_registry::MOONBEAM => {
            vec![&dex_registry::STELLASWAP, &dex_registry::BEAMSWAP]
        }
        &universal_chain_id_registry::POLKADOT => vec![],

        &universal_chain_id_registry::MOONBASE_ALPHA => vec![&dex_registry::MOONBASE_UNISWAP],
        &universal_chain_id_registry::MOONBASE_BETA => vec![],
        _ => vec![],
    }
}

// Defined in https://docs.moonbeam.network/builders/xcm/overview/#general-xcm-definitions
// ^This specifies that a blake2 hash is involved, but it actually isn't
// Logic based on https://github.com/albertov19/xcmTools/blob/main/calculateSovereignAddress.ts
/// Gets the sovereign account of account_chain on dest_chain, used in XCM messages
/// For example, get_sovereign_account(Moonbeam, Polkadot) returns Moonbeam's (parachain 2004)
/// account address on Polkadot.
pub fn get_sovereign_account(
    account_chain: UniversalChainId,
    dest_chain_info: &ChainInfo,
) -> Result<UniversalAddress> {
    if account_chain.get_relay() != dest_chain_info.chain_id.get_relay() {
        return Err(PublicError::NoSovereignAccount);
    }
    let para_id = account_chain
        .get_parachain_id()
        .ok_or(PublicError::NoSovereignAccount)?;

    let prefix = if dest_chain_info.chain_id.get_parachain_id().is_some() {
        "sibl"
    } else {
        "para"
    };
    let mut addr: Vec<u8> = Vec::new();
    addr.extend_from_slice(prefix.as_bytes());
    para_id.encode_to(&mut addr);

    match dest_chain_info.xcm_address_type {
        AddressType::Ethereum => {
            addr.resize(20 /* new size */, 0 /* padding */);
            Ok(UniversalAddress::Ethereum(EthAddress::from_slice(&addr)))
        }
        AddressType::SS58 => {
            addr.resize(32 /* new size */, 0 /* padding */);
            Ok(UniversalAddress::Substrate(SubstratePublicKey::from_slice(
                &addr,
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use hex_literal::hex;

    use super::*;

    #[test]
    fn test_sovereign_account_astar_on_moonbeam() {
        // https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fwss.api.moonbeam.network#/explorer/query/0x4873801523128e8246301e46e37df7f5e3ee75ccb15d06dc66a88f467dec8e51
        let addr = get_sovereign_account(
            universal_chain_id_registry::ASTAR,
            &chain_info_registry::MOONBEAM_INFO,
        )
        .expect("Should output a valid address");
        let expected = UniversalAddress::Ethereum(EthAddress {
            0: hex!("7369626CD6070000000000000000000000000000"),
        });
        assert_eq!(addr, expected);
    }

    #[test]
    fn test_sovereign_account_moonbeam_on_astar() {
        // https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Frpc.astar.network#/explorer/query/0x3eb44462727ab68abc33b06aee47ce3c61fc6734ed796b331034349729e08e31
        // subkey inspect YYd75rPbqhhtAT826DJWF5PnpaDLQofq8sJTtReQofbwVwm
        // Public Key URI `YYd75rPbqhhtAT826DJWF5PnpaDLQofq8sJTtReQofbwVwm` is account:
        // Network ID/version: plasm
        // Public key (hex):   0x7369626cd4070000000000000000000000000000000000000000000000000000
        // Account ID:         0x7369626cd4070000000000000000000000000000000000000000000000000000
        // SS58 Address:       YYd75rPbqhhtAT826DJWF5PnpaDLQofq8sJTtReQofbwVwm
        let addr = get_sovereign_account(
            universal_chain_id_registry::MOONBEAM,
            &chain_info_registry::ASTAR_INFO,
        )
        .expect("Should output a valid address");
        let expected = UniversalAddress::Substrate(SubstratePublicKey {
            0: hex!("7369626cd4070000000000000000000000000000000000000000000000000000"),
        });
        assert_eq!(addr, expected);
    }

    #[test]
    fn test_sovereign_account_moonbeam_on_polkadot() {
        // https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Frpc.dotters.network%2Fpolkadot#/explorer/query/0xd45e457a848d08291b2e9db41bd7df207b0f94a6d61a63a83d9d0f46da8662e6
        // subkey inspect 13YMK2eZbf9AyGhewRs6W6QTJvBSM5bxpnTD8WgeDofbg8Q1
        // Public Key URI `13YMK2eZbf9AyGhewRs6W6QTJvBSM5bxpnTD8WgeDofbg8Q1` is account:
        // Network ID/version: polkadot
        // Public key (hex):   0x70617261d4070000000000000000000000000000000000000000000000000000
        // Account ID:         0x70617261d4070000000000000000000000000000000000000000000000000000
        // SS58 Address:       13YMK2eZbf9AyGhewRs6W6QTJvBSM5bxpnTD8WgeDofbg8Q1
        let addr = get_sovereign_account(
            universal_chain_id_registry::MOONBEAM,
            &chain_info_registry::POLKADOT_INFO,
        )
        .expect("Should output a valid address");
        let expected = UniversalAddress::Substrate(SubstratePublicKey {
            0: hex!("70617261d4070000000000000000000000000000000000000000000000000000"),
        });
        assert_eq!(addr, expected);
    }
}
