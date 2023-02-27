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

use ink_prelude::string::{String, ToString};
use pink_web3::{
    contract::{Contract, Options},
    transports::PinkHttp,
    types::{SignedTransaction, U256},
};

use privadex_chain_metadata::common::{Amount, EthAddress, Nonce, SecretKey};

use super::common;

pub struct WethContract {
    contract: Contract<PinkHttp>,
    rpc_url: String,
}

impl WethContract {
    pub fn new(rpc_url: &str, contract_address: EthAddress) -> common::Result<Self> {
        let contract = Contract::from_json(
            common::eth(rpc_url),
            contract_address,
            include_bytes!("./eth_abi/weth_abi.json"),
        )
        .map_err(|_| common::EthError::InvalidABI)?;
        Ok(Self {
            rpc_url: rpc_url.to_string(),
            contract,
        })
    }

    /// "Wrap": Deposit native token into contract (and receive wrapped native token)
    pub fn deposit(
        &self,
        amount: Amount,
        key: &SecretKey,
        nonce: Nonce,
    ) -> common::Result<SignedTransaction> {
        let func = "deposit";
        let params = ();
        let options_seed = Options::with(|options| {
            options.value = Some(U256::from(amount));
        });
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

    /// "Unwrap": Withdraw native token from contract (and pay wrapped native token)
    pub fn withdraw(
        &self,
        amount: Amount,
        key: &SecretKey,
        nonce: Nonce,
    ) -> common::Result<SignedTransaction> {
        let func = "withdraw";
        let params = (U256::from(amount),);
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

impl common::ContractWrapper for WethContract {
    fn get_rpc_url(&self) -> &str {
        &self.rpc_url
    }
}

// Note: uncommenting some of the lines can send out a transaction.
// Prerequisites:
// 1. sender account must have sufficient funds (WDEV and DEV on Moonbase Alpha)
// 2. env var ETH_PRIVATE_KEY must be set to the sender account's secret key
#[cfg(test)]
mod weth_tests {
    use core::str::FromStr;
    use privadex_chain_metadata::{
        common::SecretKeyContainer, registry::chain::chain_info_registry,
    };

    use super::*;

    fn get_moonbase_alpha_weth_contract() -> WethContract {
        let chain_info = chain_info_registry::MOONBASEALPHA_INFO;
        let weth_address = chain_info
            .weth_addr
            .expect("WETH address exists for  Moonbase Alpha");
        WethContract::new(&chain_info.rpc_url, weth_address).expect("Invalid ABI")
    }

    #[test]
    fn wrap() {
        // Generated https://moonbase.moonscan.io/tx/0x64087d8facd2aa38fd2e94f534745892368b5c2a69840327c222afa40eaba38c
        pink_extension_runtime::mock_ext::mock_all_ext();
        let amount = 100_000_000_000_000_000;
        let kap_privkey = {
            let privkey_str =
                std::env::var("ETH_PRIVATE_KEY").expect("Env var ETH_PRIVATE_KEY is not set");
            SecretKeyContainer::from_str(&privkey_str)
                .expect("ETH_PRIVATE_KEY to_hex failed")
                .0
        };
        let nonce = 0;
        let _signed_txn = get_moonbase_alpha_weth_contract()
            .deposit(amount, &kap_privkey, nonce)
            .expect("WETH deposit txn");

        // common::print_and_send_txn(&chain_info_registry::MOONBASEALPHA_INFO.rpc_url, signed_txn);
    }

    #[test]
    fn unwrap() {
        // Generated https://moonbase.moonscan.io/tx/0x1c739cbd7a10be655cbacdac0469f9659f4330bbc966b57990861c26fd75ef7c
        pink_extension_runtime::mock_ext::mock_all_ext();
        let amount = 100_000_000_000_000;
        let kap_privkey = {
            let privkey_str =
                std::env::var("ETH_PRIVATE_KEY").expect("Env var ETH_PRIVATE_KEY is not set");
            SecretKeyContainer::from_str(&privkey_str)
                .expect("ETH_PRIVATE_KEY to_hex failed")
                .0
        };
        let nonce = 0;
        let _signed_txn = get_moonbase_alpha_weth_contract()
            .withdraw(amount, &kap_privkey, nonce)
            .expect("WETH deposit txn");

        // common::print_and_send_txn(&chain_info_registry::MOONBASEALPHA_INFO.rpc_url, signed_txn);
    }
}
