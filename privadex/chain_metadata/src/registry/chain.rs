use scale::{Decode, Encode};


#[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone, Hash)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum RelayChain {
    Polkadot,
    Kusama,
    Westend,
    Rococo,
    MoonbaseRelay,
}

pub mod universal_chain_id_registry {
    use crate::common::UniversalChainId;
    use super::RelayChain;

    pub const MOONBEAM: UniversalChainId = UniversalChainId::SubstrateParachain(RelayChain::Polkadot, 2004);
    // Note that we will (for now) only associate with the EVM (not Native) addresses on Astar
    pub const ASTAR: UniversalChainId = UniversalChainId::SubstrateParachain(RelayChain::Polkadot, 2006);
    pub const POLKADOT: UniversalChainId = UniversalChainId::SubstrateRelayChain(RelayChain::Polkadot);
    
    pub const MOONBASE_ALPHA: UniversalChainId = UniversalChainId::SubstrateParachain(RelayChain::MoonbaseRelay, 1000);
    pub const MOONBASE_BETA: UniversalChainId = UniversalChainId::SubstrateParachain(RelayChain::MoonbaseRelay, 888);
    pub const KHALA: UniversalChainId = UniversalChainId::SubstrateParachain(RelayChain::Kusama, 2004);
}

pub mod chain_info_registry {
    use hex_literal::hex;

    use crate::chain_info::{AddressType, ChainInfo};
    use crate::utils::signature_scheme::SignatureScheme;
    use crate::common::EthAddress;
    use super::universal_chain_id_registry;
    // Note that Ss58AddressFormat::try_from("astar").ok() uses https://github.com/paritytech/ss58-registry
    // but to keep these const I have manually pulled the values

    pub const ASTAR_INFO: ChainInfo = ChainInfo{
        chain_id: universal_chain_id_registry::ASTAR,
        ss58_prefix_raw: Some(5),
        xcm_address_type: AddressType::SS58,
        sig_scheme: SignatureScheme::Sr25519,
        evm_chain_id: Some(592),
        weth_addr: Some(EthAddress{0: hex!("Aeaaf0e2c81Af264101B9129C00F4440cCF0F720")}), // WASTR
        rpc_url: "https://astar.public.blastapi.io",
        subsquid_archive_url: "https://astar.archive.subsquid.io/graphql",
    };
    pub const MOONBEAM_INFO: ChainInfo = ChainInfo{
        chain_id: universal_chain_id_registry::MOONBEAM,
        ss58_prefix_raw: Some(1284),
        xcm_address_type: AddressType::Ethereum,
        sig_scheme: SignatureScheme::Ethereum,
        evm_chain_id: Some(1284),
        weth_addr: Some(EthAddress{0: hex!("acc15dc74880c9944775448304b263d191c6077f")}), // WGLMR
        rpc_url: "https://moonbeam.public.blastapi.io",
        subsquid_archive_url: "https://moonbeam.archive.subsquid.io/graphql",
    };
    pub const POLKADOT_INFO: ChainInfo = ChainInfo{
        chain_id: universal_chain_id_registry::POLKADOT,
        ss58_prefix_raw: Some(0),
        xcm_address_type: AddressType::SS58,
        sig_scheme: SignatureScheme::Sr25519,
        evm_chain_id: None,
        weth_addr: None,
        // Gas estimate is from an xcmPallet transfer originating from Polkadot
        rpc_url: "https://polkadot.api.onfinality.io/rpc?apikey=3415143a-c3b4-42ae-8625-7613025ac69c",
        subsquid_archive_url: "https://polkadot.archive.subsquid.io/graphql",
    };

    pub const MOONBASEALPHA_INFO: ChainInfo = ChainInfo{
        chain_id: universal_chain_id_registry::MOONBEAM,
        ss58_prefix_raw: Some(1287),
        xcm_address_type: AddressType::Ethereum,
        sig_scheme: SignatureScheme::Ethereum,
        evm_chain_id: Some(1287),
        weth_addr: Some(EthAddress{0: hex!("d909178cc99d318e4d46e7e66a972955859670e1")}), // WDEV
        rpc_url: "https://moonbeam-alpha.api.onfinality.io/public",
        subsquid_archive_url: "https://moonbase.archive.subsquid.io/graphql",
    };
    pub const MOONBASEBETA_INFO: ChainInfo = ChainInfo{
        chain_id: universal_chain_id_registry::MOONBEAM,
        ss58_prefix_raw: Some(1287),
        xcm_address_type: AddressType::Ethereum,
        sig_scheme: SignatureScheme::Ethereum,
        evm_chain_id: None, // definitely has an EVM chain ID, I just don't know what it is
        weth_addr: None,
        rpc_url: "https://frag-moonbase-beta-rpc.g.moonbase.moonbeam.network",
        subsquid_archive_url: "",
    };
}
