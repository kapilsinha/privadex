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

use hex_literal::hex;
use ink_prelude::vec;

use privadex_chain_metadata::common::{
    ChainTokenId::{Native, ERC20, XC20},
    ERC20Token, EthAddress,
    UniversalChainId::SubstrateParachain,
    UniversalTokenId, XC20Token,
};
use privadex_chain_metadata::registry::{
    bridge::xcm_bridge_registry,
    chain::RelayChain::Polkadot,
    dex::dex_registry::{ARTHSWAP, BEAMSWAP, STELLASWAP},
};

use privadex_common::fixed_point::DecimalFixedPoint;
use privadex_routing::graph::{
    edge::{
        BridgeEdge::Xcm,
        ConstantProductAMMSwapEdge,
        Edge::{Bridge, Swap},
        SwapEdge::{
            Wrap,
            // Unwrap,
            CPMM,
        },
        WrapEdge, XCMBridgeEdge,
    },
    graph::{GraphPath, GraphSolution, SplitGraphPath},
};

#[cfg(feature = "test-utils")]
use privadex_routing::{
    smart_order_router::single_path_sor::{SORConfig, SinglePathSOR},
    test_utilities::graph_factory,
};

#[cfg(feature = "test-utils")]
pub(crate) const DUMMY_SRC_ADDR: EthAddress = EthAddress {
    0: hex!("fedcba98765432100123456789abcdef00010203"),
};
#[cfg(feature = "test-utils")]
pub(crate) const DUMMY_DEST_ADDR: EthAddress = EthAddress {
    0: hex!("000102030405060708090a0b0c0d0e0f10111213"),
};

/*
 * Below is the process to generate the below static graph construction (first turn off word wrap):
 * 1. cargo test test_sor_full -- --nocapture
 * 2. Add vec! in front of SplitGraphPath and after GraphPath
 * 3. Normal string replace Dex { ... } with a reference to const Dex:
 *    From: Dex { id: Stellaswap, chain_id: SubstrateParachain(Polkadot, 2004), fee_bps: 25, graphql_url: "https://squid.subsquid.io/privadex-stellaswap/v/v0/graphql", eth_dex_router: 0x70085a09d30d6f8c4ecf6ee10120d1847383bb57 }
 *    To: &STELLASWAP
 *    From: Dex { id: Arthswap, chain_id: SubstrateParachain(Polkadot, 2006), fee_bps: 30, graphql_url: "https://squid.subsquid.io/privadex-arthswap/v/v0/graphql", eth_dex_router: 0xe915d2393a08a00c5a463053edd31bae2199b9e7 }
 *    To: &ARTHSWAP
 *    From: Dex { id: Beamswap, chain_id: SubstrateParachain(Polkadot, 2004), fee_bps: 30, graphql_url: "https://squid.subsquid.io/privadex-beamswap/v/v0/graphql", eth_dex_router: 0x96b244391d98b62d19ae89b1a4dccf0fc56970c7 }
 *    TO: &BEAMSWAP
 * 4. Regex replace hex addr strings for EthAddress:
 *    From: 0x([0-9a-f]+)
 *    To: EthAddress{0: hex!("$1")}
 * 5. Regex replace XC20{ ... } with XC20::from_asset_id(...)
 *    From: XC20Token \{ asset_id: ([0-9]+) \}
 *    To: XC20Token::from_asset_id($1)
 * 6. Manually replace Bridge { ... } with XCMBridgeEdge::from_bridge_and_derived_quantities(...)
 */

pub fn graph_solution_full_static() -> GraphSolution {
    // DOT_ASTAR -> DOT_NATIVE
    let bridge_edge1 = XCMBridgeEdge::from_bridge_and_derived_quantities(
        xcm_bridge_registry::XCM_BRIDGES[5].clone(),
        &DecimalFixedPoint::from_str_and_exp("122.45", 2),
        &DecimalFixedPoint::from_str_and_exp("1.0", 1),
        &DecimalFixedPoint::from_str_and_exp("4.58", 2).add_exp(-10),
    );
    // DOT_NATIVE -> DOT_MOONBEAM
    let bridge_edge2 = XCMBridgeEdge::from_bridge_and_derived_quantities(
        xcm_bridge_registry::XCM_BRIDGES[6].clone(),
        &DecimalFixedPoint::from_str_and_exp("1.0", 1),
        &DecimalFixedPoint::from_str_and_exp("1287328338", 0),
        &DecimalFixedPoint::from_str_and_exp("4.58", 2).add_exp(-10),
    );
    GraphSolution {
        paths: vec![SplitGraphPath {
            path: GraphPath(vec![
                Swap(CPMM(ConstantProductAMMSwapEdge {
                    src_token: UniversalTokenId {
                        chain: SubstrateParachain(Polkadot, 2006),
                        id: ERC20(ERC20Token {
                            addr: EthAddress {
                                0: hex!("29f6e49c6e3397c3a84f715885f9f233a441165c"),
                            },
                        }),
                    },
                    dest_token: UniversalTokenId {
                        chain: SubstrateParachain(Polkadot, 2006),
                        id: ERC20(ERC20Token {
                            addr: EthAddress {
                                0: hex!("6a2d262d56735dba19dd70682b39f6be9a931d98"),
                            },
                        }),
                    },
                    token0: ERC20(ERC20Token {
                        addr: EthAddress {
                            0: hex!("29f6e49c6e3397c3a84f715885f9f233a441165c"),
                        },
                    }),
                    token1: ERC20(ERC20Token {
                        addr: EthAddress {
                            0: hex!("6a2d262d56735dba19dd70682b39f6be9a931d98"),
                        },
                    }),
                    reserve0: 17410180344594059755520,
                    reserve1: 16813114969,
                    estimated_gas_fee_in_dest_token: 11,
                    estimated_gas_fee_usd: 11270644754894,
                    dex: &ARTHSWAP,
                    pair_address: EthAddress {
                        0: hex!("cf83a3d83c1265780d9374e8a7c838fe22bd3dc6"),
                    },
                })),
                Swap(CPMM(ConstantProductAMMSwapEdge {
                    src_token: UniversalTokenId {
                        chain: SubstrateParachain(Polkadot, 2006),
                        id: ERC20(ERC20Token {
                            addr: EthAddress {
                                0: hex!("6a2d262d56735dba19dd70682b39f6be9a931d98"),
                            },
                        }),
                    },
                    dest_token: UniversalTokenId {
                        chain: SubstrateParachain(Polkadot, 2006),
                        id: XC20(XC20Token::from_asset_id(
                            340282366920938463463374607431768211455,
                        )),
                    },
                    token0: ERC20(ERC20Token {
                        addr: EthAddress {
                            0: hex!("6a2d262d56735dba19dd70682b39f6be9a931d98"),
                        },
                    }),
                    token1: XC20(XC20Token::from_asset_id(
                        340282366920938463463374607431768211455,
                    )),
                    reserve0: 25326566566,
                    reserve1: 58965001158180,
                    estimated_gas_fee_in_dest_token: 26232,
                    estimated_gas_fee_usd: 11270644754894,
                    dex: &ARTHSWAP,
                    pair_address: EthAddress {
                        0: hex!("f4119c3d9e65602bb34f2455644e45c98d29bb4b"),
                    },
                })),
                Bridge(Xcm(bridge_edge1)),
                Bridge(Xcm(bridge_edge2)),
                Swap(CPMM(ConstantProductAMMSwapEdge {
                    src_token: UniversalTokenId {
                        chain: SubstrateParachain(Polkadot, 2004),
                        id: XC20(XC20Token::from_asset_id(
                            42259045809535163221576417993425387648,
                        )),
                    },
                    dest_token: UniversalTokenId {
                        chain: SubstrateParachain(Polkadot, 2004),
                        id: ERC20(ERC20Token {
                            addr: EthAddress {
                                0: hex!("acc15dc74880c9944775448304b263d191c6077f"),
                            },
                        }),
                    },
                    token0: ERC20(ERC20Token {
                        addr: EthAddress {
                            0: hex!("acc15dc74880c9944775448304b263d191c6077f"),
                        },
                    }),
                    token1: XC20(XC20Token::from_asset_id(
                        42259045809535163221576417993425387648,
                    )),
                    reserve0: 87625774395904957087744,
                    reserve1: 70243543517614,
                    estimated_gas_fee_in_dest_token: 12000000000000000,
                    estimated_gas_fee_usd: 4142787920734278,
                    dex: &BEAMSWAP,
                    pair_address: EthAddress {
                        0: hex!("d8fbdef502770832e90a6352b275f20f38269b74"),
                    },
                })),
                Swap(CPMM(ConstantProductAMMSwapEdge {
                    src_token: UniversalTokenId {
                        chain: SubstrateParachain(Polkadot, 2004),
                        id: ERC20(ERC20Token {
                            addr: EthAddress {
                                0: hex!("acc15dc74880c9944775448304b263d191c6077f"),
                            },
                        }),
                    },
                    dest_token: UniversalTokenId {
                        chain: SubstrateParachain(Polkadot, 2004),
                        id: ERC20(ERC20Token {
                            addr: EthAddress {
                                0: hex!("818ec0a7fe18ff94269904fced6ae3dae6d6dc0b"),
                            },
                        }),
                    },
                    token0: ERC20(ERC20Token {
                        addr: EthAddress {
                            0: hex!("818ec0a7fe18ff94269904fced6ae3dae6d6dc0b"),
                        },
                    }),
                    token1: ERC20(ERC20Token {
                        addr: EthAddress {
                            0: hex!("acc15dc74880c9944775448304b263d191c6077f"),
                        },
                    }),
                    reserve0: 122125802565,
                    reserve1: 353749614708794790510592,
                    estimated_gas_fee_in_dest_token: 4136,
                    estimated_gas_fee_usd: 4142787920734278,
                    dex: &BEAMSWAP,
                    pair_address: EthAddress {
                        0: hex!("b929914b89584b4081c7966ac6287636f7efd053"),
                    },
                })),
                Swap(CPMM(ConstantProductAMMSwapEdge {
                    src_token: UniversalTokenId {
                        chain: SubstrateParachain(Polkadot, 2004),
                        id: ERC20(ERC20Token {
                            addr: EthAddress {
                                0: hex!("818ec0a7fe18ff94269904fced6ae3dae6d6dc0b"),
                            },
                        }),
                    },
                    dest_token: UniversalTokenId {
                        chain: SubstrateParachain(Polkadot, 2004),
                        id: ERC20(ERC20Token {
                            addr: EthAddress {
                                0: hex!("c9baa8cfdde8e328787e29b4b078abf2dadc2055"),
                            },
                        }),
                    },
                    token0: ERC20(ERC20Token {
                        addr: EthAddress {
                            0: hex!("818ec0a7fe18ff94269904fced6ae3dae6d6dc0b"),
                        },
                    }),
                    token1: ERC20(ERC20Token {
                        addr: EthAddress {
                            0: hex!("c9baa8cfdde8e328787e29b4b078abf2dadc2055"),
                        },
                    }),
                    reserve0: 18664652931,
                    reserve1: 76692893618149933056,
                    estimated_gas_fee_in_dest_token: 16986992011195,
                    estimated_gas_fee_usd: 4125948315331289,
                    dex: &STELLASWAP,
                    pair_address: EthAddress {
                        0: hex!("ac2657ba28768fe5f09052f07a9b7ea867a4608f"),
                    },
                })),
            ]),
            fraction_amount_in: 100000000000000000000,
            fraction_bps: 10000,
        }],
        amount_in: 100000000000000000000,
        src_addr: EthAddress {
            0: hex!("0000000000000000000000000000000000000000"),
        },
        dest_addr: EthAddress {
            0: hex!("0000000000000000000000000000000000000000"),
        },
    }
}

pub fn graph_solution_medium_static() -> GraphSolution {
    // DOT_MOONBEAM -> DOT_NATIVE
    let bridge_edge = XCMBridgeEdge::from_bridge_and_derived_quantities(
        xcm_bridge_registry::XCM_BRIDGES[7].clone(),
        &DecimalFixedPoint::from_str_and_exp("1287328338", 0),
        &DecimalFixedPoint::from_str_and_exp("1.0", 1),
        &DecimalFixedPoint::from_str_and_exp("4.58", 2).add_exp(-10),
    );
    GraphSolution {
        paths: vec![SplitGraphPath {
            path: GraphPath(vec![
                Swap(Wrap(WrapEdge {
                    src_token: UniversalTokenId {
                        chain: SubstrateParachain(Polkadot, 2004),
                        id: Native,
                    },
                    dest_token: UniversalTokenId {
                        chain: SubstrateParachain(Polkadot, 2004),
                        id: ERC20(ERC20Token {
                            addr: EthAddress {
                                0: hex!("acc15dc74880c9944775448304b263d191c6077f"),
                            },
                        }),
                    },
                    estimated_gas_fee_in_dest_token: 12000000000000000,
                    estimated_gas_fee_usd: 4125948315331289,
                })),
                Swap(CPMM(ConstantProductAMMSwapEdge {
                    src_token: UniversalTokenId {
                        chain: SubstrateParachain(Polkadot, 2004),
                        id: ERC20(ERC20Token {
                            addr: EthAddress {
                                0: hex!("acc15dc74880c9944775448304b263d191c6077f"),
                            },
                        }),
                    },
                    dest_token: UniversalTokenId {
                        chain: SubstrateParachain(Polkadot, 2004),
                        id: XC20(XC20Token::from_asset_id(
                            42259045809535163221576417993425387648,
                        )),
                    },
                    token0: ERC20(ERC20Token {
                        addr: EthAddress {
                            0: hex!("acc15dc74880c9944775448304b263d191c6077f"),
                        },
                    }),
                    token1: XC20(XC20Token::from_asset_id(
                        42259045809535163221576417993425387648,
                    )),
                    reserve0: 6310610688295264816463872,
                    reserve1: 5084579044703826,
                    estimated_gas_fee_in_dest_token: 9565115,
                    estimated_gas_fee_usd: 4125948315331289,
                    dex: &STELLASWAP,
                    pair_address: EthAddress {
                        0: hex!("a927e1e1e044ca1d9fe1854585003477331fe2af"),
                    },
                })),
                Bridge(Xcm(bridge_edge)),
            ]),
            fraction_amount_in: 100000000000000000000,
            fraction_bps: 10000,
        }],
        amount_in: 100000000000000000000,
        src_addr: EthAddress {
            0: hex!("0000000000000000000000000000000000000000"),
        },
        dest_addr: EthAddress {
            0: hex!("0000000000000000000000000000000000000000"),
        },
    }
}

#[cfg(feature = "test-utils")]
pub fn graph_solution_full(
    src_token_id: UniversalTokenId,
    dest_token_id: UniversalTokenId,
    amount_in: privadex_chain_metadata::common::Amount,
) -> GraphSolution {
    let graph = graph_factory::full_graph();
    let sor_config = SORConfig::default();
    let sor = SinglePathSOR::new(
        &graph,
        DUMMY_SRC_ADDR,
        DUMMY_DEST_ADDR,
        src_token_id,
        dest_token_id,
        sor_config,
    );
    sor.compute_graph_solution(amount_in)
        .expect("We expect a solution - likely found an invalid src or dest token")
}

#[cfg(test)]
mod graph_solution_factory_tests {
    #[cfg(feature = "test-utils")]
    use super::*;
    use crate::test_utilities::graph_solution_factory;
    use ink_env::debug_println;

    #[test]
    fn test_full_solution_static() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let graph_solution = graph_solution_factory::graph_solution_full_static();
        debug_println!("Full solution: {:?}", graph_solution);
        assert_eq!(graph_solution.paths.len(), 1);
        assert_eq!(graph_solution.paths[0].path.0.len(), 7);
    }

    #[test]
    fn test_medium_solution_static() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let graph_solution = graph_solution_factory::graph_solution_medium_static();
        debug_println!("Medium solution: {:?}", graph_solution);
        assert_eq!(graph_solution.paths.len(), 1);
        assert_eq!(graph_solution.paths[0].path.0.len(), 3);
    }

    #[cfg(feature = "test-utils")]
    #[test]
    fn test_full_solution_custom_similar_to_static() {
        // This test is identical to test_full_solution_static
        pink_extension_runtime::mock_ext::mock_all_ext();
        let amount_in = 100_000_000_000_000_000_000;
        let src_token_id = UniversalTokenId {
            chain: SubstrateParachain(Polkadot, 2006),
            id: ERC20(ERC20Token {
                addr: EthAddress {
                    0: hex!("29f6e49c6e3397c3a84f715885f9f233a441165c"),
                },
            }),
        };
        let dest_token_id = UniversalTokenId {
            chain: SubstrateParachain(Polkadot, 2004),
            id: ERC20(ERC20Token {
                addr: EthAddress {
                    0: hex!("c9baa8cfdde8e328787e29b4b078abf2dadc2055"),
                },
            }),
        };
        let graph_solution =
            graph_solution_factory::graph_solution_full(src_token_id, dest_token_id, amount_in);
        debug_println!("Full solution: {:?}", graph_solution);
        assert_eq!(graph_solution.paths.len(), 1);
        // assert_eq!(graph_solution.paths[0].path.0.len(), 7);
    }

    #[cfg(feature = "test-utils")]
    #[test]
    fn test_full_solution_wrap() {
        // xcGLMR (Astar) -> WGLMR (Moonbeam)
        pink_extension_runtime::mock_ext::mock_all_ext();
        let amount_in = 100_000_000_000_000_000_000;
        let src_token_id = UniversalTokenId {
            chain: SubstrateParachain(Polkadot, 2006),
            id: XC20(XC20Token::from_asset_id(18_446_744_073_709_551_619)),
        };
        let dest_token_id = UniversalTokenId {
            chain: SubstrateParachain(Polkadot, 2004),
            id: ERC20(ERC20Token {
                addr: EthAddress {
                    0: hex!("acc15dc74880c9944775448304b263d191c6077f"),
                },
            }),
        };
        let graph_solution =
            graph_solution_factory::graph_solution_full(src_token_id, dest_token_id, amount_in);
        debug_println!("Full solution: {}", graph_solution);
        assert_eq!(graph_solution.paths.len(), 1);
        assert_eq!(graph_solution.paths[0].path.0.len(), 2);
    }

    #[cfg(feature = "test-utils")]
    #[test]
    fn test_full_solution_unwrap() {
        // WGLMR (Moonbeam) -> xcGLMR (Astar)
        pink_extension_runtime::mock_ext::mock_all_ext();
        let amount_in = 100_000_000_000_000_000_000;
        let src_token_id = UniversalTokenId {
            chain: SubstrateParachain(Polkadot, 2004),
            id: ERC20(ERC20Token {
                addr: EthAddress {
                    0: hex!("acc15dc74880c9944775448304b263d191c6077f"),
                },
            }),
        };
        let dest_token_id = UniversalTokenId {
            chain: SubstrateParachain(Polkadot, 2006),
            id: XC20(XC20Token::from_asset_id(18_446_744_073_709_551_619)),
        };
        let graph_solution =
            graph_solution_factory::graph_solution_full(src_token_id, dest_token_id, amount_in);
        debug_println!("Full solution: {}", graph_solution);
        assert_eq!(graph_solution.paths.len(), 1);
        assert_eq!(graph_solution.paths[0].path.0.len(), 2);
    }
}
