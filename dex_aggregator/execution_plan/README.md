# PrivaDEX Execution Plan
This crate performs the following:
1. Defines ExecutionPlan: this will be used by the executor to maintain state
2. Converts RoutingSolution to ExecutionPlan

## Running unit tests
```bash
cargo test --features=test-utils -- --include-ignored --nocapture
```
Specifying the features above is critical! Otherwise tests will be filtered out.

## Running examples
```bash
cargo run --example privadex_compute_execution_plan
```
Above is an end-to-end test. It performs the below:
1. Query the GraphQL endpoints and generate a graph
2. Uses the single-path SOR to compute GraphSolution
3. Convert GraphSolution to ExecutionPlan and print it

## Docker testing guide
To run tests from a Docker container, start the Docker container (instructions in the root README file):
```bash
# Unit tests
root@<container-id>:/privadex/dex_aggregator/execution_plan# cargo test --features=test-utils -- --include-ignored --nocapture
# Examples
root@<container-id>:/privadex/dex_aggregator/execution_plan# cargo run --example privadex_compute_execution_plan
```
