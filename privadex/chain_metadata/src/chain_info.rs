use scale::{Decode, Encode};
use crate::common::{Amount, EthAddress, UniversalChainId};
use crate::utils::signature_scheme::SignatureScheme;


pub use ss58_registry::Ss58AddressFormat;

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
	pub evm_chain_id: Option<u32>,
	pub weth_addr: Option<EthAddress>,

	pub rpc_url: &'static str,
	pub subsquid_archive_url: &'static str,
}

impl ChainInfo {
    // I deliberately don't store Ss58AddressFormat so that ChainInfo is
    // const-constructible
    pub fn get_ss58_prefix(&self) -> Option<Ss58AddressFormat> {
        Some(Ss58AddressFormat::custom(self.ss58_prefix_raw?))
    }
}
