#![cfg_attr(not(feature = "std"), no_std)]

use privadex_chain_metadata::common::UniversalTokenId;
extern crate alloc;

pub mod graph;
pub mod graph_builder;
pub(crate) mod graphql_client;
pub mod smart_order_router;

#[derive(Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum PublicError {
    AddEdgeFailed,
	CreateGraphFailed,
    BridgeMissingSrcToken(UniversalTokenId),
    BridgeMissingDestToken(UniversalTokenId),
    UnregisteredChainId,
    InvalidBody,
    RequestFailed,
    VertexNotInGraph(UniversalTokenId),
}
pub(crate) type Result<T> = core::result::Result<T, PublicError>;
