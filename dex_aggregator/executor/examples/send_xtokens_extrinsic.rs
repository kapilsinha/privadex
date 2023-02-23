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

use core::str::FromStr;
use hex_literal::hex;
use ink_env::debug_println;
use ink_prelude::{string::ToString, vec::Vec};
use sp_runtime::generic::Era;

use privadex_chain_metadata::common::SecretKeyContainer;
use privadex_common::{
    signature_scheme::SignatureScheme, utils::general_utils::slice_to_hex_string,
};
use privadex_executor::{
    extrinsic_call_factory,
    substrate_utils::{
        extrinsic_sig_config::ExtrinsicSigConfig, node_rpc_utils::SubstrateNodeRpcUtils,
    },
};

fn main() {
    pink_extension_runtime::mock_ext::mock_all_ext();

    let chain_utils = SubstrateNodeRpcUtils {
        rpc_url: "https://moonbeam-alpha.api.onfinality.io/public".to_string(),
    };
    let sender = hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A");

    let kap_privkey = {
        let privkey_str =
            std::env::var("ETH_PRIVATE_KEY").expect("Env var ETH_PRIVATE_KEY is not set");
        SecretKeyContainer::from_str(&privkey_str)
            .expect("ETH_PRIVATE_KEY to_hex failed")
            .0
    };
    let sigconfig = ExtrinsicSigConfig::<[u8; 20]> {
        sig_scheme: SignatureScheme::Ethereum,
        signer: sender,
        privkey: kap_privkey.to_vec(),
    };

    let nonce = chain_utils
        .get_next_system_nonce(&slice_to_hex_string(&sender))
        .expect("Expected valid nonce");
    debug_println!("nonce: {:?}", nonce);
    let runtime_version = chain_utils
        .get_runtime_version()
        .expect("Expected valid runtime version");
    debug_println!("runtime_version: {:?}", runtime_version);
    let genesis_hash = chain_utils
        .get_genesis_hash()
        .expect("Expected valid genesis hash");
    debug_println!("genesis_hash: {:?}", genesis_hash);
    let era = Era::Immortal;
    let finalized_head = if era != Era::Immortal {
        chain_utils
            .get_finalized_head_hash()
            .expect("Expected valid finalized head hash")
    } else {
        genesis_hash.clone()
    };
    debug_println!("finalized_head_hash: {:?}", finalized_head);

    let encoded_call_data = moonbase_alpha_xtokens_transfer_multiasset_demo(
        sender,                    /* dest */
        1_000_000_000_000_000u128, /* amount */
    );
    let tx_raw = chain_utils.create_extrinsic::<[u8; 20]>(
        sigconfig,
        &encoded_call_data,
        nonce,
        runtime_version,
        genesis_hash,
        finalized_head, // checkpoint block hash
        era,
        0, // tip
    );
    debug_println!("Raw txn: {:?}", slice_to_hex_string(&tx_raw));

    // Commented out to avoid actually sending out transactions
    // let send_response = chain_utils.send_extrinsic(&tx_raw);
    // debug_println!("Sent txn: {:?}", send_response);
}

// https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fwss.api.moonbase.moonbeam.network#/extrinsics/decode/0x1e01010000010403000f0080c6a47e8d0301010200e10d030005a81d8564a3ea298660e34e03e5eff9a29d7a2a0102286bee
// Hard-codes DEV token and destination = Moonbase Beta
fn moonbase_alpha_xtokens_transfer_multiasset_demo(dest: [u8; 20], amount: u128) -> Vec<u8> {
    use xcm::latest::{
        AssetId, Fungibility, Junction, Junctions, MultiAsset, MultiLocation, NetworkId,
    };

    let alpha_dev_asset = MultiAsset {
        id: AssetId::Concrete(MultiLocation {
            parents: 0u8,
            interior: Junctions::X1(Junction::PalletInstance(3u8)),
        }),
        fun: Fungibility::from(amount),
    };
    let dest_location = MultiLocation {
        parents: 1u8,
        interior: Junctions::X2(
            Junction::Parachain(888),
            Junction::AccountKey20 {
                network: NetworkId::Any,
                key: dest,
            },
        ),
    };

    extrinsic_call_factory::moonbase_alpha_xtokens_transfer_multiasset(
        alpha_dev_asset,
        dest_location,
    )
    .expect("Valid extrinsic")
}
