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

#[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum RelayChain {
    Polkadot,
    Kusama,
    Westend,
    Rococo,
    MoonbaseRelay,
}

pub mod universal_chain_id_registry {
    use super::RelayChain;
    use crate::common::UniversalChainId;

    pub const MOONBEAM: UniversalChainId =
        UniversalChainId::SubstrateParachain(RelayChain::Polkadot, 2004);
    // Note that we will (for now) only associate with the EVM (not Native) addresses on Astar
    pub const ASTAR: UniversalChainId =
        UniversalChainId::SubstrateParachain(RelayChain::Polkadot, 2006);
    pub const POLKADOT: UniversalChainId =
        UniversalChainId::SubstrateRelayChain(RelayChain::Polkadot);

    pub const MOONBASE_ALPHA: UniversalChainId =
        UniversalChainId::SubstrateParachain(RelayChain::MoonbaseRelay, 1000);
    pub const MOONBASE_BETA: UniversalChainId =
        UniversalChainId::SubstrateParachain(RelayChain::MoonbaseRelay, 888);
    pub const KHALA: UniversalChainId =
        UniversalChainId::SubstrateParachain(RelayChain::Kusama, 2004);
}

pub mod chain_info_registry {
    use hex_literal::hex;
    use privadex_common::signature_scheme::SignatureScheme;

    use super::universal_chain_id_registry;
    use crate::chain_info::{AddressType, ChainInfo};
    use crate::common::EthAddress;
    // Note that Ss58AddressFormat::try_from("astar").ok() uses https://github.com/paritytech/ss58-registry
    // but to keep these const I have manually pulled the values

    pub const ASTAR_INFO: ChainInfo = ChainInfo {
        chain_id: universal_chain_id_registry::ASTAR,
        ss58_prefix_raw: Some(5),
        xcm_address_type: AddressType::SS58,
        sig_scheme: SignatureScheme::Sr25519,
        evm_chain_id: Some(592),
        weth_addr: Some(EthAddress {
            0: hex!("Aeaaf0e2c81Af264101B9129C00F4440cCF0F720"),
        }), // WASTR
        avg_gas_fee_in_native_token: 300_000 * u128::pow(10, 9), // ASTR (18 decimals) -> basically free
        avg_bridge_fee_in_native_token: 200_000 * u128::pow(10, 9), // basically free
        rpc_url: "https://astar.public.blastapi.io", // author_submitExtrinsic fails, use private endpoint for live action
        // rpc_url: "https://astar.api.onfinality.io/rpc?apikey=[INSERT API KEY HERE]",
        subsquid_graphql_archive_url: "https://astar.explorer.subsquid.io/graphql",
    };
    pub const MOONBEAM_INFO: ChainInfo = ChainInfo {
        chain_id: universal_chain_id_registry::MOONBEAM,
        ss58_prefix_raw: Some(1284),
        xcm_address_type: AddressType::Ethereum,
        sig_scheme: SignatureScheme::Ethereum,
        evm_chain_id: Some(1284),
        weth_addr: Some(EthAddress {
            0: hex!("acc15dc74880c9944775448304b263d191c6077f"),
        }), // WGLMR
        avg_gas_fee_in_native_token: 12_000_000 * u128::pow(10, 9), // GLMR (18 decimals) -> 0.01 GLMR = ~$0.003
        avg_bridge_fee_in_native_token: 10_000_000 * u128::pow(10, 9), // ~$0.003
        rpc_url: "https://moonbeam.public.blastapi.io", // author_submitExtrinsic fails
        // rpc_url: "https://moonbeam.api.onfinality.io/rpc?apikey=[INSERT API KEY HERE]",
        subsquid_graphql_archive_url: "https://moonbeam.explorer.subsquid.io/graphql",
    };
    pub const POLKADOT_INFO: ChainInfo = ChainInfo {
        chain_id: universal_chain_id_registry::POLKADOT,
        ss58_prefix_raw: Some(0),
        xcm_address_type: AddressType::SS58,
        sig_scheme: SignatureScheme::Sr25519,
        evm_chain_id: None,
        weth_addr: None,
        // Gas estimate is from an xcmPallet transfer originating from Polkadot
        avg_gas_fee_in_native_token: 190_000_000, // DOT (10 decimals) -> 0.02 DOT = ~$0.10
        avg_bridge_fee_in_native_token: 500_000_000, // ~$0.24
        rpc_url: "https://polkadot.api.onfinality.io/rpc?apikey=[INSERT API KEY HERE]",
        subsquid_graphql_archive_url: "https://polkadot.explorer.subsquid.io/graphql",
    };

    pub const MOONBASEALPHA_INFO: ChainInfo = ChainInfo {
        chain_id: universal_chain_id_registry::MOONBASE_ALPHA,
        ss58_prefix_raw: Some(1287),
        xcm_address_type: AddressType::Ethereum,
        sig_scheme: SignatureScheme::Ethereum,
        evm_chain_id: Some(1287),
        weth_addr: Some(EthAddress {
            0: hex!("d909178cc99d318e4d46e7e66a972955859670e1"),
        }), // WDEV
        avg_gas_fee_in_native_token: 12_000_000 * u128::pow(10, 9), // GLMR (18 decimals) -> 0.01 GLMR = ~$0.003
        avg_bridge_fee_in_native_token: 10_000_000 * u128::pow(10, 9), // ~$0.003
        // Don't use: "https://rpc.api.moonbase.moonbeam.network", // doesn't support author_submitExtrinsic on HTTP (only WS)
        rpc_url: "https://moonbeam-alpha.api.onfinality.io/public",
        subsquid_graphql_archive_url: "https://moonbase.explorer.subsquid.io/graphql",
    };
    pub const MOONBASEBETA_INFO: ChainInfo = ChainInfo {
        chain_id: universal_chain_id_registry::MOONBASE_BETA,
        ss58_prefix_raw: Some(1287),
        xcm_address_type: AddressType::Ethereum,
        sig_scheme: SignatureScheme::Ethereum,
        evm_chain_id: None, // definitely has an EVM chain ID, I just don't know what it is
        weth_addr: None,
        avg_gas_fee_in_native_token: 12_000_000 * u128::pow(10, 9), // GLMR (18 decimals) -> 0.01 GLMR = ~$0.003
        avg_bridge_fee_in_native_token: 10_000_000 * u128::pow(10, 9), // ~$0.003
        rpc_url: "https://frag-moonbase-beta-rpc.g.moonbase.moonbeam.network",
        subsquid_graphql_archive_url: "",
    };
}
