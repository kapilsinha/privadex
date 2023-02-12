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

use duplicate::duplicate_item;
use ink_prelude::{vec, vec::Vec};

use privadex_chain_metadata::{
    chain_info::{AddressType, ChainInfo},
    common::{Amount, UniversalAddress, UniversalTokenId},
    get_chain_info_from_chain_id,
    registry::chain::universal_chain_id_registry,
};
use privadex_common::uuid::Uuid;
use privadex_routing::graph::edge::{
    ConstantProductAMMSwapEdge, UnwrapEdge, WrapEdge, XCMBridgeEdge,
};

use crate::execution_plan::{
    CommonExecutionMeta, CrossChainStepStatus, DexRouterFunction, EthDexSwapStep, EthStepStatus,
    EthUnwrapStep, EthWrapStep, XCMTransferStep,
};

use super::common::{ESCROW_ASTAR_NATIVE_ADDRESS, ESCROW_ETH_ADDRESS, ESCROW_SUBSTRATE_PUBLIC_KEY};

// Converts a single wrap/unwrap edge into unwrap/wrap step. Note that generally,
// wraps/unwraps will be preceded or followed by DEX swaps, in which case we generate
// an EthDexSwapStep. We only generate a singleton wrap/unwrap step if there is no
// adjacent ConstantProductAMMSwapEdge
#[duplicate_item(
    edge_type      out_type        func_name;
    [WrapEdge]     [EthWrapStep]   [convert_wrap_to_exec_step];
    [UnwrapEdge]   [EthUnwrapStep] [convert_unwrap_to_exec_step];
)]
pub(crate) fn func_name(wrapper_edge: &edge_type, uuid: Uuid, amount: Option<Amount>) -> out_type {
    let chain = wrapper_edge.src_token.chain.clone();
    let chain_info =
        get_chain_info_from_chain_id(&chain).expect("Wrap must have an associated ChainInfo");

    let common = CommonExecutionMeta {
        src_addr: UniversalAddress::Ethereum(ESCROW_ETH_ADDRESS),
        dest_addr: UniversalAddress::Ethereum(ESCROW_ETH_ADDRESS),
        // We take just the first leg's estimated gas fee, with the (largely true)
        // assumption that the length of the path does not impact gas fee and that
        // gas fee is independent of the SwapEdge type (e.g. wrap and swap are the same).
        // - which is fine since we just save one estimated_gas_fee in ChainInfo
        gas_fee_native: chain_info.avg_gas_fee_in_native_token,
        gas_fee_usd: wrapper_edge.estimated_gas_fee_usd,
    };

    out_type {
        uuid,
        chain,
        amount,
        common,
        status: EthStepStatus::NotStarted,
    }
}

// Converts several swap edges from the same DEX to a single EthDexSwapStep e.g.
// [ConstantProductAMMSwapEdge{ dex = BEAMSWAP, ...}, ConstantProductAMMSwapEdge{ dex = BEAMSWAP, ...}]
// -> EthDexSwapStep{ dex_router_addr = BeamSwap.dex_router, ...}
// We start with panics that check our preconditions. These are just sanity checks. They are only triggered
// if the calling code is buggy.
pub(crate) fn convert_same_dex_swaps_to_exec_step(
    dex_swap_edges: &[&ConstantProductAMMSwapEdge],
    uuid: Uuid,
    amount_in: Option<Amount>,
    dex_router_func: DexRouterFunction,
) -> EthDexSwapStep {
    if dex_swap_edges.len() == 0 {
        panic!(
            "There must be nonzero DEX swap edges passed to convert_same_dex_swaps_to_exec_step"
        );
    }
    if dex_swap_edges
        .iter()
        .any(|edge| edge.dex.id != dex_swap_edges[0].dex.id)
    {
        panic!("All the edges' DEXes must be the same in convert_same_dex_swaps_to_exec_step");
    }

    let chain_info = get_chain_info_from_chain_id(&dex_swap_edges[0].dex.chain_id)
        .expect("DEX must have an associated ChainInfo");

    let common = CommonExecutionMeta {
        src_addr: UniversalAddress::Ethereum(ESCROW_ETH_ADDRESS),
        dest_addr: UniversalAddress::Ethereum(ESCROW_ETH_ADDRESS),
        // We take just the first leg's estimated gas fee, with the (largely true)
        // assumption that the length of the path does not impact gas fee and that
        // gas fee is independent of the SwapEdge type (e.g. wrap and swap are the same).
        // - which is fine since we just save one estimated_gas_fee in ChainInfo
        gas_fee_native: chain_info.avg_gas_fee_in_native_token,
        gas_fee_usd: dex_swap_edges[0].estimated_gas_fee_usd,
    };

    let dex_router_addr = dex_swap_edges[0].dex.eth_dex_router.clone();

    let token_path: Vec<UniversalTokenId> = {
        let mut path = vec![dex_swap_edges[0].src_token.clone()];
        path.extend(dex_swap_edges.iter().map(|edge| edge.dest_token.clone()));
        path
    };

    EthDexSwapStep {
        uuid,
        dex_router_addr,
        dex_router_func,
        token_path,
        amount_in,
        common,
        status: EthStepStatus::NotStarted,
    }
}

pub(crate) fn convert_xcm_bridge_to_exec_step(
    bridge_edge: &XCMBridgeEdge,
    uuid: Uuid,
    amount_in: Option<Amount>,
) -> XCMTransferStep {
    let src_chain_info = get_chain_info_from_chain_id(&bridge_edge.src_token.chain)
        .expect("Bridge must have an associated source ChainInfo");
    let dest_chain_info = get_chain_info_from_chain_id(&bridge_edge.dest_token.chain)
        .expect("Bridge must have an associated destination ChainInfo");

    let src_addr = get_escrow_send_xcm_address(&src_chain_info);
    let dest_addr = get_escrow_receive_xcm_address(&dest_chain_info);
    let full_dest_multilocation = bridge_edge
        .dest_multilocation_template
        .get_full_dest_multilocation(dest_addr.clone())
        .expect("const MultiLocation template was formatted incorrectly");

    let common = CommonExecutionMeta {
        src_addr,
        dest_addr,
        // We take just the first leg's estimated gas fee, with the (largely true)
        // assumption that the length of the path does not impact gas fee and that
        // gas fee is independent of the SwapEdge type (e.g. wrap and swap are the same).
        // - which is fine since we just save one estimated_gas_fee in ChainInfo
        gas_fee_native: src_chain_info.avg_gas_fee_in_native_token,
        gas_fee_usd: bridge_edge.estimated_gas_fee_usd,
    };

    XCMTransferStep {
        uuid,
        src_token: bridge_edge.src_token.clone(),
        dest_token: bridge_edge.dest_token.clone(),
        token_asset_multilocation: bridge_edge.token_asset_multilocation.clone(),
        full_dest_multilocation,
        amount_in,
        bridge_fee_native: dest_chain_info.avg_bridge_fee_in_native_token,
        bridge_fee_usd: bridge_edge.estimated_bridge_fee_usd,
        common,
        status: CrossChainStepStatus::NotStarted,
    }
}

fn get_escrow_send_xcm_address(chain_info: &ChainInfo) -> UniversalAddress {
    if chain_info.chain_id == universal_chain_id_registry::ASTAR {
        // Use ETH address because Astar EVM uses an EVM precompile for
        // XCM transfers
        return UniversalAddress::Ethereum(ESCROW_ETH_ADDRESS);
    }

    match chain_info.xcm_address_type {
        AddressType::Ethereum => UniversalAddress::Ethereum(ESCROW_ETH_ADDRESS),
        AddressType::SS58 => UniversalAddress::Substrate(ESCROW_SUBSTRATE_PUBLIC_KEY),
    }
}

fn get_escrow_receive_xcm_address(chain_info: &ChainInfo) -> UniversalAddress {
    if chain_info.chain_id == universal_chain_id_registry::ASTAR {
        return UniversalAddress::Substrate(ESCROW_ASTAR_NATIVE_ADDRESS);
    }

    match chain_info.xcm_address_type {
        AddressType::Ethereum => UniversalAddress::Ethereum(ESCROW_ETH_ADDRESS),
        AddressType::SS58 => UniversalAddress::Substrate(ESCROW_SUBSTRATE_PUBLIC_KEY),
    }
}
