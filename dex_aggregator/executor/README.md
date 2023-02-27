# PrivaDEX Executor

This crate performs the following:

1. Implement Substrate utilities to submit Substrate extrinsics and parse finalized extrinsics and events.
2. Implement Ethereum utilities to create and submit Ethereum transactions (including smart contract calls) and parse finalized transactions.
3. Define concurrency/coordination primitives to ensure multiple workers correctly manage a single account nonce and execute all ongoing swaps.
4. Perform the "step forward" function for an arbitrary execution plan, requiring no on-chain state updates.

## Building locally

NOTE: The default clang on Mac fails with secp256k1 errors when compiling the sp-core dependency with 'full_crypto' feature set is enabled or when compiling the pink-web3 dependency.
To avoid this, use the llvm clang and AR i.e. do the following before running `cargo contract build`:

```bash
export CC=/usr/local/opt/llvm/bin/clang; export AR=/usr/local/opt/llvm/bin/llvm-ar
```

## Running unit tests

There are unit tests defined in several of the source files. Note that they are not true unit tests in that they perform over-the-network functionality instead of mocking them. These can be run as the following.

```bash
# Note that you need to set ETH_PRIVATE_KEY and SUBSTRATE_PRIVATE_KEY for the unit tests
cargo test
# You need to set env variables for S3_ACCESS_KEY and S3_SECRET_KEY
cargo test --features=s3-live-test -- --nocapture
# You need to set env variables for DYNAMODB_ACCESS_KEY and DYNAMODB_SECRET_KEY, and
# set up the appropriate tables
cargo test --features=dynamodb-live-test -- --nocapture
# You need to replace instances of [INSERT API KEY HERE] with private RPC endpoints
cargo test --features=private-rpc-endpoint -- --nocapture
cargo test --features=mock-txn-send executable -- --nocapture
```

## Running examples
```bash
# Note that these examples send real transactions and thus require actual funds
# and approved tokens on DEXes. Sample outputs from my runs are in the out
# directory
cargo run --example privadex_send_xtokens_extrinsic
cargo run --example privadex_execute_static_plan_moonbase_alpha
cargo run --example privadex_execute_static_plan_mainnets
cargo run --example privadex_e2e_execute_plan_mainnets
```

## Docker testing guide

To run tests and build the WASM contract (to test via [Phat Contract UI](https://phat.phala.network/)) from a Docker container, start the Docker container (instructions in the root README file):

```bash
# Unit tests
root@<container-id>:/privadex/dex_aggregator/executor# cargo test
# Examples
root@<container-id>:/privadex/dex_aggregator/executor# cargo run --example privadex_send_xtokens_extrinsic
root@<container-id>:/privadex/dex_aggregator/executor# cargo run --example privadex_execute_static_plan_moonbase_alpha
root@<container-id>:/privadex/dex_aggregator/executor# cargo run --example privadex_execute_static_plan_mainnets
root@<container-id>:/privadex/dex_aggregator/executor# cargo run --example privadex_e2e_execute_plan_mainnets
# Build WASM contract - note that you need to modify Cargo.toml lib target "crate-type" from "lib" to "cdylib"
# Also you need to manually install Parity's cargo contract: https://github.com/paritytech/cargo-contract
root@<container-id>:/privadex/dex_aggregator/executor# cargo contract build
```

The `cargo contract build` command above generates a `.contract` file in the `target` directory. You can upload this file to [Phat Contract UI](https://phat.phala.network/) to instantiate the Phat Contract on Poc5 Testnet. Note that you will need PHA tokens to deploy the contract and instantiate secret keys.
