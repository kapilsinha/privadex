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

use ink_env::debug_println;
use std::fs::File;

use privadex_chain_metadata::common::UniversalChainId;
use privadex_chain_metadata::registry::chain::universal_chain_id_registry::{
    ASTAR, MOONBEAM, POLKADOT,
};
use privadex_routing::graph_builder::create_graph_from_chain_ids;

fn main() {
    pink_extension_runtime::mock_ext::mock_all_ext();

    let chain_ids: Vec<UniversalChainId> = vec![ASTAR, MOONBEAM, POLKADOT];
    let graph = create_graph_from_chain_ids(&chain_ids).unwrap();
    debug_println!("Vertex count: {}", graph.simple_graph.vertex_count());
    debug_println!("Edge count: {}", graph.simple_graph.edge_count());

    let mut f = File::create("example.dot").unwrap();
    let _ = graph.simple_graph.to_dot("example", &mut f);

    for k in graph.vertices.keys() {
        let token = graph.get_token(k);
        debug_println!("{:?}", token.unwrap().id);
    }
    debug_println!("Generated dot file!");
}
