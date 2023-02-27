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

    use super::DexId;
    use crate::common::{Dex, EthAddress};
    use crate::registry::chain::universal_chain_id_registry::{ASTAR, MOONBASE_ALPHA, MOONBEAM};

    pub const ARTHSWAP: Dex = Dex {
        id: DexId::Arthswap,
        chain_id: ASTAR,
        fee_bps: 30,
        graphql_url: "https://squid.subsquid.io/privadex-arthswap/v/v0/graphql",
        eth_dex_router: EthAddress {
            0: hex!("E915D2393a08a00c5A463053edD31bAe2199b9e7"),
        }, // PancakeRouter
    };
    pub const BEAMSWAP: Dex = Dex {
        id: DexId::Beamswap,
        chain_id: MOONBEAM,
        fee_bps: 30,
        graphql_url: "https://squid.subsquid.io/privadex-beamswap/v/v0/graphql",
        eth_dex_router: EthAddress {
            0: hex!("96b244391D98B62D19aE89b1A4dCcf0fc56970C7"),
        }, // Router02
    };
    pub const STELLASWAP: Dex = Dex {
        id: DexId::Stellaswap,
        chain_id: MOONBEAM,
        fee_bps: 25,
        graphql_url: "https://squid.subsquid.io/privadex-stellaswap/v/v0/graphql",
        eth_dex_router: EthAddress {
            0: hex!("70085a09d30d6f8c4ecf6ee10120d1847383bb57"),
        }, // StellaSwap: Router v2.1
    };

    pub const MOONBASE_UNISWAP: Dex = Dex {
        id: DexId::MoonbaseUniswap,
        chain_id: MOONBASE_ALPHA,
        fee_bps: 30,
        graphql_url: "",
        eth_dex_router: EthAddress {
            0: hex!("8a1932d6e26433f3037bd6c3a40c816222a6ccd4"),
        }, // Uniswap v2
    };
}
