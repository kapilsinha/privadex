# PrivaDEX Chain Metadata
This crate has 3 functions:
1. Define common types (e.g. ChainId, TokenId, ChainInfo) to be used downstream.
2. Host generic utils (e.g. signature utils, SS58 address utils)
3. Define static registries for XCM bridges, chain info, DEXes, and tokens.

## Prerequisites
1. Set API keys for RPC nodes (search and replace for "[INSERT API KEY HERE]"). This is required for the Polkadot relay RPC node, but there are default public RPC nodes for the parachains (which suffice for unit tests).

## Unit tests
```bash
cargo test
```

## Docker testing guide
To run tests from a Docker container, start the Docker container (instructions in the root README file):
```bash
root@<container-id>:/privadex/dex_aggregator/chain_metadata# cargo test
```
