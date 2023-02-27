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

use ink_prelude::vec::Vec;

use privadex_chain_metadata::{
    common::{Amount, Dex},
    registry::dex::DexId,
};
use privadex_routing::graph::edge::{
    ConstantProductAMMSwapEdge, Edge, SwapEdge, UnwrapEdge, WrapEdge, XCMBridgeEdge,
};

use crate::execution_plan::{DexRouterFunction, ExecutionStep, ExecutionStepEnum};

use super::common::GraphToExecConversionError;
use super::converter::get_uuid_and_increment_seed;
use super::helper_to_single_exec_step as exec_step_helper;

#[derive(Debug, Clone)]
pub(crate) struct ParseSwapState {
    pub start_idx: usize,
    pub started_with_wrap: bool,
}

pub(crate) enum ProcessHelperResult {
    NoChange,
    UpdateParseSwapState(ParseSwapState),
    NewExecStep(ExecutionStep),
}

pub(crate) fn process_xcm_bridge_edge(
    uuid_seed: &mut u128,
    edge: &XCMBridgeEdge,
    amount_in: &Option<Amount>,
    parse_swap_state: &Option<ParseSwapState>,
) -> Result<ProcessHelperResult, GraphToExecConversionError> {
    match parse_swap_state {
        None => {
            let xcm_transfer_step = exec_step_helper::convert_xcm_bridge_to_exec_step(
                &edge,
                get_uuid_and_increment_seed(uuid_seed),
                amount_in.clone(),
            );
            Ok(ProcessHelperResult::NewExecStep(ExecutionStep::new(
                ExecutionStepEnum::XCMTransfer(xcm_transfer_step),
            )))
        }
        Some(_) => Err(GraphToExecConversionError::UnexpectedStillProcessingSwap),
    }
}

pub(crate) fn process_wrap_edge(
    uuid_seed: &mut u128,
    edge: &WrapEdge,
    amount_in: &Option<Amount>,
    parse_swap_state: &Option<ParseSwapState>,
    start_idx: usize,
    next_dex_id: Option<DexId>,
) -> Result<ProcessHelperResult, GraphToExecConversionError> {
    match (next_dex_id, parse_swap_state) {
        (None, None) => {
            let wrap_step = exec_step_helper::convert_wrap_to_exec_step(
                edge,
                get_uuid_and_increment_seed(uuid_seed),
                amount_in.clone(),
            );
            Ok(ProcessHelperResult::NewExecStep(ExecutionStep::new(
                ExecutionStepEnum::EthWrap(wrap_step),
            )))
        }
        (Some(_), None) => Ok(ProcessHelperResult::UpdateParseSwapState(ParseSwapState {
            start_idx,
            started_with_wrap: true,
        })),
        (_, Some(_)) => Err(GraphToExecConversionError::UnexpectedStillProcessingSwap),
    }
}

pub(crate) fn process_unwrap_edge(
    uuid_seed: &mut u128,
    edge: &UnwrapEdge,
    amount_in: &Option<Amount>,
    parse_swap_state: &Option<ParseSwapState>,
    graph_path: &Vec<Edge>,
    end_idx: usize,
    is_next_step_swap: bool,
) -> Result<ProcessHelperResult, GraphToExecConversionError> {
    match (is_next_step_swap, parse_swap_state) {
        (false, None) => {
            let unwrap_step = exec_step_helper::convert_unwrap_to_exec_step(
                edge,
                get_uuid_and_increment_seed(uuid_seed),
                amount_in.clone(),
            );
            Ok(ProcessHelperResult::NewExecStep(ExecutionStep::new(
                ExecutionStepEnum::EthUnwrap(unwrap_step),
            )))
        }
        (false, Some(s)) => {
            if s.started_with_wrap {
                Err(GraphToExecConversionError::StartedWrapEndedUnwrap)
            } else {
                let cpmm_edges: Vec<&ConstantProductAMMSwapEdge> = graph_path[s.start_idx..end_idx]
                    .iter()
                    .map(|edge| {
                        if let Edge::Swap(SwapEdge::CPMM(x)) = edge {
                            x
                        } else {
                            panic!("Expect all these swap edges to be CPMMs");
                        }
                    })
                    .collect();
                let swap_step = exec_step_helper::convert_same_dex_swaps_to_exec_step(
                    &cpmm_edges,
                    get_uuid_and_increment_seed(uuid_seed),
                    amount_in.clone(),
                    DexRouterFunction::SwapExactTokensForETH,
                );
                Ok(ProcessHelperResult::NewExecStep(ExecutionStep::new(
                    ExecutionStepEnum::EthDexSwap(swap_step),
                )))
            }
        }
        (true, _) => Err(GraphToExecConversionError::UnexpectedSwapAfterUnwrap),
    }
}

pub(crate) fn process_cpmm_edge(
    uuid_seed: &mut u128,
    edge: &ConstantProductAMMSwapEdge,
    amount_in: &Option<Amount>,
    parse_swap_state: &Option<ParseSwapState>,
    graph_path: &Vec<Edge>,
    cur_idx: usize,
    next_dex_id: Option<DexId>,
    is_next_step_unwrap: bool,
) -> Result<ProcessHelperResult, GraphToExecConversionError> {
    let is_last_consecutive_swap = {
        let ConstantProductAMMSwapEdge {
            dex: Dex { id, .. },
            ..
        } = edge;
        !is_next_step_unwrap && (Some(*id) != next_dex_id)
    };

    match (is_last_consecutive_swap, parse_swap_state) {
        (false, Some(_)) => Ok(ProcessHelperResult::NoChange),
        (false, None) => Ok(ProcessHelperResult::UpdateParseSwapState(ParseSwapState {
            start_idx: cur_idx,
            started_with_wrap: false,
        })),
        (true, None) => {
            let swap_step = exec_step_helper::convert_same_dex_swaps_to_exec_step(
                &[edge],
                get_uuid_and_increment_seed(uuid_seed),
                amount_in.clone(),
                DexRouterFunction::SwapExactTokensForTokens,
            );
            Ok(ProcessHelperResult::NewExecStep(ExecutionStep::new(
                ExecutionStepEnum::EthDexSwap(swap_step),
            )))
        }
        (true, Some(s)) => {
            let dex_router_func = {
                if s.started_with_wrap {
                    DexRouterFunction::SwapExactETHForTokens
                } else {
                    DexRouterFunction::SwapExactTokensForTokens
                }
            };
            let cpmm_edges: Vec<&ConstantProductAMMSwapEdge> = graph_path[s.start_idx..cur_idx + 1]
                .iter()
                .map(|edge| {
                    if let Edge::Swap(SwapEdge::CPMM(x)) = edge {
                        x
                    } else {
                        panic!("Expect all these swap edges to be CPMMs");
                    }
                })
                .collect();
            let swap_step = exec_step_helper::convert_same_dex_swaps_to_exec_step(
                &cpmm_edges,
                get_uuid_and_increment_seed(uuid_seed),
                amount_in.clone(),
                dex_router_func,
            );
            Ok(ProcessHelperResult::NewExecStep(ExecutionStep::new(
                ExecutionStepEnum::EthDexSwap(swap_step),
            )))
        }
    }
}
