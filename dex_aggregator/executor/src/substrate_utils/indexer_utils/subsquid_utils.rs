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

use ink_prelude::string::String;

use privadex_chain_metadata::{
    common::{
        Amount, BlockNum, ChainTokenId, Nonce, SubstrateExtrinsicHash, UniversalAddress,
        UniversalTokenId,
    },
    get_chain_info_from_chain_id, get_sovereign_account,
};

use super::super::common::{Result, SubstrateError};
use super::{graphql_helper, xcm_transfer_lookup};

// Querying gas fees from extrinsics is tricky (requires parsing events),
// so we likely won't bother updating our initial gas estimates

/// Interface for querying Substrate extrinsics and events from a Subsquid indexer
pub struct SubstrateSubsquidUtils {
    pub subsquid_graphql_archive_url: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct SubstrateFinalizedExtrinsicResult {
    pub is_extrinsic_success: bool,
    pub block_num: BlockNum,
    pub extrinsic_index: Nonce,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct SubstrateXCMTransferEventResult {
    pub block_num: BlockNum,
    pub event_index: Nonce,
    pub amount_out: Amount,
}

impl SubstrateSubsquidUtils {
    #[cfg(not(feature = "mock-txn-send"))]
    pub fn lookup_extrinsic_by_hash(
        &self,
        min_block: BlockNum,
        max_block: BlockNum,
        extrinsic_hash: &SubstrateExtrinsicHash,
    ) -> Result<SubstrateFinalizedExtrinsicResult> {
        let extrinsics_vec = graphql_helper::extrinsic_hash_lookup_call(
            &self.subsquid_graphql_archive_url,
            min_block,
            max_block,
            extrinsic_hash,
        )?;
        if extrinsics_vec.is_empty() {
            Err(SubstrateError::NotFound)
        } else {
            let extrinsic = &extrinsics_vec[0];
            Ok(SubstrateFinalizedExtrinsicResult {
                is_extrinsic_success: extrinsic.success,
                block_num: extrinsic.block.height,
                extrinsic_index: extrinsic.indexInBlock,
            })
        }
    }
    #[cfg(feature = "mock-txn-send")]
    pub fn lookup_extrinsic_by_hash(
        &self,
        min_block: BlockNum,
        max_block: BlockNum,
        extrinsic_hash: &SubstrateExtrinsicHash,
    ) -> Result<SubstrateFinalizedExtrinsicResult> {
        ink_env::debug_println!("[Mock Substrate lookup_extrinsic_by_hash]");
        // unsafe {
        //     static mut x: u32 = 0;
        //     if x < 2 {
        //         x += 1;
        //         return Err(SubstrateError::NotFound);
        //     }
        // }
        Ok(SubstrateFinalizedExtrinsicResult {
            is_extrinsic_success: true,
            block_num: max_block,
            extrinsic_index: 0,
        })
    }

    #[cfg(not(feature = "mock-txn-send"))]
    pub fn lookup_xcm_event_transfer(
        &self,
        min_block: BlockNum,
        max_block: BlockNum,
        src_token: UniversalTokenId,
        dest_token: UniversalTokenId,
        amount: Amount,
        dest_addr: UniversalAddress,
    ) -> Result<SubstrateXCMTransferEventResult> {
        let xcm_lookup = xcm_transfer_lookup::XCMTransferLookup::from_tokens_amount_addr(
            src_token, dest_token, amount, dest_addr,
        )?;
        let all_blocks = graphql_helper::xcm_transfer_event_lookup_call(
            &self.subsquid_graphql_archive_url,
            min_block,
            max_block,
            &xcm_lookup,
        )?;
        match xcm_lookup.token_pallet {
            xcm_transfer_lookup::TokenPallet::Asset => {
                Self::process_xcm_event_transfer_asset(&xcm_lookup, &all_blocks)
            }
            xcm_transfer_lookup::TokenPallet::Balance => {
                Self::process_xcm_event_transfer_balance(&xcm_lookup, &all_blocks)
            }
        }
    }
    #[cfg(feature = "mock-txn-send")]
    pub fn lookup_xcm_event_transfer(
        &self,
        min_block: BlockNum,
        max_block: BlockNum,
        src_token: UniversalTokenId,
        dest_token: UniversalTokenId,
        amount: Amount,
        dest_addr: UniversalAddress,
    ) -> Result<SubstrateXCMTransferEventResult> {
        ink_env::debug_println!("[Mock Substrate lookup_xcm_event_transfer]");
        // Cheap way to allow multiple not found periods
        // unsafe {
        //     static mut x: u32 = 0;
        //     if x < 2 {
        //         x += 1;
        //         return Err(SubstrateError::NotFound);
        //     }
        // }
        Ok(SubstrateXCMTransferEventResult {
            block_num: max_block,
            event_index: 0,
            amount_out: amount,
        })
    }

    fn process_xcm_event_transfer_asset(
        xcm_lookup: &xcm_transfer_lookup::XCMTransferLookup,
        all_blocks: &[graphql_helper::Block],
    ) -> Result<SubstrateXCMTransferEventResult> {
        ink_env::debug_println!("Blocks: {:?}", all_blocks);
        let msg_pass_event = graphql_helper::EventType::from(&xcm_lookup.msg_pass_direction);

        for block in all_blocks.iter() {
            let e = &block.events;
            let num_events = e.len();
            let any_msg_pass_events = e.iter().any(|event| event.name == msg_pass_event);
            if num_events < 3 || !any_msg_pass_events {
                continue;
            }
            for i in 0..num_events - 1 {
                if (&e[i].name, &e[i + 1].name)
                    == (
                        &graphql_helper::EventType::AssetsIssued,
                        &graphql_helper::EventType::AssetsIssued,
                    )
                {
                    if let (
                        graphql_helper::Args::AssetsIssued(args1),
                        graphql_helper::Args::AssetsIssued(args2),
                    ) = (&e[i].args, &e[i + 1].args)
                    {
                        // We are guaranteed (by struct construction) to enter this block
                        let is_correct_asset = {
                            if let ChainTokenId::XC20(token) = &xcm_lookup.dest_token.id {
                                (token.get_asset_id() == args1.assetId)
                                    && (token.get_asset_id() == args2.assetId)
                            } else {
                                false
                            }
                        };
                        let is_correct_dest = args1.owner == xcm_lookup.dest_addr;
                        let is_correct_amount =
                            args1.totalSupply + args2.totalSupply == xcm_lookup.amount;
                        if is_correct_asset && is_correct_dest && is_correct_amount {
                            return Ok(SubstrateXCMTransferEventResult {
                                block_num: block.height,
                                event_index: e[i].index_in_block,
                                amount_out: args1.totalSupply,
                            });
                        }
                    }
                }
            }
        }
        Err(SubstrateError::NotFound)
    }

    fn process_xcm_event_transfer_balance(
        xcm_lookup: &xcm_transfer_lookup::XCMTransferLookup,
        all_blocks: &[graphql_helper::Block],
    ) -> Result<SubstrateXCMTransferEventResult> {
        ink_env::debug_println!("Blocks: {:?}", all_blocks);
        let dest_chain_info = get_chain_info_from_chain_id(&xcm_lookup.dest_token.chain)
            .ok_or(SubstrateError::InvalidXcmLookup)?;
        let sovereign_account = get_sovereign_account(xcm_lookup.src_token.chain, dest_chain_info)
            .map_err(|_| SubstrateError::InvalidXcmLookup)?;
        let msg_pass_event = graphql_helper::EventType::from(&xcm_lookup.msg_pass_direction);

        for block in all_blocks.iter() {
            let e = &block.events;
            let num_events = e.len();
            let any_msg_pass_events = e.iter().any(|event| event.name == msg_pass_event);
            if num_events < 3 || !any_msg_pass_events {
                continue;
            }
            for i in 0..num_events - 1 {
                if (&e[i].name, &e[i + 1].name)
                    == (
                        &graphql_helper::EventType::BalancesWithdraw,
                        &graphql_helper::EventType::BalancesDeposit,
                    )
                {
                    if let (
                        graphql_helper::Args::BalancesUpdateArgs(args1),
                        graphql_helper::Args::BalancesUpdateArgs(args2),
                    ) = (&e[i].args, &e[i + 1].args)
                    {
                        // We are guaranteed (by struct construction) to enter this block
                        let is_correct_asset = &xcm_lookup.dest_token.id == &ChainTokenId::Native;
                        let is_correct_src = args1.who == sovereign_account;
                        let is_correct_dest = args2.who == xcm_lookup.dest_addr;
                        let is_correct_amount = args1.amount == xcm_lookup.amount;
                        if is_correct_asset
                            && is_correct_src
                            && is_correct_dest
                            && is_correct_amount
                        {
                            return Ok(SubstrateXCMTransferEventResult {
                                block_num: block.height,
                                event_index: e[i].index_in_block,
                                amount_out: args2.amount,
                            });
                        }
                    }
                }
            }
        }
        Err(SubstrateError::NotFound)
    }
}

#[cfg(test)]
mod subsquid_utils_tests {
    use hex_literal::hex;

    use privadex_chain_metadata::{
        chain_info::ChainInfo,
        common::{EthAddress, SubstratePublicKey},
        registry::{
            chain::chain_info_registry::{
                ASTAR_INFO, MOONBASEALPHA_INFO, MOONBEAM_INFO, POLKADOT_INFO,
            },
            token::universal_token_id_registry,
        },
    };

    use super::*;

    fn get_subutils(chain_info: &ChainInfo) -> SubstrateSubsquidUtils {
        SubstrateSubsquidUtils {
            subsquid_graphql_archive_url: chain_info.subsquid_graphql_archive_url.to_string(),
        }
    }

    #[test]
    fn test_moonbase_extrinsic_lookup() {
        // https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fwss.api.moonbase.moonbeam.network#/explorer/query/0x62c8d9e3a9dad14da06ae4f7d904f0c4bd26426ac0af6086689a7b3ae2088621
        pink_extension_runtime::mock_ext::mock_all_ext();
        let extrinsic_res = get_subutils(&MOONBASEALPHA_INFO)
            .lookup_extrinsic_by_hash(
                3_149_020,
                3_149_070,
                &SubstrateExtrinsicHash {
                    0: hex!("da96b87d389e7b76258442cb174b365bca944b6120f07886a6184b692789b29a"),
                },
            )
            .expect("Expect to find extrinsic");
        assert_eq!(extrinsic_res.is_extrinsic_success, true);
        assert_eq!(extrinsic_res.block_num, 3_149_025);
        assert_eq!(extrinsic_res.extrinsic_index, 5);
    }

    #[test]
    fn test_moonbeam_extrinsic_lookup() {
        // https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fmoonbeam.api.onfinality.io%2Fpublic-ws#/explorer/query/2518311
        pink_extension_runtime::mock_ext::mock_all_ext();
        let extrinsic_res = get_subutils(&MOONBEAM_INFO)
            .lookup_extrinsic_by_hash(
                2_518_300,
                2_518_350,
                &SubstrateExtrinsicHash {
                    0: hex!("4bb58b09839aa49e28888f2f62eed95b590d4308c53f1b5bf325cf87b0b1f2b7"),
                },
            )
            .expect("Expect to find extrinsic");
        assert_eq!(extrinsic_res.is_extrinsic_success, true);
        assert_eq!(extrinsic_res.block_num, 2_518_311);
        assert_eq!(extrinsic_res.extrinsic_index, 4);
    }

    #[test]
    fn test_extrinsic_lookup_not_found() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let extrinsic_res = get_subutils(&MOONBEAM_INFO).lookup_extrinsic_by_hash(
            2_518_312,
            2_518_350,
            &SubstrateExtrinsicHash {
                0: hex!("4bb58b09839aa49e28888f2f62eed95b590d4308c53f1b5bf325cf87b0b1f2b7"),
            },
        );
        assert_eq!(extrinsic_res, Err(SubstrateError::NotFound));
    }

    #[test]
    fn test_moonbeam_failed_extrinsic_lookup() {
        // https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fmoonbeam.api.onfinality.io%2Fpublic-ws#/explorer/query/0x076caba6eda1fb370957d406253816db1ec010954b55bd27df3b2ab5ba2f22eb
        pink_extension_runtime::mock_ext::mock_all_ext();
        let extrinsic_res = get_subutils(&MOONBEAM_INFO)
            .lookup_extrinsic_by_hash(
                1_649_700,
                1_649_800,
                &SubstrateExtrinsicHash {
                    0: hex!("d8f9788ffb29b94a548099665c72b02463d3ec2087fc457246fb21764b9979ef"),
                },
            )
            .expect("Expect to find extrinsic");
        assert_eq!(extrinsic_res.is_extrinsic_success, false);
        assert_eq!(extrinsic_res.block_num, 1_649_728);
        assert_eq!(extrinsic_res.extrinsic_index, 4);
    }

    #[test]
    fn test_xcmp_asset_transfer_event_lookup() {
        // Astar: https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fpublic-rpc.pinknode.io%2Fastar#/explorer/query/2493303
        // Moonbeam: https://polkadot.js.org/apps/?rpc=wss%3A%2F%2F1rpc.io%2Fglmr#/explorer/query/0x38bbbaf517d9429764785d202d344b30636392a68f8017c8b63674012b1e81f8
        pink_extension_runtime::mock_ext::mock_all_ext();
        let event_result = get_subutils(&MOONBEAM_INFO)
            .lookup_xcm_event_transfer(
                2_497_800,
                2_497_900,
                universal_token_id_registry::ASTR_NATIVE,
                universal_token_id_registry::ASTR_MOONBEAM,
                100_000_000_000_000_000,
                UniversalAddress::Ethereum(EthAddress {
                    0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
                }),
            )
            .expect("Expected results");
        assert_eq!(event_result.block_num, 2_497_827);
        assert_eq!(event_result.amount_out, 20_140_552_627_375_819);
        assert_eq!(event_result.event_index, 661);
    }

    #[test]
    fn test_xcmp_balance_transfer_event_lookup() {
        // Moonbeam: https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fwss.api.moonbeam.network#/explorer/query/2531796
        // Astar: https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Frpc.astar.network#/explorer/query/0x3eb44462727ab68abc33b06aee47ce3c61fc6734ed796b331034349729e08e31
        pink_extension_runtime::mock_ext::mock_all_ext();
        let event_result = get_subutils(&ASTAR_INFO)
            .lookup_xcm_event_transfer(
                2_527_150,
                2_527_200,
                universal_token_id_registry::ASTR_MOONBEAM,
                universal_token_id_registry::ASTR_NATIVE,
                200_000_000_000_000_000,
                UniversalAddress::Substrate(SubstratePublicKey {
                    0: hex!("5134c7f0e31c2a9e19dceddb7403b2836c69cce0b0719d2f58ec0d4da35129be"),
                }),
            )
            .expect("Expected results");
        assert_eq!(event_result.block_num, 2_527_187);
        assert_eq!(event_result.amount_out, 195_364_898_375_396_884);
        assert_eq!(event_result.event_index, 5);
    }

    #[test]
    fn test_ump_balance_transfer_event_lookup() {
        // Moonbeam: https://moonbeam.moonscan.io/tx/0xbd4474664fcfaa7bdd31c9d2aae7fe94ec6d83598f66f596a84dbe5ff7229ee7
        // Polkadot: https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Frpc.dotters.network%2Fpolkadot#/explorer/query/0xd45e457a848d08291b2e9db41bd7df207b0f94a6d61a63a83d9d0f46da8662e6
        pink_extension_runtime::mock_ext::mock_all_ext();
        let event_result = get_subutils(&POLKADOT_INFO)
            .lookup_xcm_event_transfer(
                13_372_800,
                13_372_900,
                universal_token_id_registry::DOT_MOONBEAM,
                universal_token_id_registry::DOT_NATIVE,
                40_000_000_000,
                UniversalAddress::Substrate(SubstratePublicKey {
                    0: hex!("60b94741c7094ac2820cceebeb24720af9e1049d7d4cb215f5080fbf5bdcbd4a"),
                }),
            )
            .expect("Expected results");
        assert_eq!(event_result.block_num, 13_372_856);
        assert_eq!(event_result.amount_out, 39_530_582_548);
        assert_eq!(event_result.event_index, 30);
    }

    #[test]
    fn test_dmp_asset_transfer_event_lookup() {
        // Polkadot: https://polkadot.js.org/apps/?rpc=wss%3A%2F%2F1rpc.io%2Fdot#/explorer/query/0xe278ebca27591a4303f1cc331c6bf0ad63accb0d5965b7259ca98f4dd954e32d
        // Astar: https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fpublic-rpc.pinknode.io%2Fastar#/explorer/query/0x1e517819e16edb3ddd96e8699ed1ee6e98228832c1aa8e0d626cfdaba1e9d071
        pink_extension_runtime::mock_ext::mock_all_ext();
        let event_result = get_subutils(&ASTAR_INFO)
            .lookup_xcm_event_transfer(
                2_514_150,
                2_514_200,
                universal_token_id_registry::DOT_NATIVE,
                universal_token_id_registry::DOT_ASTAR,
                1_000_000_000,
                UniversalAddress::Substrate(SubstratePublicKey {
                    0: hex!(
                        "5134c7f0e31c2a9e19dceddb7403b2836c69cce0b0719d2f58ec0d4da35129be
                "
                    ),
                }),
            )
            .expect("Expected results");
        assert_eq!(event_result.block_num, 2_514_195);
        assert_eq!(event_result.amount_out, 999_000_000);
        assert_eq!(event_result.event_index, 6);
    }

    #[test]
    fn test_xcm_transfer_event_lookup_not_found() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let event_result = get_subutils(&MOONBEAM_INFO).lookup_xcm_event_transfer(
            2_497_800,
            2_497_900,
            universal_token_id_registry::ASTR_NATIVE,
            universal_token_id_registry::ASTR_MOONBEAM,
            100_000_000_000_000_001,
            UniversalAddress::Ethereum(EthAddress {
                0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
            }),
        );
        assert_eq!(event_result, Err(SubstrateError::NotFound));
    }

    #[test]
    fn test_deserialization() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let assets_issued_event = "{\"name\":\"Assets.Issued\",\"indexInBlock\":661,\"args\":{\"assetId\":\"224077081838586484055667086558292981199\",\"owner\":\"0x05a81d8564a3ea298660e34e03e5eff9a29d7a2a\",\"totalSupply\":\"20140552627375819\"}}";
        let balances_withdraw_event = "{\"name\":\"Balances.Withdraw\",\"indexInBlock\":31,\"args\":{\"amount\": \"39530582548\",\"who\":\"0x60b94741c7094ac2820cceebeb24720af9e1049d7d4cb215f5080fbf5bdcbd4a\"}}";
        let balances_deposit_event = "{\"name\":\"Balances.Deposit\",\"indexInBlock\":31,\"args\":{\"amount\": \"40000000000\",\"who\":\"0x60b94741c7094ac2820cceebeb24720af9e1049d7d4cb215f5080fbf5bdcbd4a\"}}";
        let xcmp_success_event = "{\"name\":\"XcmpQueue.Success\",\"indexInBlock\":663,\"args\":{\"messageHash\":\"0xa367aeaf94deea8e4c03a90edafda41a0cddc45464859021d2c51dab5399af3c\",\"weight\":{\"refTime\":\"800000000\"}}}";
        let ump_executed_event = "{\"name\":\"Ump.ExecutedUpward\",\"indexInBlock\":35,\"args\":[\"0x0ec6dc35ff782af7a75e486524970fac6d3f07dc49564d5998842d7caf7da006\",{\"__kind\":\"Complete\",\"value\":\"4000000000\"}]}";
        let dmp_executed_event = "{\"name\": \"DmpQueue.ExecutedDownward\",\"indexInBlock\":8,\"args\":{\"messageId\":\"0x239aedd60a367e72b3fb95c34b55e096ceefd6910ec7a11866e99c5c06885ba0\",\"outcome\":{\"__kind\":\"Complete\",\"value\":\"4000000000\"}}}";

        for event in [
            assets_issued_event,
            balances_withdraw_event,
            balances_deposit_event,
            xcmp_success_event,
            ump_executed_event,
            dmp_executed_event,
        ]
        .into_iter()
        {
            ink_env::debug_println!("Will decode {}...", event);
            let (decoded, _): (graphql_helper::Event, usize) =
                serde_json_core::from_slice(event.as_bytes()).expect("deserialize failed");
            ink_env::debug_println!("Decoded: {:?}\n", decoded);
        }
    }
}
