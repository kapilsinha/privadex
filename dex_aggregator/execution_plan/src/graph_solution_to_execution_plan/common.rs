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

use hex_literal::hex;

use privadex_chain_metadata::common::{EthAddress, SubstratePublicKey};

pub(crate) const ESCROW_ETH_ADDRESS: EthAddress = EthAddress {
    0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
};

// This is the Substrate-mapped address of 0x05a81d8564a3eA298660e34e03E5Eff9a29d7a2A
// Converted using https://hoonsubin.github.io/evm-substrate-address-converter/
// (original article at https://medium.com/astar-network/using-astar-network-account-between-substrate-and-evm-656643df22a0)
pub(crate) const ESCROW_ASTAR_NATIVE_ADDRESS: SubstratePublicKey = SubstratePublicKey {
    0: hex!("5134c7f0e31c2a9e19dceddb7403b2836c69cce0b0719d2f58ec0d4da35129be"),
};

pub(crate) const ESCROW_SUBSTRATE_PUBLIC_KEY: SubstratePublicKey = SubstratePublicKey {
    0: hex!("7011b670bb662eedbd60a1c4c11b7c197ec22e7cfe87df00013ca2c494f3b01a"),
};

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum GraphToExecConversionError {
    GraphSolutionPathsLengthZero, // There are no SplitGraphPaths in GraphSolution
    GraphPathLengthZero,          // SplitGraphPath.path has zero edges
    NoChainInfo,                  // Could not find a ChainInfo for the requested chain
    StartedWrapEndedUnwrap, // Should not start with a wrap and end with unwrap (we do not expect cycles)
    UnexpectedStillProcessingSwap, // Should not be processing a swap (when we encounter some edge)
    UnexpectedSwapAfterUnwrap, // Should not encounter a CPMM after unwrap
}
