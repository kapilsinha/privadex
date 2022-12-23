# PrivaDEX Routing
This crate performs the following:
1. Generate a graph by making GraphQL requests to the price feed and pulling from the chain_metadata registries.
2. Smart order router: find the optimal path from source to destination.

## Running examples
```bash
cargo run --example privadex_build_visualize_graph --features=dot
```
Above generated `generated_graph.dot`, which can then be fed into a Graphviz editor.
I have had best luck with the circo and fdp engines for clear visualization.
