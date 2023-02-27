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

use ink_prelude::{
    string::{String, ToString},
    vec::Vec,
};
use pink_web3::{
    contract::{Contract, Options},
    transports::{resolve_ready, PinkHttp},
    types::{SignedTransaction, U256},
};

use privadex_chain_metadata::common::{Amount, EthAddress, MillisSinceEpoch, Nonce, SecretKey};

use super::common;

pub struct DEXRouterContract {
    contract: Contract<PinkHttp>,
    rpc_url: String,
}

impl DEXRouterContract {
    pub fn new(rpc_url: &str, contract_address: EthAddress) -> common::Result<Self> {
        let contract = Contract::from_json(
            common::eth(rpc_url),
            contract_address,
            include_bytes!("./eth_abi/dexrouter_abi.json"),
        )
        .map_err(|_| common::EthError::InvalidABI)?;
        Ok(Self {
            rpc_url: rpc_url.to_string(),
            contract,
        })
    }

    pub fn factory(&self) -> common::Result<EthAddress> {
        // block = BlockId::Number(BlockNumber::Latest) used in some tests
        let x = resolve_ready(
            self.contract
                .query("factory", (), None, Options::default(), None),
        );
        // println!("Resolution: {:?}", x);
        x.map_err(|_| common::EthError::ContractCallFailed)
    }

    pub fn weth(&self) -> common::Result<EthAddress> {
        let x = resolve_ready(
            self.contract
                .query("WETH", (), None, Options::default(), None),
        );
        // println!("Resolution: {:?}", x);
        x.map_err(|_| common::EthError::ContractCallFailed)
    }

    pub fn quote(
        &self,
        amount_a: Amount,
        reserve_a: Amount,
        reserve_b: Amount,
    ) -> common::Result<Amount> {
        let x = resolve_ready(self.contract.query(
            "quote",
            (
                U256::from(amount_a),
                U256::from(reserve_a),
                U256::from(reserve_b),
            ),
            None,
            Options::default(),
            None,
        ));
        let amount_u256 = x.map_err(|_| common::EthError::ContractCallFailed)?;
        common::u256_to_u128(amount_u256)
    }

    pub fn swap_exact_tokens_for_tokens(
        &self,
        amount_in: Amount,
        amount_out_min: Amount,
        path: Vec<EthAddress>,
        to: EthAddress,
        deadline: MillisSinceEpoch,
        key: &SecretKey,
        nonce: Nonce,
    ) -> common::Result<SignedTransaction> {
        let func = "swapExactTokensForTokens";
        let params = (
            U256::from(amount_in),
            U256::from(amount_out_min),
            path.clone(),
            to,
            U256::from(deadline),
        );
        let options_seed = Options::default();
        common::create_raw_txn(
            &self.rpc_url,
            &self.contract,
            func,
            0,
            params,
            options_seed,
            key,
            nonce,
        )
    }

    pub fn swap_exact_eth_for_tokens(
        &self,
        amount_in: Amount,
        amount_out_min: Amount,
        path: Vec<EthAddress>,
        to: EthAddress,
        deadline: MillisSinceEpoch,
        key: &SecretKey,
        nonce: Nonce,
    ) -> common::Result<SignedTransaction> {
        let func = "swapExactETHForTokens";
        let params = (
            U256::from(amount_out_min),
            path.clone(),
            to,
            U256::from(deadline),
        );
        let options_seed = Options::with(|options| options.value = Some(U256::from(amount_in)));
        common::create_raw_txn(
            &self.rpc_url,
            &self.contract,
            func,
            0,
            params,
            options_seed,
            key,
            nonce,
        )
    }

    pub fn swap_exact_tokens_for_eth(
        &self,
        amount_in: Amount,
        amount_out_min: Amount,
        path: Vec<EthAddress>,
        to: EthAddress,
        deadline: MillisSinceEpoch,
        key: &SecretKey,
        nonce: Nonce,
    ) -> common::Result<SignedTransaction> {
        let func = "swapExactTokensForETH";
        let params = (
            U256::from(amount_in),
            U256::from(amount_out_min),
            path.clone(),
            to,
            U256::from(deadline),
        );
        let options_seed = Options::default();
        common::create_raw_txn(
            &self.rpc_url,
            &self.contract,
            func,
            0,
            params,
            options_seed,
            key,
            nonce,
        )
    }
}

impl common::ContractWrapper for DEXRouterContract {
    fn get_rpc_url(&self) -> &str {
        &self.rpc_url
    }
}

// Note 1: uncommenting some of the lines can send out a transaction.
// Note 2: These do not specify a nonce (defaulting to the current nonce),
// so the tests that send out a txn must be run one at a time
// Prerequisites:
// 1. src token (e.g. VEN on Moonbase Alpha) must be approved for spending.
// Otherwise you will get GasEstimateFailed errors before the txn is sent
// (which buries a 'revert TransferHelper::transferFrom: transferFrom failed' error)
// 2. env var ETH_PRIVATE_KEY must be set to the sender account's secret key
#[cfg(test)]
mod dexrouter_tests {
    use core::str::FromStr;
    use hex_literal::hex;
    use privadex_chain_metadata::{
        common::SecretKeyContainer, get_dexes_from_chain_id, registry::chain::chain_info_registry,
    };

    use super::*;

    fn get_moonbeam_contract() -> DEXRouterContract {
        let chain_info = chain_info_registry::MOONBEAM_INFO;
        let dex = get_dexes_from_chain_id(&chain_info.chain_id)[0];
        DEXRouterContract::new(&chain_info.rpc_url, dex.eth_dex_router).expect("Invalid ABI")
    }

    fn get_moonbase_alpha_contract() -> DEXRouterContract {
        let chain_info = chain_info_registry::MOONBASEALPHA_INFO;
        let dex = get_dexes_from_chain_id(&chain_info.chain_id)[0];
        DEXRouterContract::new(&chain_info.rpc_url, dex.eth_dex_router).expect("Invalid ABI")
    }

    #[test]
    fn test_moonbeam_dexrouter_factory() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let factory = get_moonbeam_contract().factory().expect("Request failed");
        assert_eq!(
            factory,
            EthAddress {
                0: hex!("68a384d826d3678f78bb9fb1533c7e9577dacc0e")
            }
        );
    }

    #[test]
    fn test_moonbeam_dexrouter_weth() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let weth = get_moonbeam_contract().weth().expect("Request failed");
        assert_eq!(
            weth,
            EthAddress {
                0: hex!("acc15dc74880c9944775448304b263d191c6077f")
            }
        );
    }

    #[test]
    fn test_moonbeam_dexrouter_quote() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let quote = get_moonbeam_contract()
            .quote(100, 1000, 2000)
            .expect("Request failed");
        assert_eq!(quote, 200);
    }

    #[test]
    fn test_moonbase_dexrouter_swap_exact_tokens_for_tokens_txn() {
        // Generated https://moonbase.moonscan.io/tx/0x8758067010d4f67b2620bd77a0268426d0fbb3e91913b4c87b2fb83002819c9d
        pink_extension_runtime::mock_ext::mock_all_ext();
        let amount_in = 1_000_000_000_000_000_000;
        let amount_out_min = 0;
        let path = vec![
            EthAddress {
                0: hex!("CdF746C5C86Df2c2772d2D36E227B4c0203CbA25"),
            }, // VEN
            EthAddress {
                0: hex!("08B40414525687731C23F430CEBb424b332b3d35"),
            }, // ERTH
        ];
        let to = EthAddress {
            0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
        };
        let deadline = MillisSinceEpoch::MAX;
        let kap_privkey = {
            let privkey_str =
                std::env::var("ETH_PRIVATE_KEY").expect("Env var ETH_PRIVATE_KEY is not set");
            SecretKeyContainer::from_str(&privkey_str)
                .expect("ETH_PRIVATE_KEY to_hex failed")
                .0
        };
        let nonce = 0;
        let _signed_txn = get_moonbase_alpha_contract()
            .swap_exact_tokens_for_tokens(
                amount_in,
                amount_out_min,
                path,
                to,
                deadline,
                &kap_privkey,
                nonce,
            )
            .expect("Expect swap tokens for tokens");

        // common::print_and_send_txn(chain_info.rpc_url, signed_txn);
    }

    #[test]
    fn test_moonbase_dexrouter_swap_exact_eth_for_tokens_txn() {
        // Generated https://moonbase.moonscan.io/tx/0x9bce83e766f58da236fa859fc5b31c2ccc74cb5923c9d2fd9596ee2a69d7a905
        pink_extension_runtime::mock_ext::mock_all_ext();
        let chain_info = chain_info_registry::MOONBASEALPHA_INFO;
        let amount_in = 100_000_000_000_000_000;
        let amount_out_min = 0;
        let path = vec![
            chain_info.weth_addr.expect("WETH address"),
            EthAddress {
                0: hex!("CdF746C5C86Df2c2772d2D36E227B4c0203CbA25"),
            }, // VEN
        ];
        let to = EthAddress {
            0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
        };
        let deadline = MillisSinceEpoch::MAX;
        let kap_privkey = {
            let privkey_str =
                std::env::var("ETH_PRIVATE_KEY").expect("Env var ETH_PRIVATE_KEY is not set");
            SecretKeyContainer::from_str(&privkey_str)
                .expect("ETH_PRIVATE_KEY to_hex failed")
                .0
        };
        let nonce = 0;
        let _signed_txn = get_moonbase_alpha_contract()
            .swap_exact_eth_for_tokens(
                amount_in,
                amount_out_min,
                path,
                to,
                deadline,
                &kap_privkey,
                nonce,
            )
            .expect("Expect swap eth for tokens");

        // common::print_and_send_txn(chain_info.rpc_url, signed_txn);
    }

    #[test]
    fn test_moonbase_dexrouter_swap_exact_tokens_for_eth_txn() {
        // Generated https://moonbase.moonscan.io/tx/0xa5ea4631874786256c993aef7b5f13167c7fd1a1ecc529fe020ee5d74f28d64c
        pink_extension_runtime::mock_ext::mock_all_ext();
        let chain_info = chain_info_registry::MOONBASEALPHA_INFO;
        let amount_in = 1_000_000_000_000_000_000;
        let amount_out_min = 0;
        let path = vec![
            EthAddress {
                0: hex!("CdF746C5C86Df2c2772d2D36E227B4c0203CbA25"),
            }, // VEN
            chain_info.weth_addr.expect("WETH address"),
        ];
        let to = EthAddress {
            0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
        };
        let deadline = MillisSinceEpoch::MAX;
        let kap_privkey = {
            let privkey_str =
                std::env::var("ETH_PRIVATE_KEY").expect("Env var ETH_PRIVATE_KEY is not set");
            SecretKeyContainer::from_str(&privkey_str)
                .expect("ETH_PRIVATE_KEY to_hex failed")
                .0
        };
        let nonce = 0;
        let _signed_txn = get_moonbase_alpha_contract()
            .swap_exact_tokens_for_eth(
                amount_in,
                amount_out_min,
                path,
                to,
                deadline,
                &kap_privkey,
                nonce,
            )
            .expect("Expect swap tokens for eth");

        // common::print_and_send_txn(chain_info.rpc_url, signed_txn);
    }
}
