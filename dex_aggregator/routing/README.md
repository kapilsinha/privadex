# PrivaDEX Routing
This crate performs the following:
1. Generate a graph by making GraphQL requests to the price feed and pulling from the chain_metadata registries.
2. Smart order router: find the optimal path from source to destination.

## Running unit tests
```bash
cargo test --features=test-utils -- --include-ignored --nocapture
```
Specifying the features above is critical! Otherwise tests will be filtered out.
Note that the tests marked 'ignore' do take several seconds to run.

## Running integration tests
```bash
cargo test graph_create --features=test-utils -- --nocapture
```
Specifying the features above is critical! Otherwise the test will be filtered out.

## Running examples
```bash
cargo run --example privadex_build_visualize_graph --features=dot
```
Above generates `example.dot`, which can then be fed into a Graphviz editor.
I have had best luck with the circo and fdp engines for clear visualization.

## Docker testing guide
To run tests from a Docker container, start the Docker container (instructions in the root README file):
```bash
# Unit tests
root@<container-id>:/privadex/dex_aggregator/routing# cargo test --features=test-utils -- --include-ignored --nocapture
# Integration tests
root@<container-id>:/privadex/dex_aggregator/routing# cargo test graph_create --features=test-utils -- --nocapture
# Examples
# You can examine the outputted example.dot file in your favorite DOT graph visualizer
root@<container-id>:/privadex/dex_aggregator/routing# cargo run --example privadex_build_visualize_graph --features=dot
```