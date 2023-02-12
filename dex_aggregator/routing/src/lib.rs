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

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub mod graph;
pub mod graph_builder;
pub(crate) mod graphql_client;
pub mod smart_order_router;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utilities;

use privadex_chain_metadata::common::UniversalTokenId;

#[derive(Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum PublicError {
    AddEdgeFailed,
    BridgeMissingSrcToken(UniversalTokenId),
    BridgeMissingDestToken(UniversalTokenId),
    CreateGraphFailed,
    InvalidBody,
    NoPathFound,
    RequestFailed,
    SrcTokenDestTokenAreSame,
    UnregisteredChainId,
    VertexNotInGraph(UniversalTokenId),
}
pub(crate) type Result<T> = core::result::Result<T, PublicError>;
