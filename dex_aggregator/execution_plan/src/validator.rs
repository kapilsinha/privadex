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

use crate::execution_plan::{EthDexSwapStep, ExecutionPlan, ExecutionStepEnum};

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ExecutionPlanValidationError {
    ExecutionPlanPathsLengthZero, // There are no ExecutionPaths in ExecutionPlan
    ExecutionPathLengthZero,      // An ExecutionPath has zero steps
    FirstStepHasNullAmount,       // Some ExecutionPath's first ExecutionStep has amount = None
    ConsecutiveSameDexSwaps,
    ConsecutiveWraps,
    ConsecutiveUnwraps,
    ConsecutiveWrapUnwrap,
    ConsecutiveUnwrapWrap,
    InvalidPrestartStep,
    InvalidPostendStep,
    SwapAfterWrap, // Wrap + Swap should be merged into a SwapETHForTokens swap
    WrapSrcDestAddressMismatch, // Wrap step's src and dest address must match
    UnexpectedEthSend, // We currently only expect this in the prestart and postend steps
    UnexpectedERC20Transfer, // We currently only expect this in the prestart and postend steps
    UnwrapAfterSwap, // Swap + Unwrap should be merged into a SwapTokensForETH swap
    UnwrapSrcDestAddressMismatch, // Unwrap step's src and dest address must match
}

// Used in the unit tests in graph_solution_to_execution_plan
pub fn validate_execution_plan(
    execution_plan: &ExecutionPlan,
) -> Result<(), ExecutionPlanValidationError> {
    if execution_plan.paths.is_empty() {
        return Err(ExecutionPlanValidationError::ExecutionPlanPathsLengthZero);
    }
    if execution_plan
        .paths
        .iter()
        .any(|exec_path| exec_path.steps.is_empty())
    {
        return Err(ExecutionPlanValidationError::ExecutionPathLengthZero);
    }
    let _ = match execution_plan.prestart_user_to_escrow_transfer.inner {
        ExecutionStepEnum::EthSend(_) => Ok(()),
        ExecutionStepEnum::ERC20Transfer(_) => Ok(()),
        _ => Err(ExecutionPlanValidationError::InvalidPrestartStep),
    }?;
    let _ = match execution_plan.postend_escrow_to_user_transfer.inner {
        ExecutionStepEnum::EthSend(_) => Ok(()),
        ExecutionStepEnum::ERC20Transfer(_) => Ok(()),
        _ => Err(ExecutionPlanValidationError::InvalidPostendStep),
    }?;

    for exec_path in execution_plan.paths.iter() {
        if exec_path.steps[0].get_amount_in().is_none() {
            // The first step's amount_in must be non-null
            return Err(ExecutionPlanValidationError::FirstStepHasNullAmount);
        }
        for step in exec_path.steps.iter() {
            let _ = match &step.inner {
                ExecutionStepEnum::EthWrap(step) => {
                    if step.common.src_addr != step.common.dest_addr {
                        Err(ExecutionPlanValidationError::WrapSrcDestAddressMismatch)
                    } else {
                        Ok(())
                    }
                }
                ExecutionStepEnum::EthUnwrap(step) => {
                    if step.common.src_addr != step.common.dest_addr {
                        Err(ExecutionPlanValidationError::UnwrapSrcDestAddressMismatch)
                    } else {
                        Ok(())
                    }
                }
                _ => Ok(()),
            }?;
        }

        // Iterator::array_chunks is elegant but only has nightly support, so we do a raw loop
        let num_steps = exec_path.steps.len();
        for i in 0..(num_steps - 1) {
            let cur_step = &exec_path.steps[i];
            let next_step = &exec_path.steps[i + 1];
            let _ = match (&cur_step.inner, &next_step.inner) {
                (ExecutionStepEnum::EthSend(_), _) | (_, ExecutionStepEnum::EthSend(_)) => {
                    Err(ExecutionPlanValidationError::UnexpectedEthSend)
                }
                (ExecutionStepEnum::ERC20Transfer(_), _)
                | (_, ExecutionStepEnum::ERC20Transfer(_)) => {
                    Err(ExecutionPlanValidationError::UnexpectedERC20Transfer)
                }

                (ExecutionStepEnum::EthWrap(_), ExecutionStepEnum::EthWrap(_)) => {
                    Err(ExecutionPlanValidationError::ConsecutiveWraps)
                }
                (ExecutionStepEnum::EthWrap(_), ExecutionStepEnum::EthUnwrap(_)) => {
                    Err(ExecutionPlanValidationError::ConsecutiveWrapUnwrap)
                }
                (ExecutionStepEnum::EthWrap(_), ExecutionStepEnum::EthDexSwap(_)) => {
                    Err(ExecutionPlanValidationError::SwapAfterWrap)
                }
                (ExecutionStepEnum::EthUnwrap(_), ExecutionStepEnum::EthUnwrap(_)) => {
                    Err(ExecutionPlanValidationError::ConsecutiveUnwraps)
                }
                (ExecutionStepEnum::EthUnwrap(_), ExecutionStepEnum::EthWrap(_)) => {
                    Err(ExecutionPlanValidationError::ConsecutiveUnwrapWrap)
                }
                (ExecutionStepEnum::EthDexSwap(_), ExecutionStepEnum::EthUnwrap(_)) => {
                    Err(ExecutionPlanValidationError::UnwrapAfterSwap)
                }
                (
                    ExecutionStepEnum::EthDexSwap(EthDexSwapStep {
                        dex_router_addr: router1,
                        ..
                    }),
                    ExecutionStepEnum::EthDexSwap(EthDexSwapStep {
                        dex_router_addr: router2,
                        ..
                    }),
                ) => {
                    if router1 == router2 {
                        Err(ExecutionPlanValidationError::ConsecutiveSameDexSwaps)
                    } else {
                        Ok(())
                    }
                }
                _ => Ok(()),
            }?;
        }
    }
    Ok(())
}
