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

use duplicate::duplicate_item;
use ink_prelude::{
    string::{String, ToString},
    vec,
};
use pink_web3::{
    contract::{Contract, Options},
    transports::PinkHttp,
    types::{SignedTransaction, U256},
};

use privadex_chain_metadata::{
    common::{
        Amount, ChainTokenId, EthAddress, Nonce, ParachainId, SecretKey, SubstratePublicKey,
        UniversalAddress, UniversalChainId, UniversalTokenId,
    },
    registry::{chain::universal_chain_id_registry, token::universal_token_id_registry},
};

use super::common;

pub struct AstarXcmContract {
    contract: Contract<PinkHttp>,
    rpc_url: String,
}

impl AstarXcmContract {
    pub fn new(rpc_url: &str) -> common::Result<Self> {
        const ASTAR_XCM_PRECOMPILE_ADDRESS: EthAddress = EthAddress {
            0: hex_literal::hex!("0000000000000000000000000000000000005004"),
        };
        let contract = Contract::from_json(
            common::eth(rpc_url),
            ASTAR_XCM_PRECOMPILE_ADDRESS,
            include_bytes!("./eth_abi/astar_xcm_abi.json"),
        )
        .map_err(|_| common::EthError::InvalidABI)?;
        Ok(Self {
            rpc_url: rpc_url.to_string(),
            contract,
        })
    }

    pub fn assets_xcm_transfer(
        &self,
        src_token: &UniversalTokenId,
        amount: Amount,
        dest_chain: UniversalChainId,
        dest_addr: UniversalAddress,
        key: &SecretKey,
        nonce: Nonce,
    ) -> common::Result<SignedTransaction> {
        // I'm not sure of the difference between the functions other than
        // 1. Astar portal supports just 'assets_withdraw', and
        // 2. ASTR native token can be sent from an Astar EVM account via
        // 'assets_reserve_transfer' but not 'assets_withdraw'
        // We use that logic to delegate below
        match src_token {
            UniversalTokenId {
                chain: universal_chain_id_registry::ASTAR,
                id: ChainTokenId::XC20(_),
            } => self.assets_withdraw(src_token, amount, dest_chain, dest_addr, key, nonce),
            &universal_token_id_registry::ASTR_NATIVE => {
                self.assets_reserve_transfer(src_token, amount, dest_chain, dest_addr, key, nonce)
            }
            // Native token is NOT supported apparently for assets_withdraw
            _ => Err(common::EthError::InvalidArgument),
        }
    }

    fn assets_withdraw(
        &self,
        src_token: &UniversalTokenId,
        amount: Amount,
        dest_chain: UniversalChainId,
        dest_addr: UniversalAddress,
        key: &SecretKey,
        nonce: Nonce,
    ) -> common::Result<SignedTransaction> {
        let func = "assets_withdraw";
        let asset_id = match src_token {
            UniversalTokenId {
                chain: universal_chain_id_registry::ASTAR,
                id: ChainTokenId::XC20(addr),
            } => Ok(addr.get_eth_address()),
            // Native token is NOT supported apparently for assets_withdraw
            _ => Err(common::EthError::InvalidArgument),
        }?;
        self.dispatch_helper(func, asset_id, amount, dest_chain, dest_addr, key, nonce)
    }

    fn assets_reserve_transfer(
        &self,
        src_token: &UniversalTokenId,
        amount: Amount,
        dest_chain: UniversalChainId,
        dest_addr: UniversalAddress,
        key: &SecretKey,
        nonce: Nonce,
    ) -> common::Result<SignedTransaction> {
        let func = "assets_reserve_transfer";
        let asset_id = match src_token {
            UniversalTokenId {
                chain: universal_chain_id_registry::ASTAR,
                id: ChainTokenId::XC20(addr),
            } => Ok(addr.get_eth_address()),
            // Special zero address means the native token:
            // https://docs.astar.network/docs/xcm/building-with-xcm/xc-reserve-transfer
            &universal_token_id_registry::ASTR_NATIVE => Ok(EthAddress::zero()),
            _ => Err(common::EthError::InvalidArgument),
        }?;
        self.dispatch_helper(func, asset_id, amount, dest_chain, dest_addr, key, nonce)
    }

    fn dispatch_helper(
        &self,
        func: &str,
        asset_id: EthAddress,
        amount: Amount,
        dest_chain: UniversalChainId,
        dest_addr: UniversalAddress,
        key: &SecretKey,
        nonce: Nonce,
    ) -> common::Result<SignedTransaction> {
        let (is_relay, para_id) = {
            if let UniversalChainId::SubstrateParachain(_, id) = dest_chain {
                (false, id)
            } else {
                (true, 0)
            }
        };
        let options_seed = Options::default();
        match dest_addr {
            UniversalAddress::Ethereum(addr) => self.address20_helper(
                asset_id,
                amount,
                addr,
                is_relay,
                para_id,
                key,
                func,
                // overload_index: the address type for recipient_account_id comes second in the ABI
                0,
                options_seed,
                nonce,
            ),
            UniversalAddress::Substrate(addr) => self.address32_helper(
                asset_id,
                amount,
                addr,
                is_relay,
                para_id,
                key,
                func,
                // overload_index: the bytes32 type for recipient_account_id comes second in the ABI
                1,
                options_seed,
                nonce,
            ),
        }
    }

    #[duplicate_item(
        func_name           dest_addr_type;
        [address20_helper]  [EthAddress];
        [address32_helper]  [SubstratePublicKey];
    )]
    fn func_name(
        &self,
        asset_id: EthAddress,
        amount: Amount,
        dest_addr: dest_addr_type,
        is_relay: bool,
        para_id: ParachainId,
        key: &SecretKey,
        func: &str,
        overload_index: u8,
        options_seed: Options,
        nonce: Nonce,
    ) -> common::Result<SignedTransaction> {
        let params = (
            vec![asset_id],
            vec![U256::from(amount)],
            dest_addr,
            is_relay,
            U256::from(para_id),
            U256::zero(), /* fee_index in asset_id array */
        );
        common::create_raw_txn(
            &self.rpc_url,
            &self.contract,
            func,
            overload_index,
            params,
            options_seed,
            key,
            nonce,
        )
    }
}

impl common::ContractWrapper for AstarXcmContract {
    fn get_rpc_url(&self) -> &str {
        &self.rpc_url
    }
}

// Note 1: uncommenting some of the lines can send out an actual transaction.
// Note 2: These do not specify a nonce (defaulting to the current nonce),
// so the tests that send out a txn must be run one at a time
// Prerequisites:
// 1. env var ETH_PRIVATE_KEY must be set to the sender account's secret key
#[cfg(test)]
mod astar_xcm_precompile_tests {
    use core::str::FromStr;
    use hex_literal::hex;
    use ink_env::debug_println;

    use privadex_chain_metadata::common::{SecretKeyContainer, UniversalChainId};
    use privadex_common::utils::general_utils::slice_to_hex_string;

    use super::*;

    fn get_contract() -> AstarXcmContract {
        let rpc_url = "https://astar.public.blastapi.io";
        AstarXcmContract::new(&rpc_url).expect("Invalid ABI")
    }

    fn get_args_glmr_to_moonbeam() -> (
        UniversalTokenId,
        Amount,
        UniversalChainId,
        UniversalAddress,
        SecretKey,
        Nonce,
    ) {
        let amount = 200_000_000_000_000_000;
        let src_token = universal_token_id_registry::GLMR_ASTAR;
        let dest_chain = universal_chain_id_registry::MOONBEAM;
        let dest_addr = UniversalAddress::Ethereum(EthAddress {
            0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
        });
        let kap_privkey = {
            let privkey_str =
                std::env::var("ETH_PRIVATE_KEY").expect("Env var ETH_PRIVATE_KEY is not set");
            SecretKeyContainer::from_str(&privkey_str)
                .expect("ETH_PRIVATE_KEY to_hex failed")
                .0
        };
        let nonce = 3;
        (src_token, amount, dest_chain, dest_addr, kap_privkey, nonce)
    }

    fn get_args_dot_to_polkadot() -> (
        UniversalTokenId,
        Amount,
        UniversalChainId,
        UniversalAddress,
        SecretKey,
        Nonce,
    ) {
        let amount = 11_000_000_000;
        let src_token = universal_token_id_registry::DOT_ASTAR;
        let dest_chain = universal_chain_id_registry::POLKADOT;
        let dest_addr = UniversalAddress::Substrate(SubstratePublicKey {
            0: hex!("7011b670bb662eedbd60a1c4c11b7c197ec22e7cfe87df00013ca2c494f3b01a"),
        });
        let kap_privkey = {
            let privkey_str =
                std::env::var("ETH_PRIVATE_KEY").expect("Env var ETH_PRIVATE_KEY is not set");
            SecretKeyContainer::from_str(&privkey_str)
                .expect("ETH_PRIVATE_KEY to_hex failed")
                .0
        };
        let nonce = 1;
        (src_token, amount, dest_chain, dest_addr, kap_privkey, nonce)
    }

    #[test]
    fn assets_withdraw_to_address20() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        // This test corresponds to sending 0.2 GLMR from Astar to Moonbeam
        // https://blockscout.com/astar/tx/0x7cff8ca9d95af9fc4dcacd87deae1feef1a1176629c9d8e965344b3f2343039a
        // Compare to the following input data (the data payload in the signed txn):
        // 0xecf766ff00000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000010000000000000000000000000005a81d8564a3ea298660e34e03e5eff9a29d7a2a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007d400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000ffffffff00000000000000010000000000000003000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000002c68af0bb140000
        // ^Generated via curl request to the above txn:
        // curl https://astar.public.blastapi.io -X POST -H "Content-Type: application/json" --data '{"method":"eth_getTransactionByHash","params":["0x7cff8ca9d95af9fc4dcacd87deae1feef1a1176629c9d8e965344b3f2343039a"],"id":1,"jsonrpc":"2.0"}'
        // Use https://lab.miguelmota.com/ethereum-input-data-decoder/example/ to decode the data using the ABI
        let (src_token, amount, dest_chain, dest_addr, kap_privkey, nonce) =
            get_args_glmr_to_moonbeam();
        let signed_txn = get_contract()
            .assets_withdraw(
                &src_token,
                amount,
                dest_chain,
                dest_addr,
                &kap_privkey,
                nonce,
            )
            .expect("Create txn failed");

        let raw_txn_str = slice_to_hex_string(&signed_txn.raw_transaction.0);
        let expected_input_data = "ecf766ff00000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000010000000000000000000000000005a81d8564a3ea298660e34e03e5eff9a29d7a2a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007d400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000ffffffff00000000000000010000000000000003000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000002c68af0bb140000";
        assert!(raw_txn_str.contains(expected_input_data));

        // This is definitely improper decoding, but it helps us visually analyze the signed transaction in these unit tests
        let estimated_input_str = &raw_txn_str[78..(raw_txn_str.len() - 138)];
        debug_println!(
            "Raw txn: {}\n(Estimated) data input:{}\nTxn hash: {}",
            slice_to_hex_string(&signed_txn.raw_transaction.0),
            estimated_input_str,
            slice_to_hex_string(&signed_txn.transaction_hash.0),
        );
    }

    #[test]
    fn assets_xcm_transfer_xc20_to_address20() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        // Identical to the assets_withdraw_to_address20 test
        let (src_token, amount, dest_chain, dest_addr, kap_privkey, nonce) =
            get_args_glmr_to_moonbeam();
        let signed_txn = get_contract()
            .assets_xcm_transfer(
                &src_token,
                amount,
                dest_chain,
                dest_addr,
                &kap_privkey,
                nonce,
            )
            .expect("Create txn failed");

        let raw_txn_str = slice_to_hex_string(&signed_txn.raw_transaction.0);
        let expected_input_data = "ecf766ff00000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000010000000000000000000000000005a81d8564a3ea298660e34e03e5eff9a29d7a2a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007d400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000ffffffff00000000000000010000000000000003000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000002c68af0bb140000";
        assert!(raw_txn_str.contains(expected_input_data));
    }

    #[test]
    fn assets_reserve_asset_transfer_native_token_to_address20() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let (_, amount, dest_chain, dest_addr, kap_privkey, nonce) = get_args_glmr_to_moonbeam();
        let src_token = universal_token_id_registry::ASTR_NATIVE;
        let signed_txn = get_contract()
            .assets_reserve_transfer(
                &src_token,
                amount,
                dest_chain,
                dest_addr,
                &kap_privkey,
                nonce,
            )
            .expect("Create txn failed");

        let raw_txn_str = slice_to_hex_string(&signed_txn.raw_transaction.0);
        let expected_input_data = "106d59fe00000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000010000000000000000000000000005a81d8564a3ea298660e34e03e5eff9a29d7a2a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007d4000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000002c68af0bb140000";
        assert!(raw_txn_str.contains(expected_input_data));
    }

    #[test]
    fn assets_xcm_transfer_native_token_to_address20() {
        // Generated https://blockscout.com/astar/tx/0xc3f43b5837a228337dda7d8597b2a93f7e1bc3b570052be19f4d0a52ece6e3a3
        // (Moonbeam receive is at https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fmoonbeam.api.onfinality.io%2Fpublic-ws#/explorer/query/0x2d74f155203e89a133dd2a3cd18f55e91ddd436739358cc42265571e5828f93a)
        pink_extension_runtime::mock_ext::mock_all_ext();
        // Identical to the assets_reserve_asset_transfer_native_token_to_address20 test
        let (_, amount, dest_chain, dest_addr, kap_privkey, nonce) = get_args_glmr_to_moonbeam();
        let src_token = universal_token_id_registry::ASTR_NATIVE;
        let signed_txn = get_contract()
            .assets_xcm_transfer(
                &src_token,
                amount,
                dest_chain,
                dest_addr,
                &kap_privkey,
                nonce,
            )
            .expect("Create txn failed");

        let raw_txn_str = slice_to_hex_string(&signed_txn.raw_transaction.0);
        let expected_input_data = "106d59fe00000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000010000000000000000000000000005a81d8564a3ea298660e34e03e5eff9a29d7a2a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007d4000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000002c68af0bb140000";
        assert!(raw_txn_str.contains(expected_input_data));
        // common::print_and_send_txn(chain_info_registry::ASTAR_INFO.rpc_url, signed_txn);
    }

    #[test]
    fn assets_withdraw_create_txn_to_address32() {
        // Compare to the following input data (the data payload in the signed txn):
        // 0x019054d000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000001007011b670bb662eedbd60a1c4c11b7c197ec22e7cfe87df00013ca2c494f3b01a0000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000ffffffffffffffffffffffffffffffffffffffff0000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000028fa6ae00
        // ^Generated by using the Astar portal to transfer 1.1 DOT from Astar EVM to Polkadot
        // (Inspect element and change the 'Confirm' button from 'disabled' to 'enabled' if you don't have funds)
        // Use https://lab.miguelmota.com/ethereum-input-data-decoder/example/ to decode the data using the ABI
        pink_extension_runtime::mock_ext::mock_all_ext();
        let (src_token, amount, dest_chain, dest_addr, kap_privkey, nonce) =
            get_args_dot_to_polkadot();
        let signed_txn = get_contract()
            .assets_withdraw(
                &src_token,
                amount,
                dest_chain,
                dest_addr,
                &kap_privkey,
                nonce,
            )
            .expect("Create txn failed");

        let raw_txn_str = slice_to_hex_string(&signed_txn.raw_transaction.0);
        let expected_input_data = "019054d000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000001007011b670bb662eedbd60a1c4c11b7c197ec22e7cfe87df00013ca2c494f3b01a0000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000ffffffffffffffffffffffffffffffffffffffff0000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000028fa6ae00";
        assert!(raw_txn_str.contains(expected_input_data));
    }

    #[test]
    fn assets_reserve_transfer_to_address20() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let (src_token, amount, dest_chain, dest_addr, kap_privkey, nonce) =
            get_args_glmr_to_moonbeam();
        let _signed_txn = get_contract()
            .assets_reserve_transfer(
                &src_token,
                amount,
                dest_chain,
                dest_addr,
                &kap_privkey,
                nonce,
            )
            .expect("Create txn failed");

        // let estimated_input_str = &raw_txn_str[78..(raw_txn_str.len() - 138)];
        // debug_println!(
        //     "Raw txn: {}\n(Estimated) data input:{}\nTxn hash: {}",
        //     slice_to_hex_string(&signed_txn.raw_transaction.0),
        //     estimated_input_str,
        //     slice_to_hex_string(&signed_txn.transaction_hash.0),
        // );
    }

    #[test]
    fn assets_reserve_transfer_to_address32() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let (src_token, amount, dest_chain, dest_addr, kap_privkey, nonce) =
            get_args_dot_to_polkadot();
        let _signed_txn = get_contract()
            .assets_reserve_transfer(
                &src_token,
                amount,
                dest_chain,
                dest_addr,
                &kap_privkey,
                nonce,
            )
            .expect("Create txn failed");

        // let estimated_input_str = &raw_txn_str[78..(raw_txn_str.len() - 138)];
        // debug_println!(
        //     "Raw txn: {}\n(Estimated) data input:{}\nTxn hash: {}",
        //     slice_to_hex_string(&signed_txn.raw_transaction.0),
        //     estimated_input_str,
        //     slice_to_hex_string(&signed_txn.transaction_hash.0),
        // );
    }
}
