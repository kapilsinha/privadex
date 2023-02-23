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

pub mod concurrency_coordinator;
pub mod eth_utils;
pub mod executable;
pub mod extrinsic_call_factory;
pub mod key_container;
pub mod substrate_utils;

#[pink_extension::contract(env=PinkEnvironment)]
mod privadex_phat {
    use ink_env::debug_println;
    use ink_prelude::{
        string::{String, ToString},
        vec,
        vec::Vec,
    };
    use ink_storage::traits::SpreadAllocate;
    use pink_extension::PinkEnvironment;
    use scale::{Decode, Encode};
    use sp_core::Pair;

    use privadex_chain_metadata::{
        common::{
            Amount, BlockNum, EthAddress, EthTxnHash, MillisSinceEpoch, SecretKey,
            SubstratePublicKey, UniversalAddress, UniversalChainId, UniversalTokenId,
        },
        get_chain_info_from_chain_id,
        registry::chain::universal_chain_id_registry,
    };
    use privadex_common::{
        utils::general_utils::{hex_string_to_vec, slice_to_hex_string},
        uuid::Uuid,
    };
    use privadex_execution_plan::execution_plan::{
        EthPendingTxnId, EthStepStatus, ExecutionPlan, ExecutionStepEnum,
    };
    use privadex_routing::{graph::graph::GraphSolution, graph_builder, smart_order_router};

    use crate::concurrency_coordinator::execution_plan_assigner::ExecutionPlanAssigner;
    use crate::executable::{
        executable_step::TXN_NUM_BLOCKS_ALIVE,
        execute_step_meta::ExecuteStepMeta,
        traits::{Executable, ExecutableError, ExecutableSimpleStatus},
    };
    use crate::key_container::{AddressKeyPair, KeyContainer};
    use crate::substrate_utils::node_rpc_utils::SubstrateNodeRpcUtils;

    type Result<T> = core::result::Result<T, Error>;
    type HexStrNo0x = String;

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PrivaDex {
        admin: AccountId,
        escrow_eth_private_key: Option<SecretKey>,
        escrow_substrate_private_key: Option<SecretKey>,
        dynamodb_access_key: Option<String>,
        dynamodb_secret_key: Option<String>,
        s3_access_key: Option<String>,
        s3_secret_key: Option<String>,
    }

    #[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        AlreadyInitialized,
        DbRequestFailed,
        ExecutionPlanClaimedByAnotherWorker,
        FailedToCreateExecutionPlan,
        FailedToCreateGraph,
        FailedToPullExecutionPlan,
        FailedToSaveExecutionPlan,
        NoPathFound,
        NoPermissions,
        PrestartTxnIsAlreadyUsed,
        InvalidAddress,
        InvalidNumber,
        InvalidExecutionPlanUuid,
        InvalidUserToEscrowTxn,
        InvalidHexAddrString,
        InvalidTokenString,
        RpcRequestFailed,
        StepForwardFailed(ExecutableError),
        UninitializedEscrow,
        UnsupportedNetwork,
    }

    impl PrivaDex {
        #[ink(constructor)]
        pub fn new() -> Self {
            let admin = Self::env().caller();
            ink_lang::utils::initialize_contract(|this: &mut Self| {
                this.admin = admin;
                this.escrow_eth_private_key = None;
                this.escrow_substrate_private_key = None;
                this.dynamodb_access_key = None;
                this.dynamodb_secret_key = None;
                this.s3_access_key = None;
                this.s3_secret_key = None;
            })
        }

        #[ink(message)]
        pub fn init_secret_keys(
            &mut self,
            escrow_eth_private_key: HexStrNo0x, // hex string WITHOUT 0x e.g. abcdef...
            escrow_substrate_private_key: HexStrNo0x,
            dynamodb_access_key: String,
            dynamodb_secret_key: String,
            s3_secret_key: String,
            s3_access_key: String,
        ) -> Result<()> {
            if Self::env().caller() != self.admin {
                return Err(Error::NoPermissions);
            }
            if self.escrow_eth_private_key.is_some() {
                return Err(Error::AlreadyInitialized);
            }
            let eth_secret: SecretKey = io_helper::hex_str_to_u8_32(&escrow_eth_private_key)?;
            let substrate_secret: SecretKey =
                io_helper::hex_str_to_u8_32(&escrow_substrate_private_key)?;
            self.escrow_eth_private_key = Some(eth_secret);
            self.escrow_substrate_private_key = Some(substrate_secret);
            self.dynamodb_access_key = Some(dynamodb_access_key);
            self.dynamodb_secret_key = Some(dynamodb_secret_key);
            self.s3_access_key = Some(s3_access_key);
            self.s3_secret_key = Some(s3_secret_key);
            Ok(())
        }

        #[ink(message)]
        pub fn get_admin(&self) -> AccountId {
            self.admin
        }

        #[ink(message)]
        pub fn get_escrow_eth_account_address(&self) -> Result<String> {
            // We only support paths that start on Moonbeam or Astar for now, so we simply return
            // the Eth address instead of doing a match statement on network_name
            let privkey = self
                .escrow_eth_private_key
                .ok_or(Error::UninitializedEscrow)?;
            let address =
                Self::get_eth_address_from_pair(&sp_core::ecdsa::Pair::from_seed(&privkey))?;
            Ok(slice_to_hex_string(&address.0))
        }

        #[ink(message)]
        pub fn get_exec_plan(&self, exec_plan_uuid_str: HexStrNo0x) -> Result<ExecutionPlan> {
            let exec_plan_uuid = {
                let exec_plan_uuid_raw = io_helper::hex_str_to_u8_16(&exec_plan_uuid_str)?;
                Uuid::new(exec_plan_uuid_raw)
            };
            let execute_step_meta = self.create_execute_step_meta()?;
            execute_step_meta
                .pull_exec_plan_from_s3(&exec_plan_uuid)
                .map_err(|_| Error::FailedToPullExecutionPlan)
        }

        #[ink(message)]
        pub fn execution_plan_step_forward(
            &self,
            exec_plan_uuid_str: HexStrNo0x,
        ) -> Result<Option<Amount>> /* amount_out when ExecutionPlan completes */ {
            let exec_plan_uuid = {
                let exec_plan_uuid_raw = io_helper::hex_str_to_u8_16(&exec_plan_uuid_str)?;
                Uuid::new(exec_plan_uuid_raw)
            };
            let execute_step_meta = self.create_execute_step_meta()?;
            let keys = self.create_key_container()?;

            let is_claim_successful = execute_step_meta.claim_exec_plan(&exec_plan_uuid);
            if !is_claim_successful {
                return Err(Error::ExecutionPlanClaimedByAnotherWorker);
            }
            let mut exec_plan = execute_step_meta
                .pull_exec_plan_from_s3(&exec_plan_uuid)
                .map_err(|_| Error::FailedToPullExecutionPlan)?;
            let step_forward_res = {
                let result_wrapped_step_forward_res =
                    exec_plan.execute_step_forward(&execute_step_meta, &keys);
                if let Err(executable_err) = result_wrapped_step_forward_res {
                    if executable_err == ExecutableError::CalledStepForwardOnFinishedPlan {
                        let _ = execute_step_meta.remove_completed_exec_plan(&exec_plan_uuid);
                        debug_println!("Removed completed exec plan!");
                    } else {
                        // Unclaim adds the data back so we avoid doing so when we remove it. Sort of
                        // hacky, can revisit later
                        let _ = execute_step_meta.unclaim_exec_plan(&exec_plan_uuid);
                    }
                    return Err(Error::StepForwardFailed(executable_err));
                }
                result_wrapped_step_forward_res.expect("Result must be okay now")
            };

            if step_forward_res.did_status_change {
                // Discard result because there is nothing we can/need to do if it fails
                let _ = execute_step_meta.save_exec_plan_to_s3(&exec_plan);
            }
            let new_status = exec_plan.get_status();
            if new_status == ExecutableSimpleStatus::Succeeded
                || new_status == ExecutableSimpleStatus::Failed
                || new_status == ExecutableSimpleStatus::Dropped
            {
                // Discard result because there is nothing we can/need to do if it fails
                let _ = execute_step_meta.remove_completed_exec_plan(&exec_plan_uuid);
            } else {
                // TODO_lowpriority: implement this as a RAII guard for cleanliness
                // Unclaim adds the data back so we avoid doing so when we remove it. Sort of
                // hacky, can revisit later
                let _ = execute_step_meta.unclaim_exec_plan(&exec_plan_uuid);
            }

            Ok(step_forward_res.amount_out)
        }

        fn create_execute_step_meta(&self) -> Result<ExecuteStepMeta> {
            Ok(ExecuteStepMeta::new_for_astar_moonbeam_polkadot(
                self.now_millis(),
                self.s3_access_key
                    .clone()
                    .ok_or(Error::UninitializedEscrow)?,
                self.s3_secret_key
                    .clone()
                    .ok_or(Error::UninitializedEscrow)?,
                self.dynamodb_access_key
                    .clone()
                    .ok_or(Error::UninitializedEscrow)?,
                self.dynamodb_secret_key
                    .clone()
                    .ok_or(Error::UninitializedEscrow)?,
            ))
        }

        fn create_key_container(&self) -> Result<KeyContainer> {
            let eth_secret_key = self
                .escrow_eth_private_key
                .ok_or(Error::UninitializedEscrow)?
                .clone();
            let substrate_secret_key = self
                .escrow_substrate_private_key
                .ok_or(Error::UninitializedEscrow)?
                .clone();

            let eth_address =
                Self::get_eth_address_from_pair(&sp_core::ecdsa::Pair::from_seed(&eth_secret_key))?;
            let substrate_pubkey = SubstratePublicKey {
                0: sp_core::sr25519::Pair::from_seed(&substrate_secret_key)
                    .public()
                    .0,
            };

            Ok(KeyContainer {
                0: vec![
                    AddressKeyPair {
                        address: UniversalAddress::Ethereum(eth_address),
                        key: eth_secret_key,
                    },
                    AddressKeyPair {
                        address: UniversalAddress::Substrate(substrate_pubkey),
                        key: substrate_secret_key,
                    },
                ],
            })
        }

        #[ink(message)]
        pub fn start_swap(
            &self,
            user_to_escrow_transfer_eth_txn: HexStrNo0x,
            src_network_name: String,
            dest_network_name: String,
            src_eth_addr: HexStrNo0x,
            dest_eth_addr: HexStrNo0x,
            src_token: String,
            dest_token: String,
            amount_in_str: String, // String because JavaScript numbers are maxed at 2^53
        ) -> Result<Uuid> {
            let user_to_escrow_txn =
                io_helper::hex_str_to_eth_txn_hash(&user_to_escrow_transfer_eth_txn)?;
            let mut exec_plan = self.compute_execution_plan(
                src_network_name.clone(),
                dest_network_name,
                src_eth_addr,
                dest_eth_addr,
                src_token,
                dest_token,
                amount_in_str,
            )?;
            match &mut exec_plan.prestart_user_to_escrow_transfer.inner {
                ExecutionStepEnum::EthSend(step) => {
                    let cur_block =
                        Self::get_cur_block(&io_helper::chain_name_to_id(&src_network_name)?)?;
                    step.status = EthStepStatus::Submitted(EthPendingTxnId {
                        txn_hash: user_to_escrow_txn.clone(),
                        end_block_num: cur_block + TXN_NUM_BLOCKS_ALIVE,
                    });
                }
                ExecutionStepEnum::ERC20Transfer(step) => {
                    let cur_block =
                        Self::get_cur_block(&io_helper::chain_name_to_id(&src_network_name)?)?;
                    step.status = EthStepStatus::Submitted(EthPendingTxnId {
                        txn_hash: user_to_escrow_txn.clone(),
                        end_block_num: cur_block + TXN_NUM_BLOCKS_ALIVE,
                    });
                }
                _ => return Err(Error::InvalidUserToEscrowTxn),
            }
            let execute_step_meta = self.create_execute_step_meta()?;
            if !execute_step_meta.register_prestart_txn_hash(&user_to_escrow_txn) {
                return Err(Error::PrestartTxnIsAlreadyUsed);
            }
            let _ = execute_step_meta.save_exec_plan_to_s3(&exec_plan);
            let _ = execute_step_meta.register_exec_plan(&exec_plan.uuid);
            Ok(exec_plan.uuid)
        }

        fn get_cur_block(chain_id: &UniversalChainId) -> Result<BlockNum> {
            // We assume all ChainIds support Substrate-like extrinsics. Fine for the near future
            let chain_info =
                get_chain_info_from_chain_id(&chain_id).ok_or(Error::UnsupportedNetwork)?;
            let subutils = SubstrateNodeRpcUtils {
                rpc_url: chain_info.rpc_url.to_string(),
            };
            subutils
                .get_finalized_block_number()
                .map_err(|_| Error::RpcRequestFailed)
        }

        #[ink(message)]
        pub fn compute_execution_plan(
            &self,
            src_network_name: String,
            dest_network_name: String,
            src_eth_addr: HexStrNo0x,
            dest_eth_addr: HexStrNo0x,
            src_token: String,
            dest_token: String,
            amount_in_str: String,
        ) -> Result<ExecutionPlan> {
            let (graph_solution, _, _, _) = self.compute_graph_solution_with_quote(
                src_network_name,
                dest_network_name,
                src_eth_addr,
                dest_eth_addr,
                src_token,
                dest_token,
                amount_in_str,
            )?;
            let exec_plan = ExecutionPlan::try_from(graph_solution)
                .map_err(|_| Error::FailedToCreateExecutionPlan)?;
            Ok(exec_plan)
        }

        #[ink(message)]
        pub fn quote(
            &self,
            src_network_name: String,
            dest_network_name: String,
            src_token: String,
            dest_token: String,
            amount_in_str: String,
        ) -> Result<(Amount, Amount, Amount)> {
            let (_, quote, src_usd, dest_usd) = self.compute_graph_solution_with_quote(
                src_network_name,
                dest_network_name,
                "0000000000000000000000000000000000000000".to_string(), // dummy value, gets discarded for the quote
                "0000000000000000000000000000000000000000".to_string(), // dummy value, gets discarded for the quote
                src_token,
                dest_token,
                amount_in_str,
            )?;
            Ok((quote, src_usd, dest_usd))
        }

        pub fn compute_graph_solution_with_quote(
            &self,
            src_network_name: String,
            dest_network_name: String,
            src_eth_addr: HexStrNo0x,
            dest_eth_addr: HexStrNo0x,
            src_token: String,
            dest_token: String,
            amount_in_str: String,
        ) -> Result<(
            GraphSolution,
            Amount, /* quote in dest token */
            Amount, /* src token USD */
            Amount, /* dest token USD */
        )> {
            let amount_in: Amount = amount_in_str.parse().map_err(|_| Error::InvalidNumber)?;
            let src_token_id = UniversalTokenId {
                chain: io_helper::chain_name_to_id(&src_network_name)?,
                id: io_helper::token_str_to_id(&src_token)?,
            };
            let dest_token_id = UniversalTokenId {
                chain: io_helper::chain_name_to_id(&dest_network_name)?,
                id: io_helper::token_str_to_id(&dest_token)?,
            };
            let src_addr = io_helper::hex_str_to_eth_addr(&src_eth_addr)?;
            let dest_addr = io_helper::hex_str_to_eth_addr(&dest_eth_addr)?;

            let chain_ids: Vec<UniversalChainId> = vec![
                universal_chain_id_registry::ASTAR,
                universal_chain_id_registry::MOONBEAM,
                universal_chain_id_registry::POLKADOT,
            ];
            let graph = graph_builder::create_graph_from_chain_ids(&chain_ids).unwrap();
            debug_println!("Vertex count: {}", graph.simple_graph.vertex_count());
            debug_println!("Edge count: {}", graph.simple_graph.edge_count());

            let sor_config = smart_order_router::single_path_sor::SORConfig::default();
            let sor = smart_order_router::single_path_sor::SinglePathSOR::new(
                &graph,
                src_addr,
                dest_addr,
                src_token_id.clone(),
                dest_token_id.clone(),
                sor_config,
            );
            let graph_solution = sor
                .compute_graph_solution(amount_in)
                .map_err(|_| Error::NoPathFound)?;
            let src_usd_amount = graph
                .get_token(&src_token_id)
                .expect("Token is in graph since we found a path")
                .derived_usd
                .add_exp(6)
                .mul_u128(amount_in);
            let quote = graph_solution.get_quote_with_estimated_txn_fees();
            let dest_usd_amount = graph
                .get_token(&dest_token_id)
                .expect("Token is in graph since we found a path")
                .derived_usd
                .add_exp(6)
                .mul_u128(quote);
            Ok((graph_solution, quote, src_usd_amount, dest_usd_amount))
        }

        #[ink(message)]
        pub fn get_execplan_ids(&self) -> Result<Vec<Uuid>> {
            let execute_step_meta = ExecutionPlanAssigner::new(
                self.dynamodb_access_key
                    .clone()
                    .ok_or(Error::UninitializedEscrow)?,
                self.dynamodb_secret_key
                    .clone()
                    .ok_or(Error::UninitializedEscrow)?,
                self.now_millis(),
            );
            Ok(execute_step_meta.get_execplan_ids().unwrap_or_default())
        }

        fn get_eth_address_from_pair(pair: &sp_core::ecdsa::Pair) -> Result<EthAddress> {
            Self::get_eth_address_from_pubkey(&pair.public().0)
        }

        fn get_eth_address_from_pubkey(pubkey: &[u8; 33]) -> Result<EthAddress> {
            let mut address = EthAddress::zero();
            if ink_env::ecdsa_to_eth_address(pubkey, &mut address.0).is_err() {
                Err(Error::InvalidAddress)
            } else {
                Ok(address)
            }
        }

        // env().block_timestamp() is 0 off-chain so we use conditional compilation
        #[cfg(test)]
        fn now_millis(&self) -> MillisSinceEpoch {
            use std::time::SystemTime;
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis()
                .try_into()
                .unwrap()
        }

        #[cfg(not(test))]
        fn now_millis(&self) -> MillisSinceEpoch {
            self.env().block_timestamp()
        }
    }

    mod io_helper {
        use privadex_chain_metadata::{
            common::{AssetId, ChainTokenId, ERC20Token, UniversalChainId, XC20Token},
            registry::chain::universal_chain_id_registry,
        };

        use super::*;

        pub fn chain_name_to_id(chain_name: &str) -> Result<UniversalChainId> {
            match chain_name.to_lowercase().as_str() {
                "astar" => Ok(universal_chain_id_registry::ASTAR),
                "moonbeam" => Ok(universal_chain_id_registry::MOONBEAM),
                "polkadot" => Ok(universal_chain_id_registry::POLKADOT),
                _ => Err(Error::UnsupportedNetwork),
            }
        }

        pub fn token_str_to_id(token_str: &str) -> Result<ChainTokenId> {
            let lowercase_token_str = token_str.to_lowercase();
            let token_str = lowercase_token_str.as_str();
            if "native" == token_str {
                Ok(ChainTokenId::Native)
            } else if "xc20,id=" == &token_str[..8] {
                let asset_id: AssetId = token_str[8..]
                    .parse()
                    .map_err(|_| Error::InvalidTokenString)?;
                Ok(ChainTokenId::XC20(XC20Token::from_asset_id(asset_id)))
            } else if "xc20,addr=" == &token_str[..10] {
                let eth_addr = hex_str_to_eth_addr(&token_str[12..])?;
                Ok(ChainTokenId::XC20(XC20Token::from_eth_address(eth_addr)))
            } else if "erc20,addr=" == &token_str[..11] {
                let eth_addr = hex_str_to_eth_addr(&token_str[13..])?;
                Ok(ChainTokenId::ERC20(ERC20Token { addr: eth_addr }))
            } else {
                Err(Error::InvalidTokenString)
            }
        }

        pub fn hex_str_to_eth_addr(hex_str: &str) -> Result<EthAddress> {
            let raw_addr: [u8; 20] = hex_string_to_vec(&("0x".to_string() + hex_str))
                .map_err(|_| Error::InvalidHexAddrString)?
                .try_into()
                .map_err(|_| Error::InvalidHexAddrString)?;
            Ok(EthAddress { 0: raw_addr })
        }

        pub fn hex_str_to_u8_32(hex_str: &str) -> Result<[u8; 32]> {
            let raw_hash: [u8; 32] = hex_string_to_vec(&("0x".to_string() + hex_str))
                .map_err(|_| Error::InvalidHexAddrString)?
                .try_into()
                .map_err(|_| Error::InvalidHexAddrString)?;
            Ok(raw_hash)
        }

        pub fn hex_str_to_u8_16(hex_str: &str) -> Result<[u8; 16]> {
            let raw_hash: [u8; 16] = hex_string_to_vec(&("0x".to_string() + hex_str))
                .map_err(|_| Error::InvalidHexAddrString)?
                .try_into()
                .map_err(|_| Error::InvalidHexAddrString)?;
            Ok(raw_hash)
        }

        pub fn hex_str_to_eth_txn_hash(hex_str: &str) -> Result<EthTxnHash> {
            Ok(EthTxnHash {
                0: hex_str_to_u8_32(hex_str)?,
            })
        }
    }

    #[cfg(all(feature = "dynamodb-live-test", feature = "s3-live-test"))]
    #[cfg(test)]
    mod phat_tests {
        use core::str::FromStr;
        use hex_literal::hex;
        use ink_env::debug_println;
        use ink_lang as ink;
        use openbrush::traits::mock::{Addressable, SharedCallStack};

        use privadex_chain_metadata::common::{
            ChainTokenId, ERC20Token, SecretKeyContainer, XC20Token,
        };

        use super::*;

        fn get_nonull_env_var(name: &str) -> Option<String> {
            if let Ok(val) = std::env::var(name) {
                if val.len() > 0 {
                    return Some(val);
                }
            }
            ink_env::debug_println!("Env var {name} is not set");
            None
        }

        fn get_phat_contract() -> Addressable<PrivaDex> {
            let escrow_eth_private_key = SecretKeyContainer::from_str(
                &std::env::var("ETH_PRIVATE_KEY").expect("Env var ETH_PRIVATE_KEY is not set"),
            )
            .expect("ETH_PRIVATE_KEY to_hex failed")
            .0;
            let escrow_substrate_private_key = SecretKeyContainer::from_str(
                &std::env::var("SUBSTRATE_PRIVATE_KEY")
                    .expect("Env var SUBSTRATE_PRIVATE_KEY is not set"),
            )
            .expect("SUBSTRATE_PRIVATE_KEY to_hex failed")
            .0;
            let dynamodb_access_key = get_nonull_env_var("DYNAMODB_ACCESS_KEY").unwrap();
            let dynamodb_secret_key = get_nonull_env_var("DYNAMODB_SECRET_KEY").unwrap();
            let s3_access_key = get_nonull_env_var("S3_ACCESS_KEY").unwrap();
            let s3_secret_key = get_nonull_env_var("S3_SECRET_KEY").unwrap();

            let escrow_eth_pubkey = ink_env::AccountId::try_from(
                sp_core::sr25519::Pair::from_seed(&escrow_eth_private_key)
                    .public()
                    .0,
            )
            .expect("Valid account");

            let stack = SharedCallStack::new(escrow_eth_pubkey.clone());
            // Sets the stack's caller before constructor is called (which sets admin)
            stack.push(&escrow_eth_pubkey);
            let contract = Addressable::create_native(1, PrivaDex::new(), stack.clone());

            let _ = contract
                .call_mut()
                .init_secret_keys(
                    slice_to_hex_string(&escrow_eth_private_key)[2..].to_string(),
                    slice_to_hex_string(&escrow_substrate_private_key)[2..].to_string(),
                    dynamodb_access_key,
                    dynamodb_secret_key,
                    s3_secret_key,
                    s3_access_key,
                )
                .expect("Valid init");
            contract
        }

        #[ink::test]
        fn test_get_admin() {
            pink_extension_runtime::mock_ext::mock_all_ext();

            let contract = get_phat_contract();
            let admin = contract.call().get_admin();
            debug_println!("Admin: {:?}", slice_to_hex_string(admin.as_ref()));
        }

        #[ink::test]
        fn test_get_escrow_eth_account_address() {
            pink_extension_runtime::mock_ext::mock_all_ext();

            let contract = get_phat_contract();
            let addr = contract.call().get_escrow_eth_account_address();
            debug_println!("Escrow Eth account: {:?}", addr);
        }

        #[ink::test]
        fn test_token_parse() {
            pink_extension_runtime::mock_ext::mock_all_ext();

            let x =
                io_helper::token_str_to_id("erc20,addr=0x931715FEE2d06333043d11F658C8CE934aC61D0c")
                    .expect("Valid ERC20 addr");
            let y =
                io_helper::token_str_to_id("xC20,addr=0xFfFFfFff1FcaCBd218EDc0EbA20Fc2308C778080")
                    .expect("Valid XC20 addr");
            let z = io_helper::token_str_to_id("Xc20,id=42259045809535163221576417993425387648")
                .expect("Valid XC20 id");
            assert_eq!(
                x,
                ChainTokenId::ERC20(ERC20Token {
                    addr: EthAddress {
                        0: hex!("931715FEE2d06333043d11F658C8CE934aC61D0c")
                    }
                })
            );
            assert_eq!(
                y,
                ChainTokenId::XC20(XC20Token::from_eth_address(EthAddress {
                    0: hex!("FfFFfFff1FcaCBd218EDc0EbA20Fc2308C778080")
                }))
            );
            assert_eq!(z, y);
        }

        #[ink::test]
        fn test_compute_exec_plan() {
            pink_extension_runtime::mock_ext::mock_all_ext();

            let contract = get_phat_contract();
            let exec_plan = contract.call().compute_execution_plan(
                "astar".to_string(),
                "moonbeam".to_string(),
                "90204F4683D20367ae8044CfE23aC63e87C996CE".to_string(),
                "42B7D766824422F499F84703eC4E2abb273171cF".to_string(),
                "native".to_string(),
                "erc20,addr=0x931715FEE2d06333043d11F658C8CE934aC61D0c".to_string(), // USDC_wormhole
                "100000000000000000000".to_string(),
            );
            debug_println!("Execution plan: {:?}", exec_plan);
        }

        #[ink::test]
        fn test_quote() {
            pink_extension_runtime::mock_ext::mock_all_ext();

            let contract = get_phat_contract();
            let quote = contract.call().quote(
                "astar".to_string(),
                "moonbeam".to_string(),
                "native".to_string(),
                "erc20,addr=0x931715FEE2d06333043d11F658C8CE934aC61D0c".to_string(), // USDC_wormhole
                "100000000000000000000".to_string(),
            );
            debug_println!("Quote: {:?}", quote);
        }

        #[ink::test]
        fn test_start_swap() {
            pink_extension_runtime::mock_ext::mock_all_ext();

            let contract = get_phat_contract();
            let exec_plan_uuid = contract
                .call()
                .start_swap(
                    "d471de9980d69157cbdefbbb659b63c9edcc4855fc65d0898191aad5b160a80a".to_string(),
                    "astar".to_string(),
                    "moonbeam".to_string(),
                    "90204F4683D20367ae8044CfE23aC63e87C996CE".to_string(),
                    "42B7D766824422F499F84703eC4E2abb273171cF".to_string(),
                    "native".to_string(),
                    "erc20,addr=0x931715FEE2d06333043d11F658C8CE934aC61D0c".to_string(), // USDC_wormhole
                    "100000000000000000000".to_string(),
                )
                .expect("Should save execution plan into S3");
            debug_println!("Saved execution plan in S3 with UUID {:?}", exec_plan_uuid);
        }

        #[ink::test]
        fn test_get_execplan_ids() {
            pink_extension_runtime::mock_ext::mock_all_ext();

            let contract = get_phat_contract();
            let exec_plan_ids = contract.call().get_execplan_ids();
            debug_println!("Execution plans: {:?}", exec_plan_ids);
        }
    }
}
