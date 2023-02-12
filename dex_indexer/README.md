# Subsquid indexing of Substrate Frontier EVM DEXes

Currently this supports
1. StellaSwap (Moonbeam)
2. BeamSwap (Moonbeam)
3. ArthSwap (Astar)

This is very heavily based on [Beamswap-squid](https://github.com/subsquid/beamswap-squid),
which is in turn based on [Subsquid-frontier-evm-template](https://github.com/subsquid/squid-frontier-evm-template).
Find general dev info at the latter link (and on Subsquid docs).

For more info consult [FAQ](./FAQ.md).

## Prerequisites

* node 16.x
* docker
# PrivaDEX
PrivaDEX is a cross-chain DEX aggregator on Polkadot. As an example, you can
swap from USDC (Wormhole) on Moonbeam to ARSW on Astar *in one click*.

## General notes
All crates must be kept no_std compatible so that we can run them in an ink! contract environment.
This means you need to mark `src/lib.rs` with 
```
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
```
Otherwise, the crate by default links in libstd and you run into errors like the below:
```
error: duplicate lang item in crate `ink_env` (which `pink_extension` depends on): `panic_impl`.
error: duplicate lang item in crate `ink_allocator` (which `ink_env` depends on): `oom`.
```

## Official Links
App is live at :

## Setup
```bash
# 1. Install dependencies
npm ci

# 2. Symlink dex_consts.ts to the desired DEX e.g. Beamswap below. I don't like this
#    solution at all, but it's a convenient way to bypass the "no dynamic exports"
#   issue
ln -s dex_config/beamswap.ts dex_consts.ts

# 3. Compile Typescript files
make build

# 4. Generate pools_[DEX].json file. This is used in the main processor to parse events.
make pairs
```

## Run locally
```bash
# 1. Start target Postgres database (can check if it is up with docker ps -a)
make up

# 2. Re-generate database migrations in db/migrations
rm db/migrations/*js
make migration

# 3. Apply database migrations from db/migrations & start the processor
make process

# 4. The above command will block the terminal
#    being busy with fetching the chain data, 
#    transforming and storing it in the target database.
#
#    To start the graphql server open the separate terminal
#    and run. You should be able to access the playground at GQL_PORT
#    (currently set to 4350) i.e. at http://localhost:4350/graphql
npx squid-graphql-server
```

## Adding a new DEX
1. Add to dex_config directory, modeling after the other DEX files there.
2. Update `.env` to point to the new DEX.
3. Run setup and run locally

