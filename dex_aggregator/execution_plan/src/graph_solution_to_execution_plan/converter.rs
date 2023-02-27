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

use ink_prelude::{vec, vec::Vec};
use scale::Encode;

use privadex_chain_metadata::{
    common::{ChainTokenId, Dex, UniversalAddress},
    get_chain_info_from_chain_id,
};
use privadex_common::uuid::Uuid;
use privadex_routing::graph::{
    edge::{BridgeEdge, ConstantProductAMMSwapEdge, Edge, SwapEdge},
    graph::{GraphSolution, SplitGraphPath},
    traits::QuoteGetter,
};

use crate::execution_plan::{
    CommonExecutionMeta, ERC20TransferStep, EthSendStep, EthStepStatus, ExecutionPath,
    ExecutionPlan, ExecutionStep, ExecutionStepEnum,
};

use super::common::{GraphToExecConversionError, ESCROW_ETH_ADDRESS};
use super::helper_process_graph_edge::{
    self as process_graph_edge_helper, ParseSwapState, ProcessHelperResult,
};

impl TryFrom<GraphSolution> for ExecutionPlan {
    type Error = GraphToExecConversionError;

    fn try_from(graph_solution: GraphSolution) -> Result<Self, Self::Error> {
        if graph_solution.paths.len() == 0 {
            return Err(GraphToExecConversionError::GraphSolutionPathsLengthZero);
        }
        // We use a hash of the GraphSolution to generate UUIDs. This is deterministic
        // so identical GraphSolutions (including src_addr, path, dest_addr, amount)
        // will create clashing UUIDs. Honestly though if the state has not changed at
        // all, a user should not create identical swap requests (it's just self-destructive)
        // let uuid_seed = graph_solution.
        // In theory, there is a 1/2^128 probability that adding to this number causes an
        // overflow (as we populate the UUIDs for the individual execution steps) :[]
        let mut uuid_seed =
            u128::from_le_bytes(sp_core_hashing::blake2_128(&graph_solution.encode()));
        let exec_plan_uuid = get_uuid_and_increment_seed(&mut uuid_seed);

        let prestart_user_to_escrow_transfer = {
            let start_edge = graph_solution.paths[0]
                .path
                .0
                .first()
                .ok_or(GraphToExecConversionError::GraphPathLengthZero)?;
            let (token, _) = start_edge.get_src_dest_token();
            let chain_info = get_chain_info_from_chain_id(&token.chain)
                .ok_or(GraphToExecConversionError::NoChainInfo)?;

            let amount = Some(graph_solution.amount_in);
            let status = EthStepStatus::NotStarted;
            let common = CommonExecutionMeta {
                src_addr: UniversalAddress::Ethereum(graph_solution.src_addr.clone()),
                dest_addr: UniversalAddress::Ethereum(ESCROW_ETH_ADDRESS),
                gas_fee_native: chain_info.avg_gas_fee_in_native_token,
                gas_fee_usd: start_edge.get_dest_chain_estimated_gas_fee_usd(),
            };

            if token.id == ChainTokenId::Native {
                ExecutionStep::new(ExecutionStepEnum::EthSend(EthSendStep {
                    uuid: get_uuid_and_increment_seed(&mut uuid_seed),
                    chain: token.chain.clone(),
                    amount,
                    common,
                    status,
                }))
            } else {
                ExecutionStep::new(ExecutionStepEnum::ERC20Transfer(ERC20TransferStep {
                    uuid: get_uuid_and_increment_seed(&mut uuid_seed),
                    token: token.clone(),
                    amount,
                    common,
                    status,
                }))
            }
        };

        let postend_escrow_to_user_transfer = {
            let last_edge = graph_solution.paths[0]
                .path
                .0
                .last()
                .ok_or(GraphToExecConversionError::GraphPathLengthZero)?;
            let (_, token) = last_edge.get_src_dest_token();
            let chain_info = get_chain_info_from_chain_id(&token.chain)
                .ok_or(GraphToExecConversionError::NoChainInfo)?;

            let gas_fee_native = chain_info.avg_gas_fee_in_native_token;
            let gas_fee_usd = last_edge.get_dest_chain_estimated_gas_fee_usd();
            // We set amount later based on the outputs of the preceding steps
            let amount = None;
            let status = EthStepStatus::NotStarted;
            let common = CommonExecutionMeta {
                src_addr: UniversalAddress::Ethereum(ESCROW_ETH_ADDRESS),
                dest_addr: UniversalAddress::Ethereum(graph_solution.dest_addr.clone()),
                gas_fee_native,
                gas_fee_usd,
            };

            if token.id == ChainTokenId::Native {
                ExecutionStep::new(ExecutionStepEnum::EthSend(EthSendStep {
                    uuid: get_uuid_and_increment_seed(&mut uuid_seed),
                    chain: token.chain.clone(),
                    amount,
                    common,
                    status,
                }))
            } else {
                ExecutionStep::new(ExecutionStepEnum::ERC20Transfer(ERC20TransferStep {
                    uuid: get_uuid_and_increment_seed(&mut uuid_seed),
                    token: token.clone(),
                    amount,
                    common,
                    status,
                }))
            }
        };

        let paths = {
            let exec_paths: Result<Vec<ExecutionPath>, GraphToExecConversionError> = graph_solution
                .paths
                .into_iter()
                .map(|split_graph_path| {
                    split_graph_path_to_exec_path(&mut uuid_seed, split_graph_path)
                })
                .collect();
            exec_paths?
        };

        Ok(Self {
            uuid: exec_plan_uuid,
            paths,
            prestart_user_to_escrow_transfer,
            postend_escrow_to_user_transfer,
        })
    }
}

pub(super) fn get_uuid_and_increment_seed(uuid_seed: &mut u128) -> Uuid {
    let uuid = Uuid::new(uuid_seed.to_be_bytes());
    *uuid_seed += 1;
    uuid
}

fn split_graph_path_to_exec_path(
    uuid_seed: &mut u128,
    split_graph_path: SplitGraphPath,
) -> Result<ExecutionPath, GraphToExecConversionError> {
    let graph_path = &split_graph_path.path.0;
    let num_graph_steps = graph_path.len();

    if num_graph_steps == 0 {
        return Err(GraphToExecConversionError::GraphPathLengthZero);
    }
    let mut amount_in = Some(split_graph_path.fraction_amount_in);
    let mut parse_swap_state: Option<ParseSwapState> = None;
    let mut exec_steps: Vec<ExecutionStep> = vec![];

    for (i, step) in graph_path.iter().enumerate() {
        let (next_dex_id, is_next_step_unwrap) = {
            if i == num_graph_steps - 1 {
                (None, false)
            } else {
                match &graph_path[i + 1] {
                    &Edge::Swap(SwapEdge::CPMM(ConstantProductAMMSwapEdge {
                        dex: Dex { id, .. },
                        ..
                    })) => (Some(*id), false),
                    &Edge::Swap(SwapEdge::Unwrap(_)) => (None, true),
                    _ => (None, false),
                }
            }
        };
        let process_helper_result = match step {
            Edge::Bridge(BridgeEdge::Xcm(edge)) => {
                process_graph_edge_helper::process_xcm_bridge_edge(
                    uuid_seed,
                    edge,
                    &amount_in,
                    &parse_swap_state,
                )
            }
            Edge::Swap(SwapEdge::Wrap(edge)) => process_graph_edge_helper::process_wrap_edge(
                uuid_seed,
                edge,
                &amount_in,
                &parse_swap_state,
                i + 1,
                next_dex_id,
            ),
            Edge::Swap(SwapEdge::Unwrap(edge)) => process_graph_edge_helper::process_unwrap_edge(
                uuid_seed,
                edge,
                &amount_in,
                &parse_swap_state,
                graph_path,
                i,
                next_dex_id.is_some(),
            ),
            Edge::Swap(SwapEdge::CPMM(edge)) => process_graph_edge_helper::process_cpmm_edge(
                uuid_seed,
                edge,
                &amount_in,
                &parse_swap_state,
                graph_path,
                i,
                next_dex_id,
                is_next_step_unwrap,
            ),
        }?;
        match process_helper_result {
            ProcessHelperResult::NoChange => {}
            ProcessHelperResult::NewExecStep(new_exec_step) => {
                let _ = amount_in.take();
                let _ = parse_swap_state.take();
                exec_steps.push(new_exec_step);
            }
            ProcessHelperResult::UpdateParseSwapState(new_state) => {
                let _ = parse_swap_state.replace(new_state);
            }
        }
    }

    Ok(ExecutionPath {
        steps: exec_steps,
        amount_out: None,
    })
}

// Allow unused imports for convenience because several are used only
// when test_utils feature is enabled
#[allow(unused_imports)]
#[cfg(test)]
mod graph_solution_converter_tests {
    use hex_literal::hex;
    use ink_env::debug_println;
    use scale::Encode;

    use super::*;
    use crate::test_utilities::graph_solution_factory;
    use crate::validator::validate_execution_plan;
    use privadex_chain_metadata::{
        common::{
            Amount, ChainTokenId, ERC20Token, EthAddress, UniversalChainId::SubstrateParachain,
            UniversalTokenId, XC20Token, USD_AMOUNT_EXPONENT,
        },
        registry::chain::RelayChain::Polkadot,
    };

    #[cfg(feature = "test-utils")]
    fn validate_prestart_step(
        prestart_step: &ExecutionStep,
        src_token_id: &UniversalTokenId,
        amount_in: Amount,
    ) {
        match src_token_id.id {
            ChainTokenId::Native => {
                if let ExecutionStepEnum::EthSend(x) = &prestart_step.inner {
                    assert_eq!(x.chain, src_token_id.chain);
                    assert_eq!(x.amount.unwrap(), amount_in);
                    assert_eq!(
                        x.common.src_addr,
                        UniversalAddress::Ethereum(graph_solution_factory::DUMMY_SRC_ADDR)
                    );
                    assert_eq!(
                        x.common.dest_addr,
                        UniversalAddress::Ethereum(ESCROW_ETH_ADDRESS)
                    );
                    assert!(x.common.gas_fee_usd < 2 * Amount::pow(10, USD_AMOUNT_EXPONENT - 1)); // gas fee < 20 cents
                    assert!(x.common.gas_fee_usd > Amount::pow(10, USD_AMOUNT_EXPONENT - 5)); // gas fee > 0.001 cents
                    assert!(x.status == EthStepStatus::NotStarted);
                } else {
                    assert!(false)
                }
            }
            _ => {
                if let ExecutionStepEnum::ERC20Transfer(x) = &prestart_step.inner {
                    assert_eq!(&x.token, src_token_id);
                    assert_eq!(x.amount.unwrap(), amount_in);
                    assert_eq!(
                        x.common.src_addr,
                        UniversalAddress::Ethereum(graph_solution_factory::DUMMY_SRC_ADDR)
                    );
                    assert_eq!(
                        x.common.dest_addr,
                        UniversalAddress::Ethereum(ESCROW_ETH_ADDRESS)
                    );
                    assert!(x.common.gas_fee_usd < 2 * Amount::pow(10, USD_AMOUNT_EXPONENT - 1)); // gas fee < 20 cents
                    assert!(x.common.gas_fee_usd > Amount::pow(10, USD_AMOUNT_EXPONENT - 5)); // gas fee > 0.001 cents
                    assert!(x.status == EthStepStatus::NotStarted);
                } else {
                    assert!(false)
                }
            }
        }
    }

    #[cfg(feature = "test-utils")]
    fn validate_postend_step(postend_step: &ExecutionStep, dest_token_id: &UniversalTokenId) {
        match dest_token_id.id {
            ChainTokenId::Native => {
                if let ExecutionStepEnum::EthSend(x) = &postend_step.inner {
                    assert_eq!(x.chain, dest_token_id.chain);
                    assert!(x.amount.is_none());
                    assert_eq!(
                        x.common.src_addr,
                        UniversalAddress::Ethereum(ESCROW_ETH_ADDRESS)
                    );
                    assert_eq!(
                        x.common.dest_addr,
                        UniversalAddress::Ethereum(graph_solution_factory::DUMMY_DEST_ADDR)
                    );
                    assert!(x.common.gas_fee_usd < 2 * Amount::pow(10, USD_AMOUNT_EXPONENT - 1)); // gas fee < 20 cents
                    assert!(x.common.gas_fee_usd > Amount::pow(10, USD_AMOUNT_EXPONENT - 5)); // gas fee > 0.001 cents
                    assert!(x.status == EthStepStatus::NotStarted);
                } else {
                    assert!(false)
                }
            }
            _ => {
                if let ExecutionStepEnum::ERC20Transfer(x) = &postend_step.inner {
                    assert_eq!(&x.token, dest_token_id);
                    assert!(x.amount.is_none());
                    assert_eq!(
                        x.common.src_addr,
                        UniversalAddress::Ethereum(ESCROW_ETH_ADDRESS)
                    );
                    assert_eq!(
                        x.common.dest_addr,
                        UniversalAddress::Ethereum(graph_solution_factory::DUMMY_DEST_ADDR)
                    );
                    assert!(x.common.gas_fee_usd < 2 * Amount::pow(10, USD_AMOUNT_EXPONENT - 1)); // gas fee < 20 cents
                    assert!(x.common.gas_fee_usd > Amount::pow(10, USD_AMOUNT_EXPONENT - 5)); // gas fee > 0.001 cents
                    assert!(x.status == EthStepStatus::NotStarted);
                } else {
                    assert!(false)
                };
            }
        }
    }

    #[cfg(feature = "test-utils")]
    fn get_validated_graph_solution_and_exec_plan(
        src_token_id: UniversalTokenId,
        dest_token_id: UniversalTokenId,
        amount_in: Amount,
    ) -> (GraphSolution, ExecutionPlan) {
        let graph_solution =
            graph_solution_factory::graph_solution_full(src_token_id, dest_token_id, amount_in);
        debug_println!("{}", graph_solution);

        let exec_plan = ExecutionPlan::try_from(graph_solution.clone())
            .expect("Expect exec plan from graph solution");
        debug_println!("\n[{} bytes] {}", exec_plan.encoded_size(), exec_plan);

        assert!(exec_plan.encoded_size() < 4_000);
        assert_eq!(exec_plan.paths.len(), graph_solution.paths.len());
        let _ = validate_execution_plan(&exec_plan).expect("Expect no errors in ExecutionPlan");

        (graph_solution, exec_plan)
    }

    #[test]
    fn test_convert_graph_solution_medium_static() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let graph_solution = graph_solution_factory::graph_solution_medium_static();
        // debug_println!("Graph solution: {:?}", graph_solution);
        debug_println!("{}", graph_solution);
        let exec_plan = ExecutionPlan::try_from(graph_solution.clone())
            .expect("Expect exec plan from graph solution");
        debug_println!("\n[{} bytes] {}", exec_plan.encoded_size(), exec_plan);

        // Enforce tight constraint of 4 KB. In reality we are allowed up to 16 KB allocations
        assert!(exec_plan.encoded_size() < 4_000);
        assert_eq!(exec_plan.paths.len(), graph_solution.paths.len());
        let _ = validate_execution_plan(&exec_plan).expect("Expect no errors in ExecutionPlan");
    }

    #[test]
    fn test_convert_graph_solution_full_static() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let graph_solution = graph_solution_factory::graph_solution_full_static();
        // debug_println!("Graph solution: {:?}", graph_solution);
        debug_println!("{}", graph_solution);
        let exec_plan = ExecutionPlan::try_from(graph_solution.clone())
            .expect("Expect exec plan from graph solution");
        debug_println!("\n[{} bytes] {}", exec_plan.encoded_size(), exec_plan);

        // Enforce tight constraint of 4 KB. In reality we are allowed up to 16 KB allocations
        assert!(exec_plan.encoded_size() < 4_000);
        assert_eq!(exec_plan.paths.len(), graph_solution.paths.len());
        let _ = validate_execution_plan(&exec_plan).expect("Expect no errors in ExecutionPlan");
    }

    #[cfg(feature = "test-utils")]
    #[test]
    fn test_convert_graph_solution_full_same_as_static() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let amount_in = 100_000_000_000_000_000;
        let src_token_id = UniversalTokenId {
            chain: SubstrateParachain(Polkadot, 2006),
            id: ChainTokenId::ERC20(ERC20Token {
                addr: EthAddress {
                    0: hex!("29f6e49c6e3397c3a84f715885f9f233a441165c"),
                },
            }),
        };
        let dest_token_id = UniversalTokenId {
            chain: SubstrateParachain(Polkadot, 2004),
            id: ChainTokenId::ERC20(ERC20Token {
                addr: EthAddress {
                    0: hex!("c9baa8cfdde8e328787e29b4b078abf2dadc2055"),
                },
            }),
        };
        let (_, exec_plan) = get_validated_graph_solution_and_exec_plan(
            src_token_id.clone(),
            dest_token_id.clone(),
            amount_in,
        );

        validate_prestart_step(
            &exec_plan.prestart_user_to_escrow_transfer,
            &src_token_id,
            amount_in,
        );
        validate_postend_step(&exec_plan.postend_escrow_to_user_transfer, &dest_token_id);
    }

    #[cfg(feature = "test-utils")]
    #[test]
    fn test_convert_graph_solution_full_wrap() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let amount_in = 100_000_000_000_000_000_000;
        let src_token_id = UniversalTokenId {
            chain: SubstrateParachain(Polkadot, 2006),
            id: ChainTokenId::XC20(XC20Token::from_asset_id(18_446_744_073_709_551_619)),
        };
        let dest_token_id = UniversalTokenId {
            chain: SubstrateParachain(Polkadot, 2004),
            id: ChainTokenId::ERC20(ERC20Token {
                addr: EthAddress {
                    0: hex!("acc15dc74880c9944775448304b263d191c6077f"),
                },
            }),
        };
        let (_, exec_plan) = get_validated_graph_solution_and_exec_plan(
            src_token_id.clone(),
            dest_token_id.clone(),
            amount_in,
        );

        validate_prestart_step(
            &exec_plan.prestart_user_to_escrow_transfer,
            &src_token_id,
            amount_in,
        );
        validate_postend_step(&exec_plan.postend_escrow_to_user_transfer, &dest_token_id);
    }

    #[cfg(feature = "test-utils")]
    #[test]
    fn test_convert_graph_solution_full_unwrap() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let amount_in = 100_000_000_000_000_000_000;
        let src_token_id = UniversalTokenId {
            chain: SubstrateParachain(Polkadot, 2004),
            id: ChainTokenId::ERC20(ERC20Token {
                addr: EthAddress {
                    0: hex!("acc15dc74880c9944775448304b263d191c6077f"),
                },
            }),
        };
        let dest_token_id = UniversalTokenId {
            chain: SubstrateParachain(Polkadot, 2006),
            id: ChainTokenId::XC20(XC20Token::from_asset_id(18_446_744_073_709_551_619)),
        };
        let (_, exec_plan) = get_validated_graph_solution_and_exec_plan(
            src_token_id.clone(),
            dest_token_id.clone(),
            amount_in,
        );

        validate_prestart_step(
            &exec_plan.prestart_user_to_escrow_transfer,
            &src_token_id,
            amount_in,
        );
        validate_postend_step(&exec_plan.postend_escrow_to_user_transfer, &dest_token_id);
    }

    // This is a time-consuming test so we filter it out, but it loops over 3600 pairs in 12 seconds
    #[cfg(feature = "test-utils")]
    #[test]
    #[ignore]
    fn test_convert_graph_solution_full_loop() {
        use privadex_common::fixed_point::DecimalFixedPoint;

        pink_extension_runtime::mock_ext::mock_all_ext();
        let graph = privadex_routing::test_utilities::graph_factory::full_graph();
        debug_println!(
            "Full graph: # vertices = {}, # simple edges = {}, # multi edges = {}",
            graph.simple_graph.vertex_count(),
            graph.simple_graph.edge_count(),
            graph.edge_count()
        );

        // We sort tokens so this test runs in a deterministic order
        let tokens = {
            let mut v: Vec<&UniversalTokenId> = graph.vertices.keys().collect();
            v.sort();
            v
        };

        for (i, &src_token_id) in tokens.iter().enumerate() {
            for (j, &dest_token_id) in tokens.iter().enumerate() {
                if src_token_id == dest_token_id {
                    continue;
                }
                let amount_in = {
                    let src_derived_usd = &graph
                        .get_token(src_token_id)
                        .expect("Src token is in the graph")
                        .derived_usd;
                    // Start with $1000 of src_token
                    DecimalFixedPoint::u128_div(1000, &src_derived_usd)
                };

                // We can technically use get_validated_graph_solution_and_exec_plan(...) to generate the
                // GraphSolution and ExecutionPlan, but that encodes a latent dependence that the graph used
                // in graph_solution_factory matches the graph used in graph_factory. To avoid that, we just
                // compute the GraphSolution for this path here.
                let graph_solution = {
                    let sor_config =
                        privadex_routing::smart_order_router::single_path_sor::SORConfig::default();
                    let sor =
                        privadex_routing::smart_order_router::single_path_sor::SinglePathSOR::new(
                            &graph,
                            graph_solution_factory::DUMMY_SRC_ADDR,
                            graph_solution_factory::DUMMY_DEST_ADDR,
                            src_token_id.clone(),
                            dest_token_id.clone(),
                            sor_config,
                        );
                    sor.compute_graph_solution(amount_in)
                        .expect("We expect a solution")
                };
                let exec_plan = ExecutionPlan::try_from(graph_solution.clone())
                    .expect("Expect exec plan from graph solution");
                debug_println!(
                    "({}, {}):\n GraphSolution: num_edges = {},\n ExecutionPlan: num_steps = {}, encoded_size = {} bytes\
                     \n  {:?}\n  ->\n  {:?}\n\
                     {}\n",
                    i, j,
                    graph_solution.paths[0].path.0.len(),
                    exec_plan.paths[0].steps.len(), exec_plan.encoded_size(),
                    src_token_id, dest_token_id,
                    exec_plan,
                );

                assert!(exec_plan.encoded_size() < 4_000);
                assert_eq!(exec_plan.paths.len(), graph_solution.paths.len());

                validate_prestart_step(
                    &exec_plan.prestart_user_to_escrow_transfer,
                    &src_token_id,
                    amount_in,
                );
                validate_postend_step(&exec_plan.postend_escrow_to_user_transfer, &dest_token_id);
                let _ =
                    validate_execution_plan(&exec_plan).expect("Expect no errors in ExecutionPlan");
            }
        }
    }
}
