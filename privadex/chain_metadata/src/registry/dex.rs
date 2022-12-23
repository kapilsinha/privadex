use core::fmt;
use scale::{Decode, Encode};


#[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum DexId {
    Arthswap,
    Beamswap,
    Stellaswap,
    MoonbaseUniswap,
}

impl fmt::Display for DexId {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
            Self::Arthswap => write!(f, "Arthswap"),
            Self::Beamswap => write!(f, "Beamswap"),
            Self::Stellaswap => write!(f, "Stellaswap"),
            Self::MoonbaseUniswap => write!(f, "Uniswap"),
		}
    }
}

pub mod dex_registry {
    use hex_literal::hex;
    
    use crate::common::{Dex, EthAddress};
    use crate::registry::chain::universal_chain_id_registry::{
        ASTAR, MOONBEAM, MOONBASE_ALPHA
    };
    use super::DexId;

    pub const ARTHSWAP: Dex = Dex{
        id: DexId::Arthswap,
        chain_id: ASTAR,
        fee_bps: 30,
        eth_dex_router: Some(EthAddress{0: hex!("E915D2393a08a00c5A463053edD31bAe2199b9e7")}) // PancakeRouter
    };
    pub const BEAMSWAP: Dex = Dex{
        id: DexId::Beamswap,
        chain_id: MOONBEAM,
        fee_bps: 30,
        eth_dex_router: Some(EthAddress{0: hex!("96b244391D98B62D19aE89b1A4dCcf0fc56970C7")}) // Router02
    };
    pub const STELLASWAP: Dex = Dex{
        id: DexId::Stellaswap,
        chain_id: MOONBEAM,
        fee_bps: 25,
        eth_dex_router: Some(EthAddress{0: hex!("70085a09d30d6f8c4ecf6ee10120d1847383bb57")}) // StellaSwap: Router v2.1
    };

    pub const MOONBASE_UNISWAP: Dex = Dex{
        id: DexId::MoonbaseUniswap,
        chain_id: MOONBASE_ALPHA,
        fee_bps: 30,
        eth_dex_router: Some(EthAddress{0: hex!("8a1932d6e26433f3037bd6c3a40c816222a6ccd4")}) // Uniswap v2
    };
}
