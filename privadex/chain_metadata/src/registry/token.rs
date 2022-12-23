pub mod universal_token_id_registry {
    use crate::common::{
		ChainTokenId,
		EthAddress,
		ERC20Token,
		XC20Token,
		UniversalChainId,
		UniversalTokenId
	};
    use crate::registry::chain::universal_chain_id_registry;
    
    pub const DOT_NATIVE: UniversalTokenId = UniversalTokenId{
		chain: universal_chain_id_registry::POLKADOT,
		id: ChainTokenId::Native,
	};
    
    // https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fwss.api.moonbeam.network#/assets
    pub const ASTR_MOONBEAM: UniversalTokenId = UniversalTokenId{
		chain: universal_chain_id_registry::MOONBEAM,
		id: ChainTokenId::XC20(
            XC20Token::from_asset_id(224_077_081_838_586_484_055_667_086_558_292_981_199)
        ),
	};
    pub const GLMR_NATIVE: UniversalTokenId = UniversalTokenId{
		chain: universal_chain_id_registry::MOONBEAM,
		id: ChainTokenId::Native,
	};
    pub const DOT_MOONBEAM: UniversalTokenId = UniversalTokenId{
		chain: universal_chain_id_registry::MOONBEAM,
		id: ChainTokenId::XC20(
            XC20Token::from_asset_id(42_259_045_809_535_163_221_576_417_993_425_387_648)
        ),
	};
    pub const USDT_MOONBEAM: UniversalTokenId = UniversalTokenId{
		chain: universal_chain_id_registry::MOONBEAM,
		id: ChainTokenId::XC20(
            XC20Token::from_asset_id(311_091_173_110_107_856_861_649_819_128_533_077_277)
        ),
	};

    // https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Frpc.astar.network#/assets
    pub const ASTR_NATIVE: UniversalTokenId = UniversalTokenId{
		chain: universal_chain_id_registry::ASTAR,
        id: ChainTokenId::Native,
	};
    pub const GLMR_ASTAR: UniversalTokenId = UniversalTokenId{
		chain: universal_chain_id_registry::ASTAR,
		id: ChainTokenId::XC20(
            XC20Token::from_asset_id(18_446_744_073_709_551_619)
        ),
	};
    pub const DOT_ASTAR: UniversalTokenId = UniversalTokenId{
		chain: universal_chain_id_registry::ASTAR,
		id: ChainTokenId::XC20(
            XC20Token::from_asset_id(340_282_366_920_938_463_463_374_607_431_768_211_455)
        ),
	};
    pub const USDT_ASTAR: UniversalTokenId = UniversalTokenId{
		chain: universal_chain_id_registry::ASTAR,
		id: ChainTokenId::XC20(
            XC20Token::from_asset_id(4_294_969_280)
        ),
	};

	pub static REGISTERED_XC20_TOKENS: [UniversalTokenId; 6] = [
		GLMR_ASTAR, DOT_ASTAR, USDT_ASTAR, // Astar XC20s
		ASTR_MOONBEAM, DOT_MOONBEAM, USDT_MOONBEAM // Moonbeam XC20s
	];

	pub fn chain_and_eth_addr_to_token(chain_id: UniversalChainId, addr: EthAddress) -> UniversalTokenId {
		let potential_xc20_token = UniversalTokenId{
			chain: chain_id,
			id: ChainTokenId::XC20(XC20Token::from_eth_address(addr))
		};
		if REGISTERED_XC20_TOKENS.contains(&potential_xc20_token) {
			potential_xc20_token
		} else {
			UniversalTokenId{
				chain: chain_id,
				id: ChainTokenId::ERC20(ERC20Token{addr})
			}
		}
	}
}

pub(crate) mod token_multilocation_spec_registry {
	use xcm::latest::{
		MultiLocation,
		Junction,
		Junctions,
	};

	use crate::common::TokenMultiLocationSpec;
	use crate::registry::chain::universal_chain_id_registry;
	use super::universal_token_id_registry;

	// I have more or less verified these MultiLocations manually via actual txns
	// but of course final testing is needed for each of these
	pub(crate) const DOT_NATIVE: TokenMultiLocationSpec = TokenMultiLocationSpec{
		token: universal_token_id_registry::DOT_NATIVE,
		token_asset_multilocation: MultiLocation { parents: 0, interior: Junctions::Here }
	};

	pub(crate) const ASTR_MOONBEAM: TokenMultiLocationSpec = TokenMultiLocationSpec{
		token: universal_token_id_registry::ASTR_MOONBEAM,
		token_asset_multilocation: MultiLocation { parents: 1, interior: Junctions::X2(
			Junction::Parachain(universal_chain_id_registry::MOONBEAM.get_parachain_id_unsafe()),
			Junction::PalletInstance(10)
		)}
	};
    pub(crate) const GLMR_NATIVE: TokenMultiLocationSpec = TokenMultiLocationSpec{
		token: universal_token_id_registry::GLMR_NATIVE,
		token_asset_multilocation: MultiLocation { parents: 0, interior: Junctions::X1(
			Junction::PalletInstance(10)
		)}
	};
    pub(crate) const DOT_MOONBEAM: TokenMultiLocationSpec = TokenMultiLocationSpec{
		token: universal_token_id_registry::DOT_MOONBEAM,
		token_asset_multilocation: MultiLocation { parents: 0, interior: Junctions::X2(
			Junction::Parachain(universal_chain_id_registry::MOONBEAM.get_parachain_id_unsafe()),
			Junction::PalletInstance(10)
		)}
	};

    pub(crate) const ASTR_NATIVE: TokenMultiLocationSpec = TokenMultiLocationSpec{
		token: universal_token_id_registry::ASTR_NATIVE,
		token_asset_multilocation: MultiLocation { parents: 0, interior: Junctions::Here }
	};
    pub(crate) const GLMR_ASTAR: TokenMultiLocationSpec = TokenMultiLocationSpec{
		token: universal_token_id_registry::GLMR_ASTAR,
		token_asset_multilocation: MultiLocation { parents: 1, interior: Junctions::X1(
			Junction::Parachain(universal_chain_id_registry::ASTAR.get_parachain_id_unsafe())
		)}
	};
    pub(crate) const DOT_ASTAR: TokenMultiLocationSpec = TokenMultiLocationSpec{
		token: universal_token_id_registry::DOT_ASTAR,
		token_asset_multilocation: MultiLocation { parents: 0, interior: Junctions::X1(
			Junction::Parachain(universal_chain_id_registry::ASTAR.get_parachain_id_unsafe())
		)}
	};
}
