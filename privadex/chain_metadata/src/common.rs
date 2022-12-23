use core::{fmt, hash::Hash};
use hex_literal::hex;
use scale::{Decode, Encode};
use ss58_registry::Ss58AddressFormat;

use crate::{registry::{dex::DexId, chain::RelayChain}, utils::general_utils::slice_to_hex_string};


// We should allow only checked arithmetic. Can later wrap u128 into a struct that exposes just checked_* operations
pub type Amount = u128;
pub type AssetId = u128;
pub type ParachainId = u32;
pub use pink_web3::types::Address as EthAddress;
// Currently we hard-code 18 as the # decimals for every native token. Can revise this later if we add new
// chains where that is not true
pub const NATIVE_TOKEN_DECIMALS: u32 = 18;


#[derive(Debug, Eq, PartialEq)]
pub enum PublicError {
	BadBase58,
	BadLength,
	FormatNotAllowed,
	InvalidChecksum,
	InvalidFormat,
	InvalidHex,
	InvalidMultiLocationAddress,
	InvalidMultiLocationLength,
	InvalidNetwork,
	InvalidPath,
	InvalidPrefix,
	UnknownSs58AddressFormat(Ss58AddressFormat),
}
pub(crate) type Result<T> = core::result::Result<T, PublicError>;


#[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone, Hash)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum UniversalChainId {
    SubstrateRelayChain(RelayChain),
	// Note that the Chain ID below corresponds to the parachain ID,
	// NOT the EVM chain ID
	// You can look up the parachain_id at
	// polkadot.js.org/apps -> ChainState -> Storage -> parachainInfo.parachainId
    SubstrateParachain(RelayChain, ParachainId),
    // SubstrateStandalone(StandaloneChain),
    // EVM(ChainId),
}

impl fmt::Display for UniversalChainId {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::SubstrateRelayChain(_) => write!(f, "Relay"),
			Self::SubstrateParachain(_, parachain) => write!(f, "Para_{}", parachain),
		}
    }
}

impl UniversalChainId {
	pub const fn get_relay(&self) -> RelayChain {
		match self {
			Self::SubstrateRelayChain(relay) => *relay,
			Self::SubstrateParachain(relay, _) => *relay,
		}
	}
	
    pub const fn get_parachain_id(&self) -> Option<ParachainId> {
        if let UniversalChainId::SubstrateParachain(_, parachain_id) = self {
            Some(*parachain_id)
        } else { None }
    }

	pub const fn get_parachain_id_unsafe(&self) -> ParachainId {
        if let UniversalChainId::SubstrateParachain(_, parachain_id) = self {
            *parachain_id
        } else { panic!("Chain must be a parachain") }
    }
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, Hash)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct UniversalTokenId {
	pub chain: UniversalChainId,
	pub id: ChainTokenId,
}

impl fmt::Display for UniversalTokenId {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}[{}]", self.chain, self.id)
    }
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub(crate) struct TokenMultiLocationSpec {
	pub token: UniversalTokenId,
	// Token's MultiLocation from this chain's perspective
	pub token_asset_multilocation: xcm::latest::MultiLocation,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, Hash)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ChainTokenId {
	Native,
	ERC20(ERC20Token),
	XC20(XC20Token),
}

impl fmt::Display for ChainTokenId {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::Native => write!(f, "Native"),
			Self::ERC20(ERC20Token{addr}) => write!(f, "ERC20({})", slice_to_hex_string(&addr.0)),
			Self::XC20(xc20) => write!(f, "XC20({})", xc20.get_asset_id()),
		}
    }
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, Hash)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ERC20Token {
	pub addr: EthAddress,
}

// Astar and Moonbeam have the concept of XC-20 tokens
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, Hash)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct XC20Token {
	asset_id: AssetId,
}

impl XC20Token {
	pub const fn from_asset_id(asset_id: AssetId) -> Self {
		Self { asset_id }
	}

	pub fn from_eth_address(addr: EthAddress) -> Self {
		let suffix: [u8; 16] = addr.as_bytes()[4..].try_into().unwrap();
        let asset_id = AssetId::from_be_bytes(suffix);
		Self { asset_id }
	}

	pub fn get_asset_id(&self) -> AssetId {
		self.asset_id
	}

	// Logic is outlined in https://docs.moonbeam.network/builders/xcm/xc20/xc20/#calculate-xc20-address
	pub fn get_eth_address(&self) -> EthAddress {
		const PREFIX: [u8; 4] = hex!("FFFFFFFF");
		let suffix: [u8; 16] = self.asset_id.to_be_bytes();
		let x: [u8; 20] = [PREFIX.as_slice(), suffix.as_slice()].concat()
			.try_into()
			.expect("XC20 prefix + suffix must add to 20 bytes");
        EthAddress{0: x}
	}
}

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct Dex {
    pub id: DexId,
	pub chain_id: UniversalChainId,
	// DEX fee in basis points e.g. bps = 100 -> 1%. Will need to see if it applies to non constant-product AMM
	pub fee_bps: u16,
	pub graphql_url: &'static str,
    pub eth_dex_router: Option<EthAddress>,
}
