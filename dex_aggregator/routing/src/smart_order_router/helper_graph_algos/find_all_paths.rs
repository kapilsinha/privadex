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

use graphlib::VertexId;
use hashbrown::HashSet;
use ink_prelude::{vec, vec::Vec};

use crate::graph::{
    edge::Edge,
    graph::{Graph, GraphPathRef},
};

// Empirically, the pair with the longest path that I have found has a path of length 7: 3 swaps + bridge + 3 swaps
// e.g. 0x29F6e49c6E3397C3A84F715885F9F233A441165C (oUSD on Astar)
// to 0xc9BAA8cfdDe8E328787E29b4B078abf2DaDc2055 (BNB.multi on Moonbeam)
pub(crate) struct AllPathsFinderConfig {
    pub(crate) max_path_len: u8,
    pub(crate) max_num_bridges: u8,
    pub(crate) max_consecutive_swaps: u8,
}

impl Default for AllPathsFinderConfig {
    fn default() -> Self {
        Self {
            max_path_len: 8,
            max_num_bridges: 2,
            max_consecutive_swaps: 4,
        }
    }
}

struct StackEntry<'a> {
    pub vertex: VertexId, // can later change to another lookup or maybe borrowed Token
    pub edge: Option<&'a Edge>, // only None if root
    pub path_len: u8,
}

struct PathEntry<'a> {
    pub dest: VertexId,
    pub edge: &'a Edge,
}

// Based on the iterative solution in utils/graph_find_all_paths.py
// Note that each Edge is >1KB and there is no reason to clone them, so we return references
// to those Edges
pub(crate) fn find_all_paths<'a>(
    graph: &'a Graph,
    src: &'a VertexId,
    dest: &'a VertexId,
    config: &AllPathsFinderConfig,
) -> Vec<GraphPathRef<'a>> {
    let mut visited: HashSet<VertexId> = HashSet::new();
    let mut path: Vec<PathEntry<'a>> = Vec::new();
    let mut stack: Vec<StackEntry> = vec![StackEntry {
        vertex: src.clone(),
        edge: None,
        path_len: 0u8,
    }];

    let mut all_paths: Vec<GraphPathRef<'a>> = Vec::new();

    while !stack.is_empty() {
        let StackEntry {
            vertex: u,
            edge,
            path_len,
        } = stack.pop().expect("Stack is non-empty");

        for _ in 0..(path.len() - (path_len as usize)) {
            let PathEntry {
                dest: trimmed_vertex,
                edge: _,
            } = path.pop().expect("Path is non-empty");
            let _ = visited.remove(&trimmed_vertex);
        }

        if let Some(e) = edge {
            path.push(PathEntry { dest: u, edge: e });
        }
        let _ = visited.insert(u);

        if u == *dest {
            all_paths.push(GraphPathRef {
                0: path.iter().map(|p| p.edge).collect(),
            });
        } else {
            let num_bridges = get_num_bridges(&path);
            let num_consecutive_swaps = get_num_latest_consecutive_swaps(&path);
            for i in graph.simple_graph.out_neighbors(&u) {
                if !visited.contains(i) {
                    let edges = graph.get_edges(u, *i).expect("Edge exists in graph");
                    for edge in edges.iter() {
                        let should_consider_path = (path.len() < config.max_path_len as usize)
                            && (!edge.is_bridge() || num_bridges < config.max_num_bridges as usize)
                            && (!edge.is_swap()
                                || num_consecutive_swaps < config.max_consecutive_swaps as usize);
                        if should_consider_path {
                            stack.push(StackEntry {
                                vertex: i.clone(),
                                edge: Some(edge),
                                path_len: path.len() as u8,
                            });
                        }
                    }
                }
            }
        }
    }
    all_paths
}

fn get_num_bridges(path: &Vec<PathEntry>) -> usize {
    path.iter()
        .filter(|path_entry| path_entry.edge.is_bridge())
        .count()
}

// Example: path = [swap, bridge, swap, swap]. Output: 2
// path_length = 4
// consecutive_swap_prev_index = 1
// # consecutive swaps = 4 - (1 + 1)
fn get_num_latest_consecutive_swaps(path: &Vec<PathEntry>) -> usize {
    let consecutive_swap_prev_index = path
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, path_entry)| !path_entry.edge.is_swap())
        .next();
    if let Some((prev_index, _)) = consecutive_swap_prev_index {
        path.len() - (prev_index + 1)
    } else {
        path.len()
    }
}

#[cfg(test)]
mod find_all_paths_test {
    use hex_literal::hex;
    use ink_env::debug_println;

    use privadex_chain_metadata::{
        common::{
            ChainTokenId::{Native, ERC20},
            ERC20Token, EthAddress,
            UniversalChainId::SubstrateParachain,
            UniversalTokenId,
        },
        registry::{
            bridge::xcm_bridge_registry,
            chain::{universal_chain_id_registry::POLKADOT, RelayChain::Polkadot},
            token::universal_token_id_registry,
        },
    };
    use privadex_common::fixed_point::DecimalFixedPoint;

    use super::*;
    use crate::graph::edge::{BridgeEdge, SwapEdge, WrapEdge, XCMBridgeEdge};
    use crate::test_utilities::graph_factory;

    fn print_paths(all_paths: Vec<GraphPathRef>) {
        let num_paths = all_paths.len();
        debug_println!("{} paths total.", num_paths);
        for (i, path) in all_paths.into_iter().enumerate() {
            // Use the path Display, not Debug, trait since Display is printed on newlines
            // (easy to read)
            debug_println!("Path {} of {}: {}", i + 1, num_paths, path);
        }
    }

    fn summarize_paths(all_paths: Vec<GraphPathRef>) {
        let num_paths = all_paths.len();
        debug_println!("{} paths total.", num_paths);
        for (i, path) in all_paths.into_iter().enumerate() {
            debug_println!("Path {} of {} ({} edges):", i + 1, num_paths, path.0.len());
        }
    }

    #[test]
    fn test_path_num_bridges_helper() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let dummy_swap_edge = Edge::Swap(SwapEdge::Wrap(WrapEdge {
            src_token: universal_token_id_registry::ASTR_MOONBEAM,
            dest_token: universal_token_id_registry::ASTR_NATIVE,
            estimated_gas_fee_in_dest_token: 0,
            estimated_gas_fee_usd: 0,
        }));
        let dummy_bridge_edge = Edge::Bridge(BridgeEdge::Xcm(
            XCMBridgeEdge::from_bridge_and_derived_quantities(
                xcm_bridge_registry::XCM_BRIDGES[0].clone(),
                &DecimalFixedPoint::from_str_and_exp("0", 0),
                &DecimalFixedPoint::from_str_and_exp("0", 0),
                &DecimalFixedPoint::from_str_and_exp("0", 0),
            ),
        ));
        let path: Vec<PathEntry> = vec![
            PathEntry {
                dest: VertexId::new(100),
                edge: &dummy_swap_edge,
            },
            PathEntry {
                dest: VertexId::new(101),
                edge: &dummy_bridge_edge,
            },
            PathEntry {
                dest: VertexId::new(102),
                edge: &dummy_swap_edge,
            },
            PathEntry {
                dest: VertexId::new(103),
                edge: &dummy_bridge_edge,
            },
        ];
        let num_bridges = get_num_bridges(&path);
        assert_eq!(num_bridges, 2);
    }

    #[test]
    fn test_path_num_latest_consecutive_swaps_helper() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let dummy_swap_edge = Edge::Swap(SwapEdge::Wrap(WrapEdge {
            src_token: universal_token_id_registry::ASTR_MOONBEAM,
            dest_token: universal_token_id_registry::ASTR_NATIVE,
            estimated_gas_fee_in_dest_token: 0,
            estimated_gas_fee_usd: 0,
        }));
        let dummy_bridge_edge = Edge::Bridge(BridgeEdge::Xcm(
            XCMBridgeEdge::from_bridge_and_derived_quantities(
                xcm_bridge_registry::XCM_BRIDGES[0].clone(),
                &DecimalFixedPoint::from_str_and_exp("0", 0),
                &DecimalFixedPoint::from_str_and_exp("0", 0),
                &DecimalFixedPoint::from_str_and_exp("0", 0),
            ),
        ));
        {
            let path: Vec<PathEntry> = vec![
                PathEntry {
                    dest: VertexId::new(100),
                    edge: &dummy_swap_edge,
                },
                PathEntry {
                    dest: VertexId::new(101),
                    edge: &dummy_swap_edge,
                },
                PathEntry {
                    dest: VertexId::new(102),
                    edge: &dummy_swap_edge,
                },
            ];
            let num_consecutive_swaps = get_num_latest_consecutive_swaps(&path);
            assert_eq!(num_consecutive_swaps, 3);
        }
        {
            let path: Vec<PathEntry> = vec![
                PathEntry {
                    dest: VertexId::new(100),
                    edge: &dummy_swap_edge,
                },
                PathEntry {
                    dest: VertexId::new(101),
                    edge: &dummy_bridge_edge,
                },
                PathEntry {
                    dest: VertexId::new(102),
                    edge: &dummy_swap_edge,
                },
                PathEntry {
                    dest: VertexId::new(103),
                    edge: &dummy_bridge_edge,
                },
            ];
            let num_consecutive_swaps = get_num_latest_consecutive_swaps(&path);
            assert_eq!(num_consecutive_swaps, 0);
        }
        {
            let path: Vec<PathEntry> = vec![
                PathEntry {
                    dest: VertexId::new(101),
                    edge: &dummy_swap_edge,
                },
                PathEntry {
                    dest: VertexId::new(102),
                    edge: &dummy_bridge_edge,
                },
                PathEntry {
                    dest: VertexId::new(103),
                    edge: &dummy_swap_edge,
                },
                PathEntry {
                    dest: VertexId::new(104),
                    edge: &dummy_swap_edge,
                },
            ];
            let num_consecutive_swaps = get_num_latest_consecutive_swaps(&path);
            assert_eq!(num_consecutive_swaps, 2);
        }
    }

    #[test]
    fn test_find_all_paths_small_graph() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let graph = graph_factory::small_graph();
        debug_println!(
            "Small graph: # vertices = {}, # simple edges = {}, # multi edges = {}",
            graph.simple_graph.vertex_count(),
            graph.simple_graph.edge_count(),
            graph.edge_count()
        );

        let src = graph
            .get_vertex(&universal_token_id_registry::DOT_NATIVE)
            .expect("DOT native should be in the graph");
        let dest = graph
            .get_vertex(&universal_token_id_registry::GLMR_NATIVE)
            .expect("ASTR native should be in the graph");
        let all_paths = find_all_paths(&graph, src, dest, &AllPathsFinderConfig::default());
        assert_eq!(all_paths.len(), 1);
        print_paths(all_paths);
    }

    #[test]
    fn test_find_all_paths_medium_graph() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let graph = graph_factory::medium_graph();
        debug_println!(
            "Medium graph: # vertices = {}, # simple edges = {}, # multi edges = {}",
            graph.simple_graph.vertex_count(),
            graph.simple_graph.edge_count(),
            graph.edge_count()
        );

        let src = graph
            .get_vertex(&universal_token_id_registry::DOT_NATIVE)
            .expect("DOT native should be in the graph");
        let dest_id = UniversalTokenId {
            chain: SubstrateParachain(Polkadot, 2004),
            id: ERC20(ERC20Token {
                addr: EthAddress {
                    0: hex!("ab3f0245b83feb11d15aaffefd7ad465a59817ed"),
                },
            }),
        };
        let dest = graph
            .get_vertex(&dest_id)
            .expect("This ERC20 token should be in the graph");
        let all_paths = find_all_paths(&graph, src, dest, &AllPathsFinderConfig::default());
        assert_eq!(all_paths.len(), 1);
        print_paths(all_paths);
    }

    #[test]
    fn test_find_all_paths_medium_graph_no_paths() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let graph = graph_factory::medium_graph();
        debug_println!(
            "Medium graph: # vertices = {}, # simple edges = {}, # multi edges = {}",
            graph.simple_graph.vertex_count(),
            graph.simple_graph.edge_count(),
            graph.edge_count()
        );

        let src = graph
            .get_vertex(&universal_token_id_registry::DOT_NATIVE)
            .expect("DOT native should be in the graph");
        let dest = graph
            .get_vertex(&universal_token_id_registry::ASTR_NATIVE)
            .expect("ASTR native should be in the graph");
        let all_paths = find_all_paths(&graph, src, dest, &AllPathsFinderConfig::default());
        assert_eq!(all_paths.len(), 0);
        print_paths(all_paths);
    }

    #[test]
    fn test_find_all_paths_full_graph() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let graph = graph_factory::full_graph();
        debug_println!(
            "Full graph: # vertices = {}, # simple edges = {}, # multi edges = {}",
            graph.simple_graph.vertex_count(),
            graph.simple_graph.edge_count(),
            graph.edge_count()
        );

        for (src_id, dest_id) in [
            (
                UniversalTokenId {
                    chain: POLKADOT,
                    id: Native,
                },
                UniversalTokenId {
                    chain: SubstrateParachain(Polkadot, 2004),
                    id: ERC20(ERC20Token {
                        addr: EthAddress {
                            0: hex!("ab3f0245b83feb11d15aaffefd7ad465a59817ed"),
                        },
                    }),
                },
            ),
            (
                UniversalTokenId {
                    chain: SubstrateParachain(Polkadot, 2006),
                    id: ERC20(ERC20Token {
                        addr: EthAddress {
                            0: hex!("de2578edec4669ba7f41c5d5d2386300bcea4678"),
                        },
                    }),
                },
                UniversalTokenId {
                    chain: SubstrateParachain(Polkadot, 2004),
                    id: ERC20(ERC20Token {
                        addr: EthAddress {
                            0: hex!("818ec0a7fe18ff94269904fced6ae3dae6d6dc0b"),
                        },
                    }),
                },
            ),
        ] {
            let src = graph
                .get_vertex(&src_id)
                .expect("Src token should be in the graph");
            let dest = graph
                .get_vertex(&dest_id)
                .expect("Dest token should be in the graph");
            let all_paths = find_all_paths(&graph, src, dest, &AllPathsFinderConfig::default());
            assert!(all_paths.len() > 1);
            summarize_paths(all_paths);
        }
    }

    #[test]
    fn test_find_all_paths_full_graph_long_path() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let graph = graph_factory::full_graph();
        debug_println!(
            "Full graph: # vertices = {}, # simple edges = {}, # multi edges = {}",
            graph.simple_graph.vertex_count(),
            graph.simple_graph.edge_count(),
            graph.edge_count()
        );

        let src_id = UniversalTokenId {
            chain: SubstrateParachain(Polkadot, 2006),
            id: ERC20(ERC20Token {
                addr: EthAddress {
                    0: hex!("29F6e49c6E3397C3A84F715885F9F233A441165C"),
                },
            }),
        };
        let src = graph
            .get_vertex(&src_id)
            .expect("Token should be in the graph");
        let dest_id = UniversalTokenId {
            chain: SubstrateParachain(Polkadot, 2004),
            id: ERC20(ERC20Token {
                addr: EthAddress {
                    0: hex!("c9BAA8cfdDe8E328787E29b4B078abf2DaDc2055"),
                },
            }),
        };
        let dest = graph
            .get_vertex(&dest_id)
            .expect("This ERC20 token should be in the graph");

        let num_paths_all_config = {
            let config = AllPathsFinderConfig {
                max_path_len: 100,
                max_consecutive_swaps: 100,
                max_num_bridges: 100,
            };
            let all_paths = find_all_paths(&graph, src, dest, &config);
            let num_paths = all_paths.len();
            assert!(num_paths > 1);
            summarize_paths(all_paths);
            num_paths
        };

        let num_paths_filtered_config = {
            let config = AllPathsFinderConfig {
                max_path_len: 7,
                max_consecutive_swaps: 100,
                max_num_bridges: 100,
            };
            let all_paths = find_all_paths(&graph, src, dest, &config);
            let num_paths = all_paths.len();
            assert!(num_paths > 1);
            summarize_paths(all_paths);
            num_paths
        };
        assert!(num_paths_all_config > num_paths_filtered_config);
    }

    // This is a time-consuming test so we filter it out, but actually it loops over 3600 pairs in 11 seconds
    // - which is amazingly fast
    #[test]
    #[ignore]
    fn test_find_all_paths_full_graph_loop() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let graph = graph_factory::full_graph();
        debug_println!(
            "Full graph: # vertices = {}, # simple edges = {}, # multi edges = {}",
            graph.simple_graph.vertex_count(),
            graph.simple_graph.edge_count(),
            graph.edge_count()
        );

        // We sort tokens so this test runs in a deterministic order
        let tokens = {
            let mut v: Vec<&VertexId> = graph.vertices.values().collect();
            v.sort();
            v
        };

        for (i, &src) in tokens.iter().enumerate() {
            for (j, &dest) in tokens.iter().enumerate() {
                let all_paths = find_all_paths(&graph, src, dest, &AllPathsFinderConfig::default());
                // print_paths(all_paths);
                let min_path_len = all_paths
                    .iter()
                    .map(|path| path.0.len())
                    .min()
                    .unwrap_or_default();
                let max_path_len = all_paths
                    .iter()
                    .map(|path| path.0.len())
                    .max()
                    .unwrap_or_default();
                debug_println!(
                    "({}, {}): # paths = {}; min_path_len = {}, max_path_len = {}",
                    i,
                    j,
                    all_paths.len(),
                    min_path_len,
                    max_path_len
                );
                if min_path_len == 7 {
                    let path = all_paths
                        .iter()
                        .filter(|path| path.0.len() == 7)
                        .next()
                        .unwrap();
                    for edge in path.0.iter() {
                        debug_println!("  {:?}", edge);
                    }
                }
            }
        }
    }
}
