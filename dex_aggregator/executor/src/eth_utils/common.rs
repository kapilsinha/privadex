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

use ink_prelude::vec::Vec;
use pink_web3::{
    api::{Accounts, Eth, Namespace},
    contract::{tokens::Tokenize, Contract, Options},
    ethabi::Function,
    keys::pink::KeyPair,
    signing::Key,
    transports::{resolve_ready, PinkHttp},
    types::{Bytes, CallRequest, SignedTransaction, TransactionParameters, U256},
};

use privadex_chain_metadata::common::{Amount, BlockNum, EthAddress, EthTxnHash, Nonce, SecretKey};
use privadex_common::utils::general_utils::mul_ratio_u128;

#[derive(Debug, PartialEq)]
pub enum EthError {
    // We stick with Substrate's u128 definition of Amount for compatibility/safety,
    // so if any internal U256 is greater than u128::MAX, we return this error
    AmountTooHigh,
    BadSignature,
    BlockNumberRequestFailed,
    CreateRawTransactionFailed,
    ContractCallFailed,
    FunctionNotFound,
    GasEstimateFailed,
    InvalidABI,
    InvalidArgument,
    NonceRequestFailed,
    ParseFailed,
    SendTransactionFailed,
    SignTransactionFailed,
    TransactionNotFound,
    // pink_web3 can technically accept nonce = None because it computes the
    // account nonce in that occasion. But we need to track nonce and pass it in
    // manually. So we error if it is unspecified
    UnspecifiedNonce,
}
pub type Result<T> = core::result::Result<T, EthError>;

#[derive(Debug)]
pub struct EthTransfer {
    pub is_txn_success: bool,
    pub from: EthAddress,
    pub to: EthAddress,
    pub amount: Amount,
    pub gas_fee_native: Amount,
}

#[derive(Debug)]
pub struct ERC20Transfer {
    pub is_txn_success: bool,
    pub token: EthAddress,
    pub from: EthAddress,
    pub to: EthAddress,
    pub amount: Amount,
    pub gas_fee_native: Amount,
}

#[derive(Debug)]
pub struct TxnSummary {
    pub is_txn_success: bool,
    pub gas_fee_native: Amount,
}

pub trait ContractWrapper {
    fn get_rpc_url(&self) -> &str;

    fn send_raw_transaction(&self, signed: SignedTransaction) -> Result<EthTxnHash> {
        send_raw_transaction(self.get_rpc_url(), signed)
    }
}

#[cfg(not(feature = "mock-txn-send"))]
pub fn send_raw_transaction(rpc_url: &str, signed: SignedTransaction) -> Result<EthTxnHash> {
    eth(rpc_url)
        .send_raw_transaction(signed.raw_transaction)
        .resolve()
        .map_err(|_| EthError::SendTransactionFailed)
}

#[cfg(feature = "mock-txn-send")]
pub fn send_raw_transaction(_rpc_url: &str, signed: SignedTransaction) -> Result<EthTxnHash> {
    ink_env::debug_println!("[Mock Eth send_raw_transaction]");
    Ok(signed.transaction_hash)
}

pub fn create_send_eth_raw_txn<'a, 'b>(
    rpc_url: &str,
    to: EthAddress,
    amount: Amount,
    key: &SecretKey,
    chain_id: u64,
    nonce: Nonce,
) -> Result<SignedTransaction> {
    let txn_params = create_txn_params(to, amount, Bytes::from(Vec::new()), chain_id, nonce);
    create_raw_txn_from_txn_params(rpc_url, key, txn_params)
}

pub fn get_next_system_nonce(rpc_url: &str, address: EthAddress) -> Result<Nonce> {
    let nonce = eth(rpc_url)
        .transaction_count(address, None /* block number */)
        .resolve()
        .map_err(|_| EthError::NonceRequestFailed)?;
    if nonce > Nonce::MAX.into() {
        Err(EthError::AmountTooHigh)
    } else {
        Ok(nonce.low_u32())
    }
}

pub fn block_number(rpc_url: &str) -> Result<BlockNum> {
    let block_num = eth(rpc_url)
        .block_number()
        .resolve()
        .map_err(|_| EthError::BlockNumberRequestFailed)?;
    if block_num > BlockNum::MAX.into() {
        Err(EthError::AmountTooHigh)
    } else {
        Ok(block_num.low_u32())
    }
}

/// Creates the SignedTransaction but does NOT send it!
/// This is useful if we want to do something with the txn hash before submitting it
pub(super) fn create_raw_txn<ParamsType: Clone + Tokenize>(
    rpc_url: &str,
    contract: &Contract<PinkHttp>,
    func: &str,
    // We need to specify the index of the overload of this function to help look up
    // the correct function
    overload_index: u8,
    params: ParamsType,
    options_seed: Options,
    key: &SecretKey,
    nonce: Nonce,
) -> Result<SignedTransaction> {
    let fn_data = get_contract_func(contract, func, overload_index)?
        .encode_input(&params.into_tokens())
        .map_err(|_| EthError::CreateRawTransactionFailed)?;
    let keypair = KeyPair::from(key.clone());
    let mut options = {
        if options_seed.gas.is_some() {
            options_seed
        } else {
            estimate_gas(
                rpc_url,
                contract.address(),
                fn_data.clone(),
                keypair.address(),
                options_seed,
            )?
        }
    };
    options.nonce = Some(U256::from(nonce));

    contract_sign_txn(rpc_url, fn_data, contract.address(), options, keypair)
}

pub(super) fn eth(rpc_url: &str) -> Eth<PinkHttp> {
    Eth::new(PinkHttp::new(rpc_url.clone()))
}

pub(super) fn u256_to_u128(val: U256) -> Result<u128> {
    let low_u128 = val.low_u128();
    if val != U256::from(low_u128) {
        Err(EthError::AmountTooHigh)
    } else {
        Ok(low_u128)
    }
}

// Copied from pink-web3/src/contract/mod.rs because it is not made public there
// And wrapped in a resolve so we don't need to deal with asyncs
fn contract_sign_txn(
    rpc_url: &str,
    fn_data: Vec<u8>,
    contract_address: EthAddress,
    options: Options,
    key: KeyPair,
) -> Result<SignedTransaction> {
    let _ = validate_nonce(options.nonce)?;
    // The contract.abi().function() function takes just the first function of that name, which
    // ignores overloaded functions (same func name, different args). Thus we specify an overload index
    // to find the correct Function
    let mut tx = TransactionParameters {
        nonce: options.nonce,
        to: Some(contract_address),
        gas_price: options.gas_price,
        data: Bytes(fn_data),
        transaction_type: options.transaction_type,
        access_list: options.access_list,
        max_fee_per_gas: options.max_fee_per_gas,
        max_priority_fee_per_gas: options.max_priority_fee_per_gas,
        ..Default::default()
    };
    if let Some(gas) = options.gas {
        tx.gas = gas;
    }
    if let Some(value) = options.value {
        tx.value = value;
    }
    resolve_ready(accounts(rpc_url).sign_transaction(tx, key))
        .map_err(|_| EthError::SignTransactionFailed)
}

fn accounts(rpc_url: &str) -> Accounts<PinkHttp> {
    Accounts::new(PinkHttp::new(rpc_url.clone()))
}

/*
 * Creates the TransactionParameters for a legacy Ethereum transaction.
 * Note that Accounts::sign_transaction will later override None for the following params:
 * - `nonce`: the signing account's transaction count
 * - `gas_price`: estimated recommended gas price
 * https://docs.rs/pink-web3/latest/pink_web3/types/struct.TransactionParameters.html
 */
fn create_txn_params(
    to: EthAddress,
    value: Amount,
    data: Bytes,
    chain_id: u64,
    nonce: Nonce,
) -> TransactionParameters {
    let mut tx = TransactionParameters::default();
    tx.to = Some(to);
    tx.value = U256::from(value);
    tx.data = data;
    tx.chain_id = Some(chain_id);
    tx.nonce = Some(U256::from(nonce));
    tx
}

fn create_raw_txn_from_txn_params(
    rpc_url: &str,
    key: &SecretKey,
    txn_params: TransactionParameters,
) -> Result<SignedTransaction> {
    let _ = validate_nonce(txn_params.nonce)?;
    let keypair = KeyPair::from(key.clone());
    resolve_ready(accounts(rpc_url).sign_transaction(txn_params, keypair))
        .map_err(|_| EthError::BadSignature)
}

fn get_contract_func<'a, 'b>(
    contract: &'a Contract<PinkHttp>,
    func_name: &'b str,
    overload_index: u8,
) -> Result<&'a Function> {
    let functions_vec = contract
        .abi()
        .functions_by_name(func_name)
        .map_err(|_| EthError::FunctionNotFound)?;
    functions_vec
        .get(overload_index as usize)
        .ok_or(EthError::FunctionNotFound)
}

// The contract.estimate_gas(...) function finds the first function of that name
// and ignores overloads, so we implement our own here
fn estimate_gas(
    rpc_url: &str,
    contract_address: EthAddress,
    raw_data: Vec<u8>,
    from: EthAddress,
    options_seed: Options,
) -> Result<Options> {
    let mut options = options_seed.clone();
    let opt_gas = resolve_ready({
        eth(rpc_url).estimate_gas(
            CallRequest {
                from: Some(from),
                to: Some(contract_address),
                gas: options_seed.gas,
                gas_price: options_seed.gas_price,
                value: options_seed.value,
                data: Some(Bytes(raw_data)),
                transaction_type: options_seed.transaction_type,
                access_list: options_seed.access_list,
                max_fee_per_gas: options_seed.max_fee_per_gas,
                max_priority_fee_per_gas: options_seed.max_priority_fee_per_gas,
            },
            None,
        )
    });
    ink_env::debug_println!("Estimate gas: {:?}", opt_gas);
    let gas = opt_gas.map_err(|_| EthError::GasEstimateFailed)?;
    if let Ok(gas_u128) = u256_to_u128(gas) {
        // Add +100% to the gas limit since we sometimes (rarely) see run-out-of-gas errors e.g.
        // https://dashboard.tenderly.co/tx/moonbeam/0xcdf9aadfc4ddbc2c3238c16129b29bf3d156c4d4eec4edbe446f31304586d5e4
        // We are more than happy to pay more gas to (essentially) ensure no out-of-gas errors
        options.gas = Some(U256::from(mul_ratio_u128(gas_u128, 2, 1)));
    } else {
        options.gas = Some(gas);
    }
    Ok(options)
}

fn validate_nonce(nonce: Option<U256>) -> Result<()> {
    if nonce.is_none() {
        Err(EthError::UnspecifiedNonce)
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[allow(dead_code)]
pub(super) fn print_and_send_txn(rpc_url: &str, signed_txn: SignedTransaction) {
    ink_env::debug_println!("{:?}", signed_txn);
    let txn_hash = send_raw_transaction(rpc_url, signed_txn).expect("returned txn hash");
    ink_env::debug_println!("Txn hash: {:?}", txn_hash);
}

#[cfg(test)]
mod eth_utils_common_tests {
    use core::str::FromStr;
    use hex_literal::hex;
    use privadex_chain_metadata::{
        common::SecretKeyContainer, registry::chain::chain_info_registry,
    };

    use super::*;

    #[test]
    fn test_astar_nonce() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let address = EthAddress {
            0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
        };
        let nonce = get_next_system_nonce(&chain_info_registry::ASTAR_INFO.rpc_url, address)
            .expect("Expect nonce value");
        // ink_env::debug_println!("Nonce: {}", nonce);
        assert!(nonce > 1);
    }

    #[test]
    fn test_astar_block_num() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let block_num =
            block_number(&chain_info_registry::ASTAR_INFO.rpc_url).expect("Expect block num");
        assert!(block_num > 2_662_091);
    }

    #[test]
    fn test_send_eth_create_txn() {
        // Generated: https://moonbase.moonscan.io/tx/0x44b9890af58b0fce5d2b90dbc4b15cac78331d89b2ddf7185b5634097f94c6d4
        pink_extension_runtime::mock_ext::mock_all_ext();
        let kap_privkey = {
            let privkey_str =
                std::env::var("ETH_PRIVATE_KEY").expect("Env var ETH_PRIVATE_KEY is not set");
            SecretKeyContainer::from_str(&privkey_str)
                .expect("ETH_PRIVATE_KEY to_hex failed")
                .0
        };
        let chain_info = &chain_info_registry::MOONBASEALPHA_INFO;
        let nonce = 0;
        let _ = create_send_eth_raw_txn(
            chain_info.rpc_url,
            EthAddress {
                0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
            },
            1_000_000_000_000_000,
            &kap_privkey,
            chain_info.evm_chain_id.expect("EVM chain ID"),
            nonce,
        )
        .expect("Valid signed txn");

        // print_and_send_txn(chain_info.rpc_url, signed_txn);
    }
}
