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

// Can create and move this to a price feed crate, but this is a good place until then
use ink_prelude::{vec, vec::Vec};
use privadex_chain_metadata::{
    common::{
        Amount, Dex, EthAddress, UniversalTokenId, NATIVE_TOKEN_DECIMALS, USD_AMOUNT_EXPONENT,
    },
    registry::token::universal_token_id_registry,
};
use privadex_common::fixed_point::DecimalFixedPoint;

use crate::graph::{edge::ConstantProductAMMSwapEdge, graph::Token};
use crate::{PublicError, Result};

use hashbrown::HashSet;

#[allow(dead_code)]
pub fn get_tokens_and_edges(
    dex: &'static Dex,
    min_token_pair_reserve_usd: u32,
    avg_gas_fee_in_native_token: Amount,
) -> Result<(Vec<Token>, Vec<ConstantProductAMMSwapEdge>)> {
    let mut token_id_set: HashSet<UniversalTokenId> = HashSet::new();
    get_additional_tokens_and_edges(
        dex,
        min_token_pair_reserve_usd,
        avg_gas_fee_in_native_token,
        &mut token_id_set,
    )
}

// min_token_pair_reserve_usd is in actual $ (no 'decimals' multiplicative factor)
// e.g. $500 -> 500
pub fn get_additional_tokens_and_edges<'a>(
    dex: &'static Dex,
    min_token_pair_reserve_usd: u32,
    avg_gas_fee_in_native_token: Amount,
    token_id_set: &'a mut HashSet<UniversalTokenId>, // Tokens already in this set won't be added
) -> Result<(Vec<Token>, Vec<ConstantProductAMMSwapEdge>)> {
    let combined_raw =
        graphql_low_level_interface::combined_call(dex.graphql_url, min_token_pair_reserve_usd)?;

    let usd_per_native_token_unit = combined_raw
        .bundleById
        .ethPrice
        .add_exp(-(NATIVE_TOKEN_DECIMALS as i8));

    let mut tokens: Vec<Token> = vec![];
    let mut cpmm_edges: Vec<ConstantProductAMMSwapEdge> = vec![];

    for token_pair in combined_raw.pairs.iter() {
        let token0_id = universal_token_id_registry::chain_and_eth_addr_to_token(
            dex.chain_id,
            token_pair.token0.id,
        );
        let token1_id = universal_token_id_registry::chain_and_eth_addr_to_token(
            dex.chain_id,
            token_pair.token1.id,
        );
        let reserve0 = token_pair
            .reserve0
            .add_exp(token_pair.token0.decimals as i8)
            .val();
        let reserve1 = token_pair
            .reserve1
            .add_exp(token_pair.token1.decimals as i8)
            .val();
        for (src_id, dest_id, src_token, dest_token) in [
            (
                &token0_id,
                &token1_id,
                &token_pair.token0,
                &token_pair.token1,
            ),
            (
                &token1_id,
                &token0_id,
                &token_pair.token1,
                &token_pair.token0,
            ),
        ]
        .into_iter()
        {
            let src_derived_eth = src_token
                .derivedETH
                .add_exp((NATIVE_TOKEN_DECIMALS as i8) - (src_token.decimals as i8));
            let dest_derived_eth = dest_token
                .derivedETH
                .add_exp((NATIVE_TOKEN_DECIMALS as i8) - (dest_token.decimals as i8));
            if token_id_set.insert(src_id.clone()) {
                tokens.push(Token {
                    id: src_id.clone(),
                    // (# USD / # this token unit) = (# native token units / # this token unit) *
                    //                               (# USD / # native token unit)
                    derived_usd: src_derived_eth.mul_small(&usd_per_native_token_unit),
                    // (# native token units / # this token unit)
                    derived_eth: src_derived_eth,
                });
            }

            let estimated_gas_fee_in_dest_token =
                DecimalFixedPoint::u128_div(avg_gas_fee_in_native_token, &dest_derived_eth);
            let estimated_gas_fee_usd = usd_per_native_token_unit
                .add_exp(USD_AMOUNT_EXPONENT as i8)
                .mul_u128(avg_gas_fee_in_native_token);

            cpmm_edges.push(ConstantProductAMMSwapEdge {
                src_token: src_id.clone(),
                dest_token: dest_id.clone(),
                token0: token0_id.id.clone(),
                token1: token1_id.id.clone(),
                reserve0,
                reserve1,
                // # dest_token_units = (# native token units) / (# native token units / # dest token unit)
                estimated_gas_fee_in_dest_token,
                estimated_gas_fee_usd,
                dex,
                pair_address: token_pair.id,
            })
        }
    }

    Ok((tokens, cpmm_edges))
}

mod graphql_low_level_interface {
    use ink_prelude::{format, vec::Vec};
    use privadex_common::fixed_point::DecimalFixedPoint;
    #[allow(unused_imports)]
    use privadex_common::utils::{
        general_utils::{hex_string_to_vec, slice_to_hex_string},
        http_request::http_post_wrapper,
    };
    use serde::{de, Deserialize, Deserializer};

    use super::{EthAddress, PublicError, Result};

    #[derive(Deserialize, Debug)]
    pub(super) struct DataWrapper<T> {
        pub data: T,
    }

    #[cfg(test)]
    #[derive(Deserialize, Debug)]
    #[allow(non_snake_case)]
    pub(super) struct EthPriceBundle {
        pub bundleById: EthPrice,
    }

    #[derive(Deserialize, Debug)]
    #[allow(non_snake_case)]
    pub(super) struct EthPrice {
        #[serde(deserialize_with = "str_to_eth_price_fixed_point")]
        pub ethPrice: DecimalFixedPoint,
    }

    #[cfg(test)]
    #[derive(Deserialize, Debug)]
    #[serde(bound(deserialize = "ink_prelude::vec::Vec<TokenPair>: Deserialize<'de>"))]
    pub(super) struct TokenPairVec {
        pub pairs: Vec<TokenPair>,
    }

    #[cfg(test)]
    #[derive(Deserialize, Debug)]
    #[allow(non_snake_case)]
    #[allow(dead_code)]
    pub(super) struct TokenPair {
        #[serde(deserialize_with = "hex_str_to_ethaddress")]
        pub id: EthAddress,
        #[serde(deserialize_with = "hex_str_to_ethaddress")]
        pub token0Id: EthAddress,
        #[serde(deserialize_with = "hex_str_to_ethaddress")]
        pub token1Id: EthAddress,
        #[serde(deserialize_with = "str_to_reserve_fixed_point")]
        pub reserve0: DecimalFixedPoint,
        #[serde(deserialize_with = "str_to_reserve_fixed_point")]
        pub reserve1: DecimalFixedPoint,
    }

    #[cfg(test)]
    #[derive(Deserialize, Debug)]
    #[serde(bound(deserialize = "ink_prelude::vec::Vec<Token>: Deserialize<'de>"))]
    pub(super) struct TokenVec {
        pub tokens: Vec<Token>,
    }

    #[derive(Deserialize, Debug)]
    #[allow(non_snake_case)]
    pub(super) struct Token {
        #[serde(deserialize_with = "hex_str_to_ethaddress")]
        pub id: EthAddress,
        #[serde(deserialize_with = "str_to_derived_eth_fixed_point")]
        pub derivedETH: DecimalFixedPoint,
        pub decimals: u32,
    }

    #[derive(Deserialize, Debug)]
    #[allow(non_snake_case)]
    pub(super) struct NestedTokenPair {
        #[serde(deserialize_with = "hex_str_to_ethaddress")]
        pub id: EthAddress,
        pub token0: Token,
        pub token1: Token,
        #[serde(deserialize_with = "str_to_reserve_fixed_point")]
        pub reserve0: DecimalFixedPoint,
        #[serde(deserialize_with = "str_to_reserve_fixed_point")]
        pub reserve1: DecimalFixedPoint,
    }

    #[derive(Deserialize, Debug)]
    #[serde(bound(deserialize = "ink_prelude::vec::Vec<NestedTokenPair>: Deserialize<'de>"))]
    #[allow(non_snake_case)]
    pub(super) struct CombinedResponse {
        pub bundleById: EthPrice,
        pub pairs: Vec<NestedTokenPair>,
    }

    // Empirically the value of RAW ethPrice ($ per token - generally 10^18 token units)
    // is 0.04 -> 5 for the chains' native tokens
    fn str_to_eth_price_fixed_point<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> core::result::Result<DecimalFixedPoint, D::Error> {
        let string = <&str>::deserialize(deserializer)?;
        let fixed_point = DecimalFixedPoint::from_str_and_exp(string, 8);
        Ok(fixed_point)
    }

    // Empirically the value of RAW derivedETH (native token per token, decimal adjusted)
    // is e-3 -> 500,000
    fn str_to_derived_eth_fixed_point<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> core::result::Result<DecimalFixedPoint, D::Error> {
        let string = <&str>::deserialize(deserializer)?;
        let fixed_point = DecimalFixedPoint::from_str_and_exp(string, 9);
        Ok(fixed_point)
    }

    // Empirically the raw value of reserve0 and reserve1 is 0.25 -> 50,000,000
    fn str_to_reserve_fixed_point<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> core::result::Result<DecimalFixedPoint, D::Error> {
        let string = <&str>::deserialize(deserializer)?;
        let fixed_point = DecimalFixedPoint::from_str_and_exp(string, 8);
        Ok(fixed_point)
    }

    fn hex_str_to_ethaddress<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> core::result::Result<EthAddress, D::Error> {
        let string = <&str>::deserialize(deserializer)?;
        let addr_vec =
            hex_string_to_vec(string).map_err(|_| de::Error::custom("Hex string to vec failed"))?;
        let addr_arr: [u8; 20] = addr_vec
            .try_into()
            .map_err(|_| de::Error::custom("Hex address is not 20 bytes"))?;
        Ok(EthAddress { 0: addr_arr })
    }

    // This function exists just so that we can collect ETH price and tokens and pairs in one GraphQL call.
    // This allows us to query all data for a DEX in one call (with some redundant tokens data that we will
    // remove - but it helps with parsing)
    // Note: We filter out derivedETH == 0 because it causes dangerous (overflow) issues downstream
    // in calculating USD value, fees, etc.
    pub(super) fn combined_call(query_url: &str, min_reserve_usd: u32) -> Result<CombinedResponse> {
        let query = format!(
            "\
            pairs(orderBy: reserveUSD_DESC, \
                where: {{ AND: {{token0: {{derivedETH_gt: \\\"0\\\"}}, \
                                 token1: {{derivedETH_gt: \\\"0\\\"}}, \
                                 reserveUSD_gt: \\\"{}\\\"}} \
                       }}) {{ \
                id \
                reserve0 \
                reserve1 \
                token0 {{ \
                    decimals \
                    derivedETH \
                    id \
                }} \
                token1 {{ \
                    decimals \
                    derivedETH \
                    id \
                }} \
            }} \
            bundleById(id: \\\"1\\\") {{ ethPrice }} \
            ",
            min_reserve_usd
        );
        let raw_bytes = graphql_query(query_url, &query)?;
        let (decoded, _): (DataWrapper<CombinedResponse>, usize) =
            serde_json_core::from_slice(&raw_bytes).or(Err(PublicError::InvalidBody))?;
        Ok(decoded.data)
    }

    #[cfg(test)]
    pub(super) fn eth_price_call(query_url: &str) -> Result<DecimalFixedPoint> {
        let query = get_eth_price_query();
        let raw_bytes = graphql_query(query_url, &query)?;
        let (decoded, _): (DataWrapper<EthPriceBundle>, usize) =
            serde_json_core::from_slice(&raw_bytes).or(Err(PublicError::InvalidBody))?;
        Ok(decoded.data.bundleById.ethPrice)
    }

    #[cfg(test)]
    fn get_eth_price_query() -> String {
        "bundleById(id: \\\"1\\\") { ethPrice }".to_string()
    }

    #[cfg(test)]
    pub(super) fn tokens_call(query_url: &str, token_addrs: &[EthAddress]) -> Result<Vec<Token>> {
        let query = get_tokens_query(token_addrs);
        let raw_bytes = graphql_query(query_url, &query)?;
        let (decoded, _): (DataWrapper<TokenVec>, usize) =
            serde_json_core::from_slice(&raw_bytes).or(Err(PublicError::InvalidBody))?;
        Ok(decoded.data.tokens)
    }

    #[cfg(test)]
    fn get_tokens_query(token_addrs: &[EthAddress]) -> String {
        // Below is a string that looks like
        // String[\"0xffffffff1fcacbd218edc0eba20fc2308c778080\" \"0xacc15dc74880c9944775448304b263d191c6077f\"]
        let addrs_str: String =
            token_addrs
                .iter()
                .fold("".to_string(), |cur: String, next: &EthAddress| {
                    cur + " \\\"" + slice_to_hex_string(&next.0).as_str() + "\\\""
                });
        format!(
            "\
            tokens( \
                where: {{id_in: [{} ]}} \
            ) {{ \
                decimals \
                id \
                derivedETH \
            }} \
            ",
            addrs_str.as_str()
        )
    }

    // Note: We filter out derivedETH == 0 because it causes dangerous (overflow) issues downstream
    // in calculating USD value, fees, etc.
    #[cfg(test)]
    pub(super) fn pairs_call(query_url: &str, min_reserve_usd: u32) -> Result<Vec<TokenPair>> {
        let query = format!(
            "\
            pairs(orderBy: reserveUSD_DESC, \
                where: {{ AND: {{token0: {{derivedETH_gt: \\\"0\\\"}}, \
                                token1: {{derivedETH_gt: \\\"0\\\"}}, \
                                reserveUSD_gt: \\\"{}\\\"}} \
                    }}) {{ \
                id \
                reserve1 \
                reserve0 \
                token0Id \
                token1Id \
                }} \
            ",
            min_reserve_usd
        );
        let raw_bytes = graphql_query(query_url, &query)?;
        let (decoded, _): (DataWrapper<TokenPairVec>, usize) =
            serde_json_core::from_slice(&raw_bytes).or(Err(PublicError::InvalidBody))?;
        Ok(decoded.data.pairs)
    }

    #[cfg(test)]
    pub(super) fn pairs_call_no_derived_eth_filter(
        query_url: &str,
        min_reserve_usd: u32,
    ) -> Result<Vec<TokenPair>> {
        let query = format!(
            "\
            pairs(orderBy: reserveUSD_DESC, where: {{reserveUSD_gt: \\\"{}\\\"}}) {{ \
                id \
                reserve1 \
                reserve0 \
                token0Id \
                token1Id \
                }} \
            ",
            min_reserve_usd
        );
        let raw_bytes = graphql_query(query_url, &query)?;
        let (decoded, _): (DataWrapper<TokenPairVec>, usize) =
            serde_json_core::from_slice(&raw_bytes).or(Err(PublicError::InvalidBody))?;
        Ok(decoded.data.pairs)
    }

    fn graphql_query<'a, 'b>(query_url: &'a str, nested_data: &'b str) -> Result<Vec<u8>> {
        let data = format!(r#"{{"query": "{{ {} }}" }}"#, nested_data).into_bytes();
        http_post_wrapper(query_url, data).map_err(|_| PublicError::RequestFailed)
    }
}

// Note that the below tests require a network connection to work! We deliberately do not
// mock the HTTP responses so we can also test the GraphQL service
#[cfg(test)]
mod graphql_client_tests {
    use hex_literal::hex;
    use ink_env::debug_println;
    use privadex_chain_metadata::registry::dex::dex_registry::{ARTHSWAP, BEAMSWAP, STELLASWAP};

    use super::graphql_low_level_interface::*;
    use super::*;

    #[test]
    fn test_graphql_tokens_cpmm_edges() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let (tokens, cpmm_edges) =
            get_tokens_and_edges(&STELLASWAP, 4_000_000, Amount::pow(10, 16)).unwrap();
        assert!(tokens.len() > 0);
        assert!(cpmm_edges.len() >= tokens.len() / 2);
        debug_println!("Tokens: {:?}", tokens);
        debug_println!("CPMM edges: {:?}", cpmm_edges);
    }

    #[test]
    fn test_graphql_client_combined() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let combined_data = combined_call(ARTHSWAP.graphql_url, 2_000_000).unwrap();
        // debug_println!("Combined data: {:?}", combined_data);
        assert!(combined_data.pairs.len() > 0);
    }

    #[test]
    fn test_graphql_client_eth_price() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let stellaswap_glmr_price = eth_price_call(STELLASWAP.graphql_url)
            .unwrap()
            .add_exp(18)
            .val();
        let beamswap_glmr_price = eth_price_call(BEAMSWAP.graphql_url)
            .unwrap()
            .add_exp(18)
            .val();
        // debug_println!("GLMR price (from Stellaswap) = {}", stellaswap_glmr_price);
        // debug_println!("GLMR price (from Beamswap) = {}", beamswap_glmr_price);
        let (min, max) = {
            if stellaswap_glmr_price > beamswap_glmr_price {
                (beamswap_glmr_price, stellaswap_glmr_price)
            } else {
                (stellaswap_glmr_price, beamswap_glmr_price)
            }
        };
        // Check that StellaSwap and BeamSwap-based GLMR prices are within 1% of one another
        assert!((max - min) < min / 50);
    }

    #[test]
    fn test_graphql_client_pairs_data() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let pairs_data = pairs_call(ARTHSWAP.graphql_url, 2000000).unwrap();
        // debug_println!("Pairs data: {:?}", pairs_data);
        let pairs_expanded_data = pairs_call(ARTHSWAP.graphql_url, 1000000).unwrap();
        assert!(pairs_data.len() < pairs_expanded_data.len());
    }

    // We want to ensure that the derived_eth filter (which exists to protect
    // from division and overflow errors downstream), does not filter out too
    // many pairs
    #[test]
    fn test_graphql_client_pairs_derived_eth_filter() {
        pink_extension_runtime::mock_ext::mock_all_ext();

        let min_reserve_usd = 12_000;

        {
            let query_url = ARTHSWAP.graphql_url;
            let pairs_data = pairs_call(query_url, min_reserve_usd).unwrap();
            let pairs_data_unfiltered =
                pairs_call_no_derived_eth_filter(query_url, min_reserve_usd).unwrap();
            assert!(pairs_data.len() == pairs_data_unfiltered.len());
        }
        {
            let query_url = BEAMSWAP.graphql_url;
            let pairs_data = pairs_call(query_url, min_reserve_usd).unwrap();
            let pairs_data_unfiltered =
                pairs_call_no_derived_eth_filter(query_url, min_reserve_usd).unwrap();
            assert!(pairs_data.len() == pairs_data_unfiltered.len());
        }
        {
            let query_url = STELLASWAP.graphql_url;
            let pairs_data = pairs_call(query_url, min_reserve_usd).unwrap();
            let pairs_data_unfiltered =
                pairs_call_no_derived_eth_filter(query_url, min_reserve_usd).unwrap();
            assert!(pairs_data.len() == pairs_data_unfiltered.len());
        }
    }

    #[test]
    fn test_graphql_client_tokens_data() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let tokens_arr = vec![
            EthAddress {
                0: hex!("ffffffff1fcacbd218edc0eba20fc2308c778080"),
            },
            EthAddress {
                0: hex!("acc15dc74880c9944775448304b263d191c6077f"),
            },
            EthAddress {
                0: hex!("0E358838ce72d5e61E0018a2ffaC4bEC5F4c88d2"),
            },
        ];
        let _tokens_data = tokens_call(STELLASWAP.graphql_url, &tokens_arr);
        // debug_println!("Token data: {:?}", _tokens_data);
    }

    #[test]
    fn test_decode_eth_price_bundle() {
        let eth_price_bundle = "{\"data\":{\"bundleById\":{\"ethPrice\":\"0.03961864636235579427619351783955519601792472066032979151843229658302586546416183\"}}}".as_bytes();
        let (decoded, _): (DataWrapper<EthPriceBundle>, usize) =
            serde_json_core::from_slice(eth_price_bundle).unwrap();
        // debug_println!("Decoded eth price bundle: {:?}", decoded);
        assert_eq!(
            decoded.data.bundleById.ethPrice,
            DecimalFixedPoint::from_str_and_exp("0.039618646362355794", 8)
        );
    }

    #[test]
    fn test_decode_pairs() {
        let pairs_data = "{\"data\":{\"pairs\":[\
                                    {\"id\":\"0xccefddff4808f3e1e0340e19e43f1e9fd088b3f2\",\
                                    \"reserve1\":\"62258525.543089871923048517\",\
                                    \"reserve0\":\"6948969.880701618116660496\",\
                                    \"token0Id\":\"0x75364d4f779d0bd0facd9a218c67f87dd9aff3b4\",\
                                    \"token1Id\":\"0xaeaaf0e2c81af264101b9129c00f4440ccf0f720\"},\
                                    {\"id\":\"0xb4461721d3ad256cd59d207fefbfe05791ef8568\",\
                                    \"reserve1\":\"28315125.294836490821050008\",\
                                    \"reserve0\":\"28155035.667599855639049157\",\
                                    \"token0Id\":\"0xaeaaf0e2c81af264101b9129c00f4440ccf0f720\",\
                                    \"token1Id\":\"0xe511ed88575c57767bafb72bfd10775413e3f2b0\"}\
                                ]}}"
        .as_bytes();
        let (decoded, _): (DataWrapper<TokenPairVec>, usize) =
            serde_json_core::from_slice(pairs_data).unwrap();
        // debug_println!("Decoded pairs data: {:?}", decoded);

        assert_eq!(decoded.data.pairs.len(), 2);
        assert_eq!(
            decoded.data.pairs[0].id,
            EthAddress {
                0: hex!("ccefddff4808f3e1e0340e19e43f1e9fd088b3f2")
            }
        );
        assert_eq!(
            decoded.data.pairs[1].reserve0,
            DecimalFixedPoint::from_str_and_exp("28155035.667599855639049157", 8)
        );
    }

    #[test]
    fn test_decode_tokens() {
        let token_data = "{\"data\":{\"tokens\":[\
                                    {\"decimals\":10,\"id\":\"0xffffffff1fcacbd218edc0eba20fc2308c778080\",\"derivedETH\":\"13.25557946238032928633136399182815382591191059661949183017614686822975192913156202440376738835679403885314894159356521321815196612257485232024359841901618753415\"},\
                                    {\"decimals\":18,\"id\":\"0xacc15dc74880c9944775448304b263d191c6077f\",\"derivedETH\":\"1\"}\
                                ]}}".as_bytes();
        let (decoded, _): (DataWrapper<TokenVec>, usize) =
            serde_json_core::from_slice(token_data).unwrap();
        // debug_println!("Decoded token data: {:?}", decoded);

        assert_eq!(decoded.data.tokens.len(), 2);
        assert_eq!(
            decoded.data.tokens[0].id,
            EthAddress {
                0: hex!("ffffffff1fcacbd218edc0eba20fc2308c778080")
            }
        );
        assert_eq!(
            decoded.data.tokens[0].derivedETH,
            DecimalFixedPoint::from_str_and_exp("13.255579462", 9)
        );
        assert_eq!(decoded.data.tokens[0].decimals, 10);
    }

    #[test]
    fn test_combined_call() {
        let combined_data = "{\"data\":{\
                                    \"pairs\":[\
                                        {\"id\":\"0xccefddff4808f3e1e0340e19e43f1e9fd088b3f2\",\"reserve0\":\"6952946.44665235172725434\",\"reserve1\":\"62223196.301748411321042674\",\
                                            \"token0\":{\"decimals\":18,\"derivedETH\":\"8.909583873683757648908068\",\"id\":\"0x75364d4f779d0bd0facd9a218c67f87dd9aff3b4\"},\
                                            \"token1\":{\"decimals\":10,\"derivedETH\":\"1\",\"id\":\"0xaeaaf0e2c81af264101b9129c00f4440ccf0f720\"}\
                                        }\
                                    ], \
                                    \"bundleById\":{\"ethPrice\":\"0.03961864636235579427619351783955519601792472066032979151843229658302586546416183\"}\
                                }}".as_bytes();
        let (decoded, _): (DataWrapper<CombinedResponse>, usize) =
            serde_json_core::from_slice(combined_data).unwrap();
        debug_println!("Decoded token data: {:?}", decoded);

        assert_eq!(decoded.data.pairs.len(), 1);
        assert_eq!(
            decoded.data.pairs[0].id,
            EthAddress {
                0: hex!("ccefddff4808f3e1e0340e19e43f1e9fd088b3f2")
            }
        );
        assert_eq!(decoded.data.pairs[0].token0.decimals, 18);
        assert_eq!(decoded.data.pairs[0].token1.decimals, 10);
        assert_eq!(
            decoded.data.pairs[0].token0.derivedETH,
            DecimalFixedPoint::from_str_and_exp("8.909583873", 9)
        );
        assert_eq!(
            decoded.data.bundleById.ethPrice,
            DecimalFixedPoint::from_str_and_exp("0.039618646362355794", 8)
        );
    }
}
