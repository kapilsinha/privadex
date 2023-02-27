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

use core::{fmt, str::FromStr};
use ink_prelude::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use serde::{de, Deserialize, Deserializer};

use privadex_chain_metadata::common::{
    Amount, AssetId, BlockNum, EthAddress, Nonce, SubstrateExtrinsicHash, SubstratePublicKey,
    UniversalAddress,
};
use privadex_common::utils::{
    general_utils::{hex_string_to_vec, slice_to_hex_string},
    http_request::http_post_wrapper,
};

use super::super::common::{Result, SubstrateError};
use super::xcm_transfer_lookup;

pub fn extrinsic_hash_lookup_call(
    query_url: &str,
    min_block: BlockNum,
    max_block: BlockNum,
    extrinsic_hash: &SubstrateExtrinsicHash,
) -> Result<Vec<Extrinsic>> {
    let query = get_extrinsic_hash_lookup_query(min_block, max_block, extrinsic_hash);
    ink_env::debug_println!("Query: {}", query);
    let raw_bytes = graphql_query(query_url, &query)?;

    let (decoded, _): (DataWrapper<ExtrinsicVec>, usize) =
        serde_json_core::from_slice(&raw_bytes).or(Err(SubstrateError::InvalidBody))?;
    Ok(decoded.data.extrinsics)
}

pub fn xcm_transfer_event_lookup_call(
    query_url: &str,
    min_block: BlockNum,
    max_block: BlockNum,
    xcm_lookup: &xcm_transfer_lookup::XCMTransferLookup,
) -> Result<Vec<Block>> {
    let query = get_xcm_transfer_event_lookup_query(min_block, max_block, xcm_lookup);
    // ink_env::debug_println!("Query: {}", query);
    let raw_bytes = graphql_query(query_url, &query)?;

    let (decoded, _): (DataWrapper<BlocksVec>, usize) =
        serde_json_core::from_slice(&raw_bytes).or(Err(SubstrateError::InvalidBody))?;
    Ok(decoded.data.blocks)
}

fn get_extrinsic_hash_lookup_query(
    min_block: BlockNum,
    max_block: BlockNum,
    extrinsic_hash: &SubstrateExtrinsicHash,
) -> String {
    format!(
        "\
            extrinsics(limit: 1, \
                where: {{ block: {{ AND: {{ height_gte: {}, height_lte: {} }} }}, \
                            hash_eq: \\\"{}\\\" }}) \
            {{ \
                block {{ \
                    height \
                }} \
                indexInBlock \
                success \
            }} \
            ",
        min_block,
        max_block,
        &slice_to_hex_string(&extrinsic_hash.0)
    )
    .to_string()
}

fn get_xcm_transfer_event_lookup_query(
    min_block: BlockNum,
    max_block: BlockNum,
    xcm_lookup: &xcm_transfer_lookup::XCMTransferLookup,
) -> String {
    let event_name_in = match xcm_lookup.token_pallet {
        xcm_transfer_lookup::TokenPallet::Asset => format!(
            "\\\"Assets.Issued\\\" \\\"{}\\\"",
            xcm_lookup.msg_pass_direction.event_success_name()
        ),
        xcm_transfer_lookup::TokenPallet::Balance => format!(
            "\\\"Balances.Withdraw\\\" \\\"Balances.Deposit\\\" \\\"{}\\\"",
            xcm_lookup.msg_pass_direction.event_success_name()
        ),
    };
    let extrinsic_call_name = match xcm_lookup.msg_pass_direction {
        xcm_transfer_lookup::MessagePassingDirection::Ump => "\\\"ParaInherent.enter\\\"",
        _ => "\\\"ParachainSystem.set_validation_data\\\"",
    };
    // We set a large event limit below for # of returned blocks and events. We do not expect them to be hit
    // but they are necessary to avoid "query execution canceled due to statement timeout"
    format!(
        "\
            blocks(limit: {}, where: {{ height_gte: {}, height_lte: {} }}) {{ \
                height \
                events(limit:50, where: {{ \
                    extrinsic: {{ call: {{ name_eq: {} }} }}, \
                    name_in: [ {} ] \
                }}) {{ \
                    name \
                    indexInBlock \
                    args \
                }} \
            }} \
            ",
        max_block - min_block + 1,
        min_block,
        max_block,
        extrinsic_call_name,
        event_name_in,
    )
    .to_string()
}

// The below works but is slow (takes ~5 seconds to execute on Moonbeam). Via some experimentation
// I found that the where clause in blocks is the bottleneck (I assume field indexing issues).
// Thus we adjust the query
#[allow(dead_code)]
fn get_xcm_transfer_event_lookup_query_deprecated(
    min_block: BlockNum,
    max_block: BlockNum,
    xcm_lookup: &xcm_transfer_lookup::XCMTransferLookup,
) -> String {
    let event_name_in = match xcm_lookup.token_pallet {
        xcm_transfer_lookup::TokenPallet::Asset => "\\\"Assets.Issued\\\"",
        xcm_transfer_lookup::TokenPallet::Balance => {
            "\\\"Balances.Withdraw\\\" \\\"Balances.Deposit\\\""
        }
    };
    // We set a large event limit below for # of returned blocks and events. We do not expect them to be hit
    // but they are necessary to avoid "query execution canceled due to statement timeout"
    format!("\
            blocks(limit: {}, where: {{ height_gte: {}, height_lte: {}, \
                events_some: {{ \
                    call: {{ name_eq: \\\"ParachainSystem.set_validation_data\\\" }}, \
                    name_eq: \\\"{}\\\", \
                }} \
            }}) {{ \
                height \
                events(limit:20, where: {{ \
                    extrinsic: {{ call: {{name_eq: \\\"ParachainSystem.set_validation_data\\\"}} }}, \
                    name_in: [ {} ] \
                }}) {{ \
                    name \
                    indexInBlock \
                    args \
                }} \
            }} \
            ",
            max_block - min_block + 1,
            min_block,
            max_block,
            xcm_lookup.msg_pass_direction.event_success_name(),
            event_name_in,
    ).to_string()
}

#[derive(Deserialize, Debug)]
struct DataWrapper<T> {
    pub data: T,
}

#[derive(Deserialize, Debug)]
#[serde(bound(deserialize = "ink_prelude::vec::Vec<Extrinsic>: Deserialize<'de>"))]
struct ExtrinsicVec {
    pub extrinsics: Vec<Extrinsic>,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct Extrinsic {
    pub indexInBlock: Nonce,
    pub success: bool,
    pub block: ExtrinsicBlock,
}

#[derive(Deserialize, Debug)]
pub struct ExtrinsicBlock {
    pub height: BlockNum,
}

#[derive(Deserialize, Debug)]
#[serde(bound(deserialize = "ink_prelude::vec::Vec<Block>: Deserialize<'de>"))]
struct BlocksVec {
    pub blocks: Vec<Block>,
}

#[derive(Deserialize, Debug)]
#[serde(bound(deserialize = "ink_prelude::vec::Vec<Event>: Deserialize<'de>"))]
pub struct Block {
    pub height: BlockNum,
    pub events: Vec<Event>,
}

#[derive(Debug)]
// #[allow(non_snake_case)]
pub struct Event {
    pub name: EventType,
    pub index_in_block: Nonce,
    pub args: Args,
}

// Note that the deserialization relies on ordering args after name.
// Subsquid seems to order fields in the response the same way they
// are ordered in the request, so this should be fine
impl<'de> Deserialize<'de> for Event {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // These args are completely ignored but I can't figure out a cleaner way to
        // deserialize; you need to parse to the end and you can't serialize into an
        // arbitrary bytearray (and I'd like to avoid using a HashMap)
        #[derive(Deserialize, Debug)]
        #[allow(non_snake_case)]
        #[allow(dead_code)]
        struct XcmpArgs<'a> {
            pub messageHash: &'a str,
            pub weight: RefTimeContainer<'a>,
        }
        #[derive(Deserialize, Debug)]
        #[allow(non_snake_case)]
        #[allow(dead_code)]
        struct RefTimeContainer<'a> {
            pub refTime: &'a str,
        }

        #[derive(Deserialize, Debug)]
        #[allow(non_snake_case)]
        #[allow(dead_code)]
        struct DmpQueueArgs<'a> {
            pub messageId: &'a str,
            pub outcome: OutcomeContainer<'a>,
        }
        #[derive(Deserialize, Debug)]
        #[allow(non_snake_case)]
        #[allow(dead_code)]
        struct OutcomeContainer<'a> {
            pub __kind: &'a str,
            pub value: &'a str,
        }

        #[derive(Debug)]
        #[allow(non_snake_case)]
        #[allow(dead_code)]
        struct UmpArgs<'a> {
            pub messageId: &'a str,
            pub outcome: OutcomeContainer<'a>,
        }
        impl<'de> Deserialize<'de> for UmpArgs<'de> {
            fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct UmpVisitor;

                impl<'de> de::Visitor<'de> for UmpVisitor {
                    /// Return type of this visitor. This visitor computes the max of a
                    /// sequence of values of type T, so the type of the maximum is T.
                    type Value = UmpArgs<'de>;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str(
                            "struct UmpArgs (from array [messageId, {__kind: _, value: _}])",
                        )
                    }

                    fn visit_seq<S>(
                        self,
                        mut seq: S,
                    ) -> core::result::Result<UmpArgs<'de>, S::Error>
                    where
                        S: de::SeqAccess<'de>,
                    {
                        let message_id: &str = seq.next_element()?.ok_or_else(|| {
                            de::Error::custom(
                                "Failed to get message_id from the first array element",
                            )
                        })?;
                        let outcome: OutcomeContainer = seq.next_element()?.ok_or_else(|| {
                            de::Error::custom("Failed to get Outcome from the second array element")
                        })?;
                        Ok(UmpArgs {
                            messageId: message_id,
                            outcome,
                        })
                    }
                }

                // Create the visitor and ask the deserializer to drive it. The
                // deserializer will call visitor.visit_seq() if a seq is present in
                // the input data.
                deserializer.deserialize_seq(UmpVisitor)
            }
        }

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "camelCase")]
        enum Field {
            Name,
            IndexInBlock,
            Args,
        }

        struct EventVisitor;

        impl<'de> de::Visitor<'de> for EventVisitor {
            type Value = Event;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Event")
            }

            fn visit_map<V>(self, mut map: V) -> core::result::Result<Event, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut name = None;
                let mut index_in_block = None;
                let mut args = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Name => {
                            if name.is_some() {
                                return Err(de::Error::duplicate_field("name"));
                            }
                            let val: &str = map.next_value()?;
                            name = Some(
                                EventType::from_str(val)
                                    .map_err(|_| de::Error::custom("Unexpected name value"))?,
                            );
                        }
                        Field::IndexInBlock => {
                            if index_in_block.is_some() {
                                return Err(de::Error::duplicate_field("index_in_block"));
                            }
                            let val: Nonce = map.next_value()?;
                            index_in_block = Some(val);
                        }
                        Field::Args => {
                            if args.is_some() {
                                return Err(de::Error::duplicate_field("args"));
                            }
                            args = match name {
                                Some(EventType::AssetsIssued) => {
                                    let val: AssetsIssuedArgs = map.next_value()?;
                                    Some(Args::AssetsIssued(val))
                                }
                                Some(EventType::BalancesDeposit)
                                | Some(EventType::BalancesWithdraw) => {
                                    let val: BalancesUpdateArgs = map.next_value()?;
                                    Some(Args::BalancesUpdateArgs(val))
                                }
                                Some(EventType::Xcmp) => {
                                    let _val: XcmpArgs = map.next_value()?;
                                    Some(Args::Ignored)
                                }
                                Some(EventType::Ump) => {
                                    let _val: UmpArgs = map.next_value()?;
                                    Some(Args::Ignored)
                                }
                                Some(EventType::Dmp) => {
                                    let _val: DmpQueueArgs = map.next_value()?;
                                    Some(Args::Ignored)
                                }
                                None => {
                                    return Err(de::Error::missing_field("name"));
                                }
                            };
                        }
                    }
                }
                let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
                let index_in_block =
                    index_in_block.ok_or_else(|| de::Error::missing_field("index_in_block"))?;
                let args = args.ok_or_else(|| de::Error::missing_field("args"))?;
                Ok(Event {
                    name,
                    index_in_block,
                    args,
                })
            }
        }

        const FIELDS: &'static [&'static str] = &["name", "index_in_block", "args"];
        deserializer.deserialize_struct("Event", FIELDS, EventVisitor)
    }
}

#[derive(Deserialize, Debug, PartialEq)]
pub enum EventType {
    AssetsIssued,
    BalancesDeposit,
    BalancesWithdraw,
    Xcmp,
    Ump,
    Dmp,
}

impl From<&xcm_transfer_lookup::MessagePassingDirection> for EventType {
    fn from(msg_pass_direction: &xcm_transfer_lookup::MessagePassingDirection) -> Self {
        match msg_pass_direction {
            &xcm_transfer_lookup::MessagePassingDirection::Xcmp => Self::Xcmp,
            &xcm_transfer_lookup::MessagePassingDirection::Ump => Self::Ump,
            &xcm_transfer_lookup::MessagePassingDirection::Dmp => Self::Dmp,
        }
    }
}

impl FromStr for EventType {
    type Err = SubstrateError;

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        match s {
            "Assets.Issued" => Ok(Self::AssetsIssued),
            "Balances.Deposit" => Ok(Self::BalancesDeposit),
            "Balances.Withdraw" => Ok(Self::BalancesWithdraw),
            "XcmpQueue.Success" => Ok(Self::Xcmp),
            "Ump.ExecutedUpward" => Ok(Self::Ump),
            "DmpQueue.ExecutedDownward" => Ok(Self::Dmp),
            _ => Err(SubstrateError::UnknownEvent),
        }
    }
}

#[derive(Deserialize, Debug)]
pub enum Args {
    AssetsIssued(AssetsIssuedArgs),
    BalancesUpdateArgs(BalancesUpdateArgs),
    Ignored,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct AssetsIssuedArgs {
    #[serde(deserialize_with = "quoted_str_to_asset_id")]
    pub assetId: AssetId,
    #[serde(deserialize_with = "hex_str_to_universal_address")]
    pub owner: UniversalAddress,
    #[serde(deserialize_with = "quoted_str_to_amount")]
    pub totalSupply: Amount,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct BalancesUpdateArgs {
    #[serde(deserialize_with = "quoted_str_to_amount")]
    pub amount: Amount,
    #[serde(deserialize_with = "hex_str_to_universal_address")]
    pub who: UniversalAddress,
}

fn quoted_str_to_asset_id<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> core::result::Result<AssetId, D::Error> {
    let string = <&str>::deserialize(deserializer)?;
    let num: AssetId = string
        .parse()
        .map_err(|_| de::Error::custom("String to AssetId failed"))?;
    Ok(num)
}

fn quoted_str_to_amount<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> core::result::Result<Amount, D::Error> {
    let string = <&str>::deserialize(deserializer)?;
    let num: Amount = string
        .parse()
        .map_err(|_| de::Error::custom("String to Amount failed"))?;
    Ok(num)
}

fn hex_str_to_universal_address<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> core::result::Result<UniversalAddress, D::Error> {
    let string = <&str>::deserialize(deserializer)?;
    let addr_vec =
        hex_string_to_vec(string).map_err(|_| de::Error::custom("Hex string to vec failed"))?;
    if addr_vec.len() == 20 {
        let addr_arr: [u8; 20] = addr_vec.try_into().expect("Hex address is 20 bytes");
        Ok(UniversalAddress::Ethereum(EthAddress { 0: addr_arr }))
    } else if addr_vec.len() == 32 {
        let addr_arr: [u8; 32] = addr_vec.try_into().expect("Hex address is 32 bytes");
        Ok(UniversalAddress::Substrate(SubstratePublicKey {
            0: addr_arr,
        }))
    } else {
        Err(de::Error::custom("Hex address is not 20 or 32 bytes"))
    }
}

fn graphql_query<'a, 'b>(query_url: &'a str, nested_data: &'b str) -> Result<Vec<u8>> {
    let data = format!(r#"{{"query": "{{ {} }}" }}"#, nested_data).into_bytes();
    http_post_wrapper(query_url, data).map_err(|_| SubstrateError::RequestFailed)
}
