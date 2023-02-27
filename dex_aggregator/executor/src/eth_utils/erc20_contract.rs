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
    signing::keccak256,
    transports::{resolve_ready, PinkHttp},
    types::{Log, SignedTransaction, U256},
};

use privadex_chain_metadata::common::{Amount, EthAddress, EthTxnHash, Nonce, SecretKey};

use super::common;

pub struct ERC20Contract {
    contract: Contract<PinkHttp>,
    rpc_url: String,
}

impl ERC20Contract {
    pub fn new(rpc_url: &str, contract_address: EthAddress) -> common::Result<Self> {
        let contract = Contract::from_json(
            common::eth(rpc_url),
            contract_address,
            include_bytes!("./eth_abi/erc20_abi.json"),
        )
        .map_err(|_| common::EthError::InvalidABI)?;
        Ok(Self {
            rpc_url: rpc_url.to_string(),
            contract,
        })
    }

    pub fn parse_transfer_log(
        log: &Log,
        is_txn_success: bool,
        gas_fee_native: Amount,
    ) -> common::Result<common::ERC20Transfer> {
        let topic = EthTxnHash {
            0: keccak256("Transfer(address,address,uint256)".as_bytes()),
        };
        if log.topics.len() != 3 || topic != log.topics[0] {
            return Err(common::EthError::ParseFailed);
        }
        let amount_u256 = U256::from_big_endian(&log.data.0);
        let amount = common::u256_to_u128(amount_u256)?;
        let x = common::ERC20Transfer {
            is_txn_success,
            token: log.address.into(),
            from: log.topics[1].into(),
            to: log.topics[2].into(),
            amount,
            gas_fee_native,
        };
        Ok(x)
    }

    pub fn name(&self) -> common::Result<String> {
        // block = BlockId::Number(BlockNumber::Latest) used in some tests
        let x = resolve_ready(
            self.contract
                .query("name", (), None, Options::default(), None),
        );
        // println!("Resolution: {:?}", x);
        x.map_err(|_| common::EthError::ContractCallFailed)
    }

    pub fn decimals(&self) -> common::Result<u8> {
        let x = resolve_ready(
            self.contract
                .query("decimals", (), None, Options::default(), None),
        );
        // println!("Resolution: {:?}", x);
        x.map_err(|_| common::EthError::ContractCallFailed)
    }

    pub fn balance_of(&self, who: EthAddress) -> common::Result<Amount> {
        let x =
            resolve_ready(
                self.contract
                    .query("balanceOf", (who,), None, Options::default(), None),
            );
        // println!("Resolution: {:?}", x);
        let amount_u256 = x.map_err(|_| common::EthError::ContractCallFailed)?;
        common::u256_to_u128(amount_u256)
    }

    pub fn transfer(
        &self,
        to: EthAddress,
        amount: Amount,
        key: &SecretKey,
        nonce: Nonce,
    ) -> common::Result<SignedTransaction> {
        let func = "transfer";
        let params = (to, U256::from(amount));
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

impl common::ContractWrapper for ERC20Contract {
    fn get_rpc_url(&self) -> &str {
        &self.rpc_url
    }
}

// Note: uncommenting some of the lines can send out a transaction.
// Prerequisites:
// 1. sender account must have sufficient funds
// 2. env var ETH_PRIVATE_KEY must be set to the sender account's secret key
#[cfg(test)]
mod erc20_tests {
    use core::str::FromStr;
    use hex_literal::hex;
    use privadex_chain_metadata::{
        common::SecretKeyContainer, registry::chain::chain_info_registry,
    };

    use super::*;

    fn get_moonbeam_token_contract() -> ERC20Contract {
        let token_address = EthAddress {
            0: hex!("FfFFfFff1FcaCBd218EDc0EbA20Fc2308C778080"),
        }; // xcDOT
        ERC20Contract::new(&chain_info_registry::MOONBEAM_INFO.rpc_url, token_address)
            .expect("Invalid ABI")
    }

    fn get_moonbase_alpha_token_contract() -> ERC20Contract {
        let token_address = EthAddress {
            0: hex!("CdF746C5C86Df2c2772d2D36E227B4c0203CbA25"),
        }; // VEN
        ERC20Contract::new(
            &chain_info_registry::MOONBASEALPHA_INFO.rpc_url,
            token_address,
        )
        .expect("Invalid ABI")
    }

    #[test]
    fn erc20_name() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let name = get_moonbeam_token_contract()
            .name()
            .expect("Request failed");
        assert_eq!(name, "xcDOT");
    }

    #[test]
    fn erc20_decimals() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let decimals = get_moonbeam_token_contract()
            .decimals()
            .expect("Request failed");
        assert_eq!(decimals, 10);
    }

    #[test]
    fn erc20_balance_of() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let user = EthAddress {
            0: hex!("c6e37086d09ec2048f151d11cdb9f9bbbdb7d685"),
        };
        let balance = get_moonbeam_token_contract()
            .balance_of(user)
            .expect("Request failed");
        // println!("Balance: {:?}", balance);
        assert!(balance > 10000000000000000);
    }

    #[test]
    fn erc20_transfer() {
        // Generated https://moonbase.moonscan.io/tx/0x0e73d6651fe1f6d496cd0e4c0e343d8c8544a3afd12c0a0fcea3577f1b28a80b
        pink_extension_runtime::mock_ext::mock_all_ext();
        let to = EthAddress {
            0: hex!("573394b77fc17f91e9e67f147a9ece24d67c5073"),
        };
        let amount = 1_000_000_000_000_000_000;
        let kap_privkey = {
            let privkey_str =
                std::env::var("ETH_PRIVATE_KEY").expect("Env var ETH_PRIVATE_KEY is not set");
            SecretKeyContainer::from_str(&privkey_str)
                .expect("ETH_PRIVATE_KEY to_hex failed")
                .0
        };
        let nonce = 0;
        let _signed_txn = get_moonbase_alpha_token_contract()
            .transfer(to, amount, &kap_privkey, nonce)
            .expect("Signed ERC20 transfer txn");

        // common::print_and_send_txn(&chain_info_registry::MOONBASEALPHA_INFO.rpc_url, signed_txn);
    }
}
