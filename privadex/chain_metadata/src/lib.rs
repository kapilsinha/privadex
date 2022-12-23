#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

// #[macro_use]
// extern crate static_assertions;

pub mod bridge;
pub mod chain_info;
pub mod common;
pub mod registry;
pub mod utils;

use chain_info::ChainInfo;
use common::{UniversalChainId, Dex};
use registry::{
	chain::{chain_info_registry, universal_chain_id_registry},
	dex::dex_registry
};

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
        
		&universal_chain_id_registry::MOONBASE_ALPHA => Some(&chain_info_registry::MOONBASEALPHA_INFO),
		&universal_chain_id_registry::MOONBASE_BETA => Some(&chain_info_registry::MOONBASEBETA_INFO),
        _ => None,
    }
}

pub fn get_dexes_from_chain_id(chain_id: &UniversalChainId) -> Vec<&'static Dex> {
	match chain_id {
		&universal_chain_id_registry::ASTAR => vec![&dex_registry::ARTHSWAP],
		&universal_chain_id_registry::MOONBEAM => vec![&dex_registry::STELLASWAP, &dex_registry::BEAMSWAP],
		&universal_chain_id_registry::POLKADOT => vec![],
        
		&universal_chain_id_registry::MOONBASE_ALPHA => vec![&dex_registry::MOONBASE_UNISWAP],
		&universal_chain_id_registry::MOONBASE_BETA => vec![],
        _ => vec![],
    }
}