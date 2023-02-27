# PrivaDEX
PrivaDEX is the cross-chain DEX aggregator native to Polkadot. As an example, you can swap from USDC (Wormhole) on Moonbeam to ARSW on Astar *in one click*.

## Technical notes
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

## Build Docker image and start Docker container
```bash
# Build image
docker build -t privadex_chain_metadata .
# Create and start container
docker run -it --name privadex_chain_metadata privadex_chain_metadata
```

If you have already created the container, you can enter it by starting and attaching the container:
```bash
docker start privadex_chain_metadata && docker attach privadex_chain_metadata
```

## Official Links
* [Live app](https://app.privadex.xyz)
* [90 second video demo](https://www.youtube.com/watch?v=QA5429uEZbw)
* [Website](https://www.privadex.xyz)
* [Discord](https://discord.gg/dpPDNreeQ3)
* [Twitter](https://twitter.com/doprivadex)
