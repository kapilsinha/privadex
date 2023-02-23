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

#[allow(unused_imports)]
use pink_web3::types::{Bytes, Transaction, TransactionId, TransactionReceipt, U256};
#[allow(unused_imports)]
use privadex_chain_metadata::common::{Amount, EthAddress, EthTxnHash};

use super::{common, erc20_contract::ERC20Contract};

/// Parse information out of transfer transactions
#[cfg(not(feature = "mock-txn-send"))]
pub fn parse_transfer_from_eth_send_txn(
    rpc_url: &str,
    eth_send_txn: EthTxnHash,
) -> common::Result<common::EthTransfer> {
    let receipt = get_txn_receipt(rpc_url, eth_send_txn)?;
    // Need to make a second RPC call to pull the amount transferred
    let txn = get_txn(rpc_url, eth_send_txn)?;
    let is_txn_success = receipt.status == Some(1.into());
    let amount = common::u256_to_u128(txn.value)?;
    let gas_fee_native = get_gas_fee_native(&receipt)?;
    let to = receipt.to.ok_or(common::EthError::ParseFailed)?;
    Ok(common::EthTransfer {
        is_txn_success,
        from: receipt.from,
        to,
        amount,
        gas_fee_native,
    })
}
#[cfg(feature = "mock-txn-send")]
pub fn parse_transfer_from_eth_send_txn(
    rpc_url: &str,
    eth_send_txn: EthTxnHash,
) -> common::Result<common::EthTransfer> {
    ink_env::debug_println!("[Mock Eth parse_transfer_from_eth_send_txn]");
    Ok(common::EthTransfer {
        is_txn_success: true,
        from: EthAddress::zero(),
        to: EthAddress::zero(),
        amount: 1_000_000_000,
        gas_fee_native: 2_000_000_000,
    })
}

#[cfg(not(feature = "mock-txn-send"))]
pub fn parse_transfer_from_erc20_txn(
    rpc_url: &str,
    erc20_txn_hash: EthTxnHash,
) -> common::Result<common::ERC20Transfer> {
    let receipt = get_txn_receipt(rpc_url, erc20_txn_hash)?;
    if receipt.logs.len() != 1 {
        return Err(common::EthError::ParseFailed);
    }
    let is_txn_success = receipt.status == Some(1.into());
    let gas_fee_native = get_gas_fee_native(&receipt)?;
    let transfer_log =
        ERC20Contract::parse_transfer_log(&receipt.logs[0], is_txn_success, gas_fee_native)?;
    // We expect that transfer_log.from == receipt.from
    Ok(transfer_log)
}
#[cfg(feature = "mock-txn-send")]
pub fn parse_transfer_from_erc20_txn(
    rpc_url: &str,
    erc20_txn_hash: EthTxnHash,
) -> common::Result<common::ERC20Transfer> {
    ink_env::debug_println!("[Mock Eth parse_transfer_from_erc20_txn]");
    Ok(common::ERC20Transfer {
        is_txn_success: true,
        token: EthAddress::zero(),
        from: EthAddress::zero(),
        to: EthAddress::zero(),
        amount: 1_000_000_000,
        gas_fee_native: 2_000_000_000,
    })
}

#[cfg(not(feature = "mock-txn-send"))]
pub fn parse_transfer_from_dex_swap_txn(
    rpc_url: &str,
    dex_swap_txn_hash: EthTxnHash,
) -> common::Result<common::ERC20Transfer> {
    // Returns the final transfer in a series of swaps
    let receipt = get_txn_receipt(rpc_url, dex_swap_txn_hash)?;
    let is_txn_success = receipt.status == Some(1.into());
    let gas_fee_native = get_gas_fee_native(&receipt)?;
    for i in (0..receipt.logs.len()).rev() {
        let transfer_log =
            ERC20Contract::parse_transfer_log(&receipt.logs[i], is_txn_success, gas_fee_native);
        if let Ok(mut log) = transfer_log {
            // The from address on the TransferLog is the åddress of a TokenPair contract.
            // We replace this with the address that initiated the transaction
            log.from = receipt.from;
            return Ok(log);
        }
    }
    // If the transaction fails, there likely will be no logs at all. So we populate dummy
    // values instead of failing to parse (which occurs repeatedly until the txn gets wrongly
    // dropped)
    // TODO: Write a unit test that covers this failed case:
    // https://moonscan.io/tx/0xcdf9aadfc4ddbc2c3238c16129b29bf3d156c4d4eec4edbe446f31304586d5e4
    Ok(common::ERC20Transfer {
        is_txn_success,
        token: EthAddress::zero(),
        from: EthAddress::zero(),
        to: EthAddress::zero(),
        amount: 0,
        gas_fee_native,
    })
}
#[cfg(feature = "mock-txn-send")]
pub fn parse_transfer_from_dex_swap_txn(
    rpc_url: &str,
    dex_swap_txn_hash: EthTxnHash,
) -> common::Result<common::ERC20Transfer> {
    ink_env::debug_println!("[Mock Eth parse_transfer_from_dex_swap_txn]");
    Ok(common::ERC20Transfer {
        is_txn_success: true,
        token: EthAddress::zero(),
        from: EthAddress::zero(),
        to: EthAddress::zero(),
        amount: 1_000_000_000,
        gas_fee_native: 2_000_000_000,
    })
}

#[cfg(not(feature = "mock-txn-send"))]
pub fn get_txn_summary(rpc_url: &str, txn_hash: EthTxnHash) -> common::Result<common::TxnSummary> {
    let receipt = get_txn_receipt(rpc_url, txn_hash)?;
    let is_txn_success = receipt.status == Some(1.into());
    let gas_fee_native = get_gas_fee_native(&receipt)?;
    Ok(common::TxnSummary {
        is_txn_success,
        gas_fee_native,
    })
}
#[cfg(feature = "mock-txn-send")]
pub fn get_txn_summary(rpc_url: &str, txn_hash: EthTxnHash) -> common::Result<common::TxnSummary> {
    ink_env::debug_println!("[Mock Eth get_txn_summary]");
    // let is_txn_success = unsafe {
    //     static mut x: u32 = 0;
    //     if x < 1 {
    //         x += 1;
    //         true
    //     } else { false }
    // };
    let is_txn_success = true;
    Ok(common::TxnSummary {
        is_txn_success,
        gas_fee_native: 2_000_000_000,
    })
}

fn get_gas_fee_native(receipt: &TransactionReceipt) -> common::Result<Amount> {
    let gas_price_u256 = receipt
        .effective_gas_price
        .ok_or(common::EthError::ParseFailed)?;
    let gas_used_u256 = receipt.gas_used.ok_or(common::EthError::ParseFailed)?;
    let gas_price = common::u256_to_u128(gas_price_u256)?;
    let gas_used = common::u256_to_u128(gas_used_u256)?;
    Ok(gas_price.checked_mul(gas_used).unwrap_or_default())
}

fn get_txn_receipt(rpc_url: &str, txn_hash: EthTxnHash) -> common::Result<TransactionReceipt> {
    common::eth(rpc_url)
        .transaction_receipt(txn_hash)
        .resolve()
        .map_err(|_| common::EthError::TransactionNotFound)?
        .ok_or(common::EthError::TransactionNotFound)
}

fn get_txn(rpc_url: &str, txn_hash: EthTxnHash) -> common::Result<Transaction> {
    common::eth(rpc_url)
        .transaction(TransactionId::Hash(txn_hash))
        .resolve()
        .map_err(|_| common::EthError::TransactionNotFound)?
        .ok_or(common::EthError::TransactionNotFound)
}

#[cfg(test)]
mod parse_txn_tests {
    use hex_literal::hex;
    use privadex_chain_metadata::{common::EthAddress, registry::chain::chain_info_registry};

    use super::*;

    #[test]
    fn test_parse_eth_send_txn() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let rpc_url = chain_info_registry::MOONBASEALPHA_INFO.rpc_url;
        let eth_transfer = parse_transfer_from_eth_send_txn(
            rpc_url,
            EthTxnHash {
                0: hex!("8db02e3c51f23a7f4540ba127490ddcbe7a8ad39bcd2f7d5e57b5dc8d6b891e6"),
            },
        )
        .expect("Found ETH transfer");
        assert_eq!(eth_transfer.is_txn_success, true);
        assert_eq!(
            eth_transfer.from,
            EthAddress {
                0: hex!("05a81d8564a3ea298660e34e03e5eff9a29d7a2a")
            }
        );
        assert_eq!(
            eth_transfer.to,
            EthAddress {
                0: hex!("05a81d8564a3ea298660e34e03e5eff9a29d7a2a")
            }
        );
        assert_eq!(eth_transfer.amount, 1_000_000_000_000_000);
        assert_eq!(eth_transfer.gas_fee_native, 21_000_000_000_000);
    }

    #[test]
    fn test_parse_erc20_transfer() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let txn_hash = EthTxnHash {
            0: hex!("ccedfd63a7f3e2c98e33ea1eebb4bf76ade3b607c8800e5cbbe6557a01549d61"),
        };
        let rpc_url = chain_info_registry::MOONBEAM_INFO.rpc_url;
        let erc20_transfer =
            parse_transfer_from_erc20_txn(&rpc_url, txn_hash).expect("Parse failed");
        assert_eq!(
            erc20_transfer.token,
            EthAddress {
                0: hex!("ffffffff1fcacbd218edc0eba20fc2308c778080")
            }
        );
        assert_eq!(
            erc20_transfer.from,
            EthAddress {
                0: hex!("fe86fc0cca9f7bb38e18322475c29f0cddb5104e")
            }
        );
        assert_eq!(
            erc20_transfer.to,
            EthAddress {
                0: hex!("e065662bf49f036756f5170edcd5cfca0a56f9a2")
            }
        );
        assert_eq!(erc20_transfer.amount, 4_000_000_000);
        assert_eq!(erc20_transfer.gas_fee_native, 4_592_672_000_000_000);
    }

    #[test]
    fn test_parse_erc20_not_transfer() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let txn_hash = EthTxnHash {
            0: hex!("d9ff564a3b27e41a9c59eabbec5f5564c3bf1c0bba9e54c595c3e916082ff3a8"),
        };
        let rpc_url = chain_info_registry::MOONBEAM_INFO.rpc_url;
        let err = parse_transfer_from_erc20_txn(&rpc_url, txn_hash)
            .expect_err("Transaction is not a transfer");
        assert_eq!(err, common::EthError::ParseFailed);
    }

    #[test]
    fn test_parse_erc20_invalid_txn() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let txn_hash = EthTxnHash {
            0: hex!("a9ff564a3b27e41a9c59eabbec5f5564c3bf1c0bba9e54c595c3e916082ff3a8"),
        };
        let rpc_url = chain_info_registry::MOONBEAM_INFO.rpc_url;
        let err = parse_transfer_from_erc20_txn(&rpc_url, txn_hash)
            .expect_err("Transaction is not a transfer");
        assert_eq!(err, common::EthError::TransactionNotFound);
    }

    #[test]
    fn test_parse_dex_swap_transfer() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let txn_hash = EthTxnHash {
            0: hex!("0c5106f1c50362be4bf51dae49616ac6686d6dd2875b99457abbf1d32ac3bbe1"),
        };
        let rpc_url = chain_info_registry::MOONBEAM_INFO.rpc_url;
        let dex_swap_transfer =
            parse_transfer_from_dex_swap_txn(rpc_url, txn_hash).expect("Parse failed");
        assert_eq!(
            dex_swap_transfer.token,
            EthAddress {
                0: hex!("9D5d41D8C03e38194A577347206F8829B9cF7C9a")
            }
        );
        assert_eq!(
            dex_swap_transfer.from,
            EthAddress {
                0: hex!("05a81d8564a3ea298660e34e03e5eff9a29d7a2a")
            }
        );
        assert_eq!(
            dex_swap_transfer.to,
            EthAddress {
                0: hex!("05a81d8564a3ea298660e34e03e5eff9a29d7a2a")
            }
        );
        assert_eq!(dex_swap_transfer.amount, 33_033_115_877_566);
        assert_eq!(dex_swap_transfer.gas_fee_native, 21_770_126_000_000_000);
    }
}
