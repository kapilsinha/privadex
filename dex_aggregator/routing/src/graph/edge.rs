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
use scale::Encode;
use xcm::latest::MultiLocation;

use privadex_chain_metadata::{
    bridge::{WalletMultiLocationTemplate, XCMBridge},
    common::{
        Amount, ChainTokenId, Dex, EthAddress, UniversalChainId, UniversalTokenId,
        USD_AMOUNT_EXPONENT,
    },
    get_chain_info_from_chain_id,
};
use privadex_common::{fixed_point::DecimalFixedPoint, utils::general_utils::mul_ratio_u128};

use super::traits::QuoteGetter;

#[derive(Debug, Clone, Encode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Edge {
    Swap(SwapEdge),
    Bridge(BridgeEdge),
}

impl Edge {
    pub(crate) fn is_swap(&self) -> bool {
        if let Self::Swap(_) = self {
            true
        } else {
            false
        }
    }

    pub(crate) fn is_bridge(&self) -> bool {
        if let Self::Bridge(_) = self {
            true
        } else {
            false
        }
    }
}

impl fmt::Display for Edge {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Swap(x) => write!(f, "Swap[{}]", x),
            Self::Bridge(_) => write!(f, "Bridge"),
        }
    }
}

impl QuoteGetter for Edge {
    fn get_src_dest_token(&self) -> (&UniversalTokenId, &UniversalTokenId) {
        match self {
            Self::Swap(swap_edge) => swap_edge.get_src_dest_token(),
            Self::Bridge(bridge_edge) => bridge_edge.get_src_dest_token(),
        }
    }

    fn get_quote(&self, amount_in: Amount) -> Amount {
        match self {
            Self::Swap(swap_edge) => swap_edge.get_quote(amount_in),
            Self::Bridge(bridge_edge) => bridge_edge.get_quote(amount_in),
        }
    }

    fn get_quote_with_estimated_txn_fees(&self, amount_in: Amount) -> Amount {
        match self {
            Self::Swap(swap_edge) => swap_edge.get_quote_with_estimated_txn_fees(amount_in),
            Self::Bridge(bridge_edge) => bridge_edge.get_quote_with_estimated_txn_fees(amount_in),
        }
    }

    fn get_estimated_txn_fees_in_dest_token(&self) -> Amount {
        match self {
            Self::Swap(swap_edge) => swap_edge.get_estimated_txn_fees_in_dest_token(),
            Self::Bridge(bridge_edge) => bridge_edge.get_estimated_txn_fees_in_dest_token(),
        }
    }

    fn get_estimated_txn_fees_usd(&self) -> Amount {
        match self {
            Self::Swap(swap_edge) => swap_edge.get_estimated_txn_fees_usd(),
            Self::Bridge(bridge_edge) => bridge_edge.get_estimated_txn_fees_usd(),
        }
    }

    fn get_dest_chain_estimated_gas_fee_usd(&self) -> Amount {
        match self {
            Self::Swap(swap_edge) => swap_edge.get_dest_chain_estimated_gas_fee_usd(),
            Self::Bridge(bridge_edge) => bridge_edge.get_dest_chain_estimated_gas_fee_usd(),
        }
    }
}

#[derive(Debug, Clone, Encode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum SwapEdge {
    CPMM(ConstantProductAMMSwapEdge),
    Wrap(WrapEdge),
    Unwrap(UnwrapEdge),
    // StableswapAMMSwapEdge
    // ConcLiquidityAMMSwapEdge
}

impl SwapEdge {
    pub fn get_chain_id(&self) -> UniversalChainId {
        match self {
            Self::CPMM(edge) => edge.src_token.chain,
            Self::Wrap(edge) => edge.src_token.chain,
            Self::Unwrap(edge) => edge.src_token.chain,
        }
    }
}

impl fmt::Display for SwapEdge {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::CPMM(x) => write!(f, "CPMM_{}", x.dex.id),
            Self::Wrap(_) => write!(f, "Wrap"),
            Self::Unwrap(_) => write!(f, "Unwrap"),
        }
    }
}

impl QuoteGetter for SwapEdge {
    fn get_src_dest_token(&self) -> (&UniversalTokenId, &UniversalTokenId) {
        match self {
            SwapEdge::CPMM(cpmm_edge) => cpmm_edge.get_src_dest_token(),
            SwapEdge::Wrap(wrap_edge) => wrap_edge.get_src_dest_token(),
            SwapEdge::Unwrap(unwrap_edge) => unwrap_edge.get_src_dest_token(),
        }
    }

    fn get_quote(&self, amount_in: Amount) -> Amount {
        match self {
            SwapEdge::CPMM(cpmm_edge) => cpmm_edge.get_quote(amount_in),
            SwapEdge::Wrap(wrap_edge) => wrap_edge.get_quote(amount_in),
            SwapEdge::Unwrap(unwrap_edge) => unwrap_edge.get_quote(amount_in),
        }
    }

    fn get_quote_with_estimated_txn_fees(&self, amount_in: Amount) -> Amount {
        match self {
            SwapEdge::CPMM(cpmm_edge) => cpmm_edge.get_quote_with_estimated_txn_fees(amount_in),
            SwapEdge::Wrap(wrap_edge) => wrap_edge.get_quote_with_estimated_txn_fees(amount_in),
            SwapEdge::Unwrap(unwrap_edge) => {
                unwrap_edge.get_quote_with_estimated_txn_fees(amount_in)
            }
        }
    }

    fn get_estimated_txn_fees_in_dest_token(&self) -> Amount {
        match self {
            SwapEdge::CPMM(cpmm_edge) => cpmm_edge.get_estimated_txn_fees_in_dest_token(),
            SwapEdge::Wrap(wrap_edge) => wrap_edge.get_estimated_txn_fees_in_dest_token(),
            SwapEdge::Unwrap(unwrap_edge) => unwrap_edge.get_estimated_txn_fees_in_dest_token(),
        }
    }

    fn get_estimated_txn_fees_usd(&self) -> Amount {
        match self {
            SwapEdge::CPMM(cpmm_edge) => cpmm_edge.get_estimated_txn_fees_usd(),
            SwapEdge::Wrap(wrap_edge) => wrap_edge.get_estimated_txn_fees_usd(),
            SwapEdge::Unwrap(unwrap_edge) => unwrap_edge.get_estimated_txn_fees_usd(),
        }
    }

    fn get_dest_chain_estimated_gas_fee_usd(&self) -> Amount {
        match self {
            SwapEdge::CPMM(cpmm_edge) => cpmm_edge.get_dest_chain_estimated_gas_fee_usd(),
            SwapEdge::Wrap(wrap_edge) => wrap_edge.get_dest_chain_estimated_gas_fee_usd(),
            SwapEdge::Unwrap(unwrap_edge) => unwrap_edge.get_dest_chain_estimated_gas_fee_usd(),
        }
    }
}

#[derive(Debug, Clone, Encode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ConstantProductAMMSwapEdge {
    // Used for SOR
    pub src_token: UniversalTokenId,
    pub dest_token: UniversalTokenId,
    pub token0: ChainTokenId,
    pub token1: ChainTokenId,
    pub reserve0: Amount,
    pub reserve1: Amount,
    // derived value: chain_info.avg_gas_fee / dest_token.derivedEth
    pub estimated_gas_fee_in_dest_token: Amount,
    // Not used for routing but is useful downstream when executing a GraphSolution
    pub estimated_gas_fee_usd: Amount,

    // Token pair metadata needed for executor
    pub dex: &'static Dex,
    pub pair_address: EthAddress,
}

impl QuoteGetter for ConstantProductAMMSwapEdge {
    fn get_src_dest_token(&self) -> (&UniversalTokenId, &UniversalTokenId) {
        (&self.src_token, &self.dest_token)
    }

    fn get_quote(&self, amount_in: Amount) -> Amount {
        let (num_reserve, denom_reserve) = {
            if self.src_token.id == self.token0 && self.dest_token.id == self.token1 {
                (self.reserve1, self.reserve0)
            } else if self.src_token.id == self.token1 && self.dest_token.id == self.token0 {
                (self.reserve0, self.reserve1)
            } else {
                panic!(
                    "ConstantProductAMMSwapEdge src_token, dest_token do not match token0, token1"
                )
            }
        };

        let after_fee_bps = Amount::from(10_000 - self.dex.fee_bps);
        // Order of operations matters so we avoid int overflows!
        let denominator = denom_reserve + mul_ratio_u128(amount_in, after_fee_bps, 10_000);
        let part_numerator = mul_ratio_u128(num_reserve, after_fee_bps, 10_000);
        mul_ratio_u128(amount_in, part_numerator, denominator)
    }

    fn get_estimated_txn_fees_in_dest_token(&self) -> Amount {
        self.estimated_gas_fee_in_dest_token
    }

    fn get_estimated_txn_fees_usd(&self) -> Amount {
        self.estimated_gas_fee_usd
    }

    fn get_dest_chain_estimated_gas_fee_usd(&self) -> Amount {
        self.estimated_gas_fee_usd
    }
}

#[derive(Debug, Clone, Encode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct WrapEdge {
    pub src_token: UniversalTokenId, // Native
    pub dest_token: UniversalTokenId,
    pub estimated_gas_fee_in_dest_token: Amount,
    // Not used for routing but is useful downstream when executing a GraphSolution
    pub estimated_gas_fee_usd: Amount,
}

impl QuoteGetter for WrapEdge {
    fn get_src_dest_token(&self) -> (&UniversalTokenId, &UniversalTokenId) {
        (&self.src_token, &self.dest_token)
    }

    fn get_quote(&self, amount_in: Amount) -> Amount {
        amount_in
    }

    fn get_estimated_txn_fees_in_dest_token(&self) -> Amount {
        self.estimated_gas_fee_in_dest_token
    }

    fn get_estimated_txn_fees_usd(&self) -> Amount {
        self.estimated_gas_fee_usd
    }

    fn get_dest_chain_estimated_gas_fee_usd(&self) -> Amount {
        self.estimated_gas_fee_usd
    }
}

#[derive(Debug, Clone, Encode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct UnwrapEdge {
    pub src_token: UniversalTokenId,
    pub dest_token: UniversalTokenId,            // Native
    pub estimated_gas_fee_in_dest_token: Amount, // dest_token = native token
    // Not used for routing but is useful downstream when executing a GraphSolution
    pub estimated_gas_fee_usd: Amount,
}

impl QuoteGetter for UnwrapEdge {
    fn get_src_dest_token(&self) -> (&UniversalTokenId, &UniversalTokenId) {
        (&self.src_token, &self.dest_token)
    }

    fn get_quote(&self, amount_in: Amount) -> Amount {
        amount_in
    }

    fn get_estimated_txn_fees_in_dest_token(&self) -> Amount {
        self.estimated_gas_fee_in_dest_token
    }

    fn get_estimated_txn_fees_usd(&self) -> Amount {
        self.estimated_gas_fee_usd
    }

    fn get_dest_chain_estimated_gas_fee_usd(&self) -> Amount {
        self.estimated_gas_fee_usd
    }
}

#[derive(Debug, Clone, Encode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum BridgeEdge {
    Xcm(XCMBridgeEdge),
}

impl QuoteGetter for BridgeEdge {
    fn get_src_dest_token(&self) -> (&UniversalTokenId, &UniversalTokenId) {
        match self {
            BridgeEdge::Xcm(xcm_bridge_edge) => xcm_bridge_edge.get_src_dest_token(),
        }
    }

    fn get_quote(&self, amount_in: Amount) -> Amount {
        match self {
            BridgeEdge::Xcm(xcm_bridge_edge) => xcm_bridge_edge.get_quote(amount_in),
        }
    }

    fn get_quote_with_estimated_txn_fees(&self, amount_in: Amount) -> Amount {
        match self {
            BridgeEdge::Xcm(xcm_bridge_edge) => {
                xcm_bridge_edge.get_quote_with_estimated_txn_fees(amount_in)
            }
        }
    }

    fn get_estimated_txn_fees_in_dest_token(&self) -> Amount {
        match self {
            BridgeEdge::Xcm(xcm_bridge_edge) => {
                xcm_bridge_edge.get_estimated_txn_fees_in_dest_token()
            }
        }
    }

    fn get_estimated_txn_fees_usd(&self) -> Amount {
        match self {
            BridgeEdge::Xcm(xcm_bridge_edge) => xcm_bridge_edge.get_estimated_txn_fees_usd(),
        }
    }

    fn get_dest_chain_estimated_gas_fee_usd(&self) -> Amount {
        match self {
            BridgeEdge::Xcm(xcm_bridge_edge) => {
                xcm_bridge_edge.get_dest_chain_estimated_gas_fee_usd()
            }
        }
    }
}

#[derive(Debug, Clone, Encode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct XCMBridgeEdge {
    pub src_token: UniversalTokenId,
    pub dest_token: UniversalTokenId,
    // derived value: src_token.chain_info.avg_gas_fee / dest_token.derivedEth
    pub estimated_gas_fee_in_src_token: Amount,
    // Not used for routing but is useful downstream when executing a GraphSolution
    pub estimated_gas_fee_usd: Amount,
    // derived value: estimated_bridge_fee_in_native_token / dest_token.derivedEth
    pub estimated_bridge_fee_in_dest_token: Amount,
    // Not used for routing but is useful downstream when executing a GraphSolution
    pub estimated_bridge_fee_usd: Amount,

    estimated_dest_chain_gas_fee_usd: Amount,

    // XCM instruction and fee metadata needed for executor
    pub token_asset_multilocation: MultiLocation,
    pub dest_multilocation_template: WalletMultiLocationTemplate,
}

// Can change this to a generic trait when more bridge types are added
impl XCMBridgeEdge {
    pub fn from_bridge_and_derived_quantities(
        xcm_bridge: XCMBridge,
        src_token_derived_eth: &DecimalFixedPoint,
        dest_token_derived_eth: &DecimalFixedPoint,
        token_derived_usd: &DecimalFixedPoint,
    ) -> Self {
        let estimated_gas_fee_in_src_chain_native_token =
            get_chain_info_from_chain_id(&xcm_bridge.src_token.chain)
                .expect("XCM bridge must have an associated src ChainInfo")
                .avg_gas_fee_in_native_token;

        // # src_token_units = # src_native_token_units / (# src_native_token_units / # src_token_units)
        let estimated_gas_fee_in_src_token = DecimalFixedPoint::u128_div(
            estimated_gas_fee_in_src_chain_native_token,
            src_token_derived_eth,
        );
        let estimated_gas_fee_usd = token_derived_usd
            .add_exp(USD_AMOUNT_EXPONENT as i8)
            .mul_u128(estimated_gas_fee_in_src_token);
        // ink_env::debug_println!(
        //     "Token fee = {}, ${}, src_derived_eth={:?}",
        //     estimated_gas_fee_in_src_token,
        //     estimated_gas_fee_usd as f64 / (Amount::pow(10, 18) as f64),
        //     src_token_derived_eth
        // );

        // # dest_token_units = # dest_native_token_units / (# dest_native_token_units / # dest_token_units)
        let estimated_bridge_fee_in_dest_token = DecimalFixedPoint::u128_div(
            xcm_bridge.estimated_bridge_fee_in_dest_chain_native_token,
            dest_token_derived_eth,
        );
        let estimated_bridge_fee_usd = token_derived_usd
            .add_exp(USD_AMOUNT_EXPONENT as i8)
            .mul_u128(estimated_bridge_fee_in_dest_token);

        // This is NOT the gas fee that is paid because this is for the dest chain
        let estimated_dest_chain_gas_fee_in_dest_native_token =
            get_chain_info_from_chain_id(&xcm_bridge.dest_token.chain)
                .expect("XCM bridge must have an associated dest ChainInfo")
                .avg_gas_fee_in_native_token;
        let estimated_dest_chain_gas_fee_usd = DecimalFixedPoint::u128_mul_div(
            estimated_dest_chain_gas_fee_in_dest_native_token,
            &token_derived_usd.add_exp(USD_AMOUNT_EXPONENT as i8),
            dest_token_derived_eth,
        );

        Self {
            src_token: xcm_bridge.src_token,
            dest_token: xcm_bridge.dest_token,
            estimated_gas_fee_in_src_token,
            estimated_gas_fee_usd,
            estimated_bridge_fee_in_dest_token,
            estimated_bridge_fee_usd,
            estimated_dest_chain_gas_fee_usd,
            token_asset_multilocation: xcm_bridge.token_asset_multilocation,
            dest_multilocation_template: xcm_bridge.dest_multilocation_template,
        }
    }
}

impl QuoteGetter for XCMBridgeEdge {
    fn get_src_dest_token(&self) -> (&UniversalTokenId, &UniversalTokenId) {
        (&self.src_token, &self.dest_token)
    }

    fn get_quote(&self, amount_in: Amount) -> Amount {
        amount_in
    }

    fn get_estimated_txn_fees_in_dest_token(&self) -> Amount {
        self.estimated_gas_fee_in_src_token + self.estimated_bridge_fee_in_dest_token
    }

    fn get_estimated_txn_fees_usd(&self) -> Amount {
        self.estimated_gas_fee_usd + self.estimated_bridge_fee_usd
    }

    fn get_dest_chain_estimated_gas_fee_usd(&self) -> Amount {
        self.estimated_dest_chain_gas_fee_usd
    }
}

// Ensure that our new int implementation matches the output of our old float implementation
#[cfg(test)]
mod float_tests {
    use ink_env::debug_println;

    use super::*;

    fn quote_float(
        amount_in: Amount,
        num_reserve: Amount,
        denom_reserve: Amount,
        dex_fee_bps: Amount,
    ) -> Amount {
        let after_fee_bps = Amount::from(10_000 - dex_fee_bps);
        // Order of operations and casting to float (loss of precision) matters so we avoid int overflows!
        let numerator =
            (amount_in as f64) * (num_reserve as f64) * (after_fee_bps as f64) / 10_000.0;
        let denominator =
            (denom_reserve as f64) + ((amount_in as f64) * (after_fee_bps as f64) / 10_000.0);
        (numerator / denominator).round() as Amount
    }

    fn quote_int(
        amount_in: Amount,
        num_reserve: Amount,
        denom_reserve: Amount,
        dex_fee_bps: Amount,
    ) -> Amount {
        let after_fee_bps = Amount::from(10_000 - dex_fee_bps);
        // Order of operations matters so we avoid int overflows!
        let denominator = denom_reserve + mul_ratio_u128(amount_in, after_fee_bps, 10_000);
        let part_numerator = mul_ratio_u128(num_reserve, after_fee_bps, 10_000);
        mul_ratio_u128(amount_in, part_numerator, denominator)
    }

    #[test]
    fn test_quotes() {
        let num_reserve = 1_000_000_000_000_000_000_000_000_000_000_000;
        let denom_reserve = 5_000_000_000_000_000_000_000;
        let amount_in = 3_000_000_000_000_000_000_000_000_000_000_000;
        let quotef = quote_float(amount_in, num_reserve, denom_reserve, 9_970);
        let quotei = quote_int(amount_in, num_reserve, denom_reserve, 9_970);
        debug_println!("{}, {}", quotei, quotef);
    }
}
