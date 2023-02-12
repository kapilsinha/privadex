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
use privadex_routing::test_utilities::graph_factory;

#[test]
fn test_full_graph_create() {
    pink_extension_runtime::mock_ext::mock_all_ext();
    let graph = graph_factory::full_graph();
    debug_println!(
        "Full graph: # vertices = {}, # simple edges = {}, # multi edges = {}",
        graph.simple_graph.vertex_count(),
        graph.simple_graph.edge_count(),
        graph.edge_count()
    );
    assert!(graph.edge_count() > 0);
}

#[test]
fn test_medium_graph_create() {
    pink_extension_runtime::mock_ext::mock_all_ext();
    let graph = graph_factory::medium_graph();
    debug_println!(
        "Medium graph: # vertices = {}, # simple edges = {}, # multi edges = {}",
        graph.simple_graph.vertex_count(),
        graph.simple_graph.edge_count(),
        graph.edge_count()
    );
    assert!(graph.edge_count() > 0);
}

#[test]
fn test_small_graph_create() {
    pink_extension_runtime::mock_ext::mock_all_ext();
    let graph = graph_factory::small_graph();
    debug_println!(
        "Small graph: # vertices = {}, # simple edges = {}, # multi edges = {}",
        graph.simple_graph.vertex_count(),
        graph.simple_graph.edge_count(),
        graph.edge_count()
    );
    assert!(graph.edge_count() > 0);
}
