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

use ink_prelude::{vec, vec::Vec};
use scale::{Decode, Encode};

use privadex_chain_metadata::bridge::split_into_dest_and_beneficiary;

#[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ExtrinsicCallFactoryError {
    FailedToSplitFullDestMultiLocation,
}
type Result<T> = core::result::Result<T, ExtrinsicCallFactoryError>;

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct UnsignedExtrinsic<Call> {
    pallet_id: u8,
    call_id: u8,
    call: Call,
}

// GENERAL NOTE: The extrinsic formats do get changed e.g. weigh_limit changed
// from a raw u64 to WeightLimit in late 2022 in an upgrade.
// I need a way to monitor these breaking changes and update the encoding accordingly.

pub fn moonbeam_xtokens_transfer_multiasset(
    asset: xcm::prelude::MultiAsset,
    full_dest: xcm::prelude::MultiLocation,
) -> Result<Vec<u8>> {
    #[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    struct XTokensTransferMultiassetCall {
        asset: xcm::prelude::VersionedMultiAsset,
        dest: xcm::prelude::VersionedMultiLocation,
        dest_weight_limit: xcm::prelude::WeightLimit,
    }

    let raw_call_data = UnsignedExtrinsic {
        pallet_id: 0x6a,
        call_id: 0x01,
        call: XTokensTransferMultiassetCall {
            asset: xcm::prelude::VersionedMultiAsset::from(asset),
            dest: xcm::prelude::VersionedMultiLocation::from(full_dest),
            dest_weight_limit: xcm::prelude::WeightLimit::Limited(10_000_000_000u64),
        },
    };

    Ok(raw_call_data.encode())
}

pub fn moonbase_alpha_xtokens_transfer_multiasset(
    asset: xcm::prelude::MultiAsset,
    full_dest: xcm::prelude::MultiLocation,
) -> Result<Vec<u8>> {
    #[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    struct XTokensTransferMultiassetCall {
        asset: xcm::prelude::VersionedMultiAsset,
        dest: xcm::prelude::VersionedMultiLocation,
        dest_weight_limit: xcm::prelude::WeightLimit,
    }

    let raw_call_data = UnsignedExtrinsic {
        pallet_id: 0x1e,
        call_id: 0x01,
        call: XTokensTransferMultiassetCall {
            asset: xcm::prelude::VersionedMultiAsset::from(asset),
            dest: xcm::prelude::VersionedMultiLocation::from(full_dest),
            dest_weight_limit: xcm::prelude::WeightLimit::Limited(10_000_000_000u64),
        },
    };

    Ok(raw_call_data.encode())
}

pub fn polkadot_xcm_limited_reserve_transfer_assets(
    asset: xcm::prelude::MultiAsset,
    full_dest: xcm::prelude::MultiLocation,
) -> Result<Vec<u8>> {
    #[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    struct XcmLimitedReserveTransferAssets {
        dest: xcm::prelude::VersionedMultiLocation,
        beneficiary: xcm::prelude::VersionedMultiLocation,
        assets: xcm::prelude::VersionedMultiAssets,
        fee_asset_item: u32,
        weight_limit: xcm::prelude::WeightLimit,
    }

    let (dest, beneficiary) = split_into_dest_and_beneficiary(full_dest)
        .map_err(|_| ExtrinsicCallFactoryError::FailedToSplitFullDestMultiLocation)?;

    let assets =
        xcm::prelude::VersionedMultiAssets::from(xcm::prelude::MultiAssets::from(vec![asset]));
    let fee_asset_item = 0u32;
    let weight_limit = xcm::prelude::WeightLimit::Limited(10_000_000_000u64);

    let raw_call_data = UnsignedExtrinsic {
        pallet_id: 0x63,
        call_id: 0x08,
        call: XcmLimitedReserveTransferAssets {
            dest: xcm::prelude::VersionedMultiLocation::from(dest),
            beneficiary: xcm::prelude::VersionedMultiLocation::from(beneficiary),
            assets,
            fee_asset_item,
            weight_limit,
        },
    };
    Ok(raw_call_data.encode())
}

#[cfg(test)]
mod extrinsic_call_factory_tests {
    use hex_literal::hex;
    use xcm::latest::{
        AssetId, Fungibility, Junction, Junctions, MultiAsset, MultiLocation, NetworkId,
    };

    use privadex_chain_metadata::{
        common::{EthAddress, SubstratePublicKey, UniversalAddress},
        registry::bridge::xcm_bridge_registry::XCM_BRIDGES,
    };
    #[allow(unused_imports)]
    use privadex_common::utils::general_utils::slice_to_hex_string;

    use super::*;

    #[test]
    fn test_moonbase_alpha_xtokens_dev_to_moonbase_beta() {
        let dest = hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A");
        let amount = 1_000_000_000_000_000;
        let alpha_dev_asset = MultiAsset {
            id: AssetId::Concrete(MultiLocation {
                parents: 0u8,
                interior: Junctions::X1(Junction::PalletInstance(3u8)),
            }),
            fun: Fungibility::from(amount),
        };
        // Moonbase Beta account
        let full_dest = MultiLocation {
            parents: 1u8,
            interior: Junctions::X2(
                Junction::Parachain(888),
                Junction::AccountKey20 {
                    network: NetworkId::Any,
                    key: dest,
                },
            ),
        };

        let extrinsic_data = moonbase_alpha_xtokens_transfer_multiasset(alpha_dev_asset, full_dest)
            .expect("Valid extrinsic");
        // ink_env::debug_println!("Data: {:?}", slice_to_hex_string(&extrinsic_data));
        // https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fwss.api.moonbase.moonbeam.network#/extrinsics/decode/0x1e01010000010403000f0080c6a47e8d0301010200e10d030005a81d8564a3ea298660e34e03e5eff9a29d7a2a0102286bee
        let expected_extrinsic_data = hex!("1e01010000010403000f0080c6a47e8d0301010200e10d030005a81d8564a3ea298660e34e03e5eff9a29d7a2a010700e40b5402").to_vec();
        assert_eq!(extrinsic_data, expected_extrinsic_data);
    }

    #[test]
    fn test_moonbeam_xtokens_astr_to_astar() {
        // https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fwss.api.moonbeam.network#/explorer/query/2531796
        let xcm_bridge = &XCM_BRIDGES[1];
        let dest = UniversalAddress::Substrate(SubstratePublicKey {
            0: hex!("5134c7f0e31c2a9e19dceddb7403b2836c69cce0b0719d2f58ec0d4da35129be"),
        });
        let amount = 200_000_000_000_000_000;
        let astr_moonbeam_asset = MultiAsset {
            id: AssetId::Concrete(xcm_bridge.token_asset_multilocation.clone()),
            fun: Fungibility::from(amount),
        };
        let full_dest = xcm_bridge
            .dest_multilocation_template
            .get_full_dest_multilocation(dest)
            .expect("Valid dest MultiLocation");

        let extrinsic_data = moonbeam_xtokens_transfer_multiasset(astr_moonbeam_asset, full_dest)
            .expect("Valid extrinsic");
        // ink_env::debug_println!("Data: {:?}", slice_to_hex_string(&extrinsic_data));
        // https://polkadot.js.org/apps/?rpc=wss%3A%2F%2F1rpc.io%2Fglmr#/extrinsics/decode/0x6a010100010100591f0013000014bbf08ac60201010200591f01005134c7f0e31c2a9e19dceddb7403b2836c69cce0b0719d2f58ec0d4da35129be0102286bee
        let expected_extrinsic_data = hex!("6a010100010100591f0013000014bbf08ac60201010200591f01005134c7f0e31c2a9e19dceddb7403b2836c69cce0b0719d2f58ec0d4da35129be010700e40b5402").to_vec();
        assert_eq!(extrinsic_data, expected_extrinsic_data);
    }

    #[test]
    fn test_moonbeam_xtokens_dot_to_polkadot() {
        // https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fwss.api.moonbeam.network#/explorer/query/2518311
        let xcm_bridge = &XCM_BRIDGES[7];
        let dest = UniversalAddress::Substrate(SubstratePublicKey {
            0: hex!("60b94741c7094ac2820cceebeb24720af9e1049d7d4cb215f5080fbf5bdcbd4a"),
        });
        let amount = 40_000_000_000;
        let dot_moonbeam_asset = MultiAsset {
            id: AssetId::Concrete(xcm_bridge.token_asset_multilocation.clone()),
            fun: Fungibility::from(amount),
        };
        let full_dest = xcm_bridge
            .dest_multilocation_template
            .get_full_dest_multilocation(dest)
            .expect("Valid dest MultiLocation");

        let extrinsic_data = moonbeam_xtokens_transfer_multiasset(dot_moonbeam_asset, full_dest)
            .expect("Valid extrinsic");
        // ink_env::debug_println!("Data: {:?}", slice_to_hex_string(&extrinsic_data));
        // https://polkadot.js.org/apps/?rpc=wss%3A%2F%2F1rpc.io%2Fglmr#/extrinsics/decode/0x6a0101000100000700902f5009010101010060b94741c7094ac2820cceebeb24720af9e1049d7d4cb215f5080fbf5bdcbd4a0102286bee
        let expected_extrinsic_data = hex!("6a0101000100000700902f5009010101010060b94741c7094ac2820cceebeb24720af9e1049d7d4cb215f5080fbf5bdcbd4a010700e40b5402").to_vec();
        assert_eq!(extrinsic_data, expected_extrinsic_data);
    }

    #[test]
    fn test_moonbeam_xtokens_glmr_to_astar() {
        // https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fwss.api.moonbeam.network#/explorer/query/0x05a095d62c54ccee3c643914b41541503fe626591a6bf2ba3e559fc0ffdb6219
        let xcm_bridge = &XCM_BRIDGES[2];
        let dest = UniversalAddress::Substrate(SubstratePublicKey {
            0: hex!("5134c7f0e31c2a9e19dceddb7403b2836c69cce0b0719d2f58ec0d4da35129be"),
        });
        let amount = 100_000_000_000_000_000;
        let glmr_native_asset = MultiAsset {
            id: AssetId::Concrete(xcm_bridge.token_asset_multilocation.clone()),
            fun: Fungibility::from(amount),
        };
        let full_dest = xcm_bridge
            .dest_multilocation_template
            .get_full_dest_multilocation(dest)
            .expect("Valid dest MultiLocation");

        let extrinsic_data = moonbeam_xtokens_transfer_multiasset(glmr_native_asset, full_dest)
            .expect("Valid extrinsic");
        // ink_env::debug_println!("Data: {:?}", slice_to_hex_string(&extrinsic_data));
        // https://polkadot.js.org/apps/?rpc=wss%3A%2F%2F1rpc.io%2Fglmr#/extrinsics/decode/0x6a0101000001040a001300008a5d7845630101010200591f01005134c7f0e31c2a9e19dceddb7403b2836c69cce0b0719d2f58ec0d4da35129be0102286bee
        let expected_extrinsic_data = hex!("6a0101000001040a001300008a5d7845630101010200591f01005134c7f0e31c2a9e19dceddb7403b2836c69cce0b0719d2f58ec0d4da35129be010700e40b5402").to_vec();
        assert_eq!(extrinsic_data, expected_extrinsic_data);
    }

    #[test]
    fn test_polkadot_xcm_transfer_dot_to_astar() {
        // https://polkadot.js.org/apps/?rpc=wss%3A%2F%2F1rpc.io%2Fdot#/explorer/query/0xe278ebca27591a4303f1cc331c6bf0ad63accb0d5965b7259ca98f4dd954e32d
        let xcm_bridge = &XCM_BRIDGES[4];
        let dest = UniversalAddress::Substrate(SubstratePublicKey {
            0: hex!("5134c7f0e31c2a9e19dceddb7403b2836c69cce0b0719d2f58ec0d4da35129be"),
        });
        let amount = 1_000_000_000;
        let dot_native_asset = MultiAsset {
            id: AssetId::Concrete(xcm_bridge.token_asset_multilocation.clone()),
            fun: Fungibility::from(amount),
        };
        let full_dest = xcm_bridge
            .dest_multilocation_template
            .get_full_dest_multilocation(dest)
            .expect("Valid dest MultiLocation");

        let extrinsic_data =
            polkadot_xcm_limited_reserve_transfer_assets(dot_native_asset, full_dest)
                .expect("Valid extrinsic");
        // ink_env::debug_println!("Data: {:?}", slice_to_hex_string(&extrinsic_data));
        // https://polkadot.js.org/apps/?rpc=wss%3A%2F%2F1rpc.io%2Fdot#/extrinsics/decode/0x630801000100591f01000101005134c7f0e31c2a9e19dceddb7403b2836c69cce0b0719d2f58ec0d4da35129be01040000000002286bee000000000102286bee
        let expected_extrinsic_data = hex!("630801000100591f01000101005134c7f0e31c2a9e19dceddb7403b2836c69cce0b0719d2f58ec0d4da35129be01040000000002286bee00000000010700e40b5402").to_vec();
        assert_eq!(extrinsic_data, expected_extrinsic_data);
    }

    #[test]
    fn test_polkadot_xcm_transfer_dot_to_moonbeam() {
        let xcm_bridge = &XCM_BRIDGES[6];
        let dest = UniversalAddress::Ethereum(EthAddress {
            0: hex!("05a81d8564a3eA298660e34e03E5Eff9a29d7a2A"),
        });
        let amount = 1_000_000_000;
        let dot_native_asset = MultiAsset {
            id: AssetId::Concrete(xcm_bridge.token_asset_multilocation.clone()),
            fun: Fungibility::from(amount),
        };
        let full_dest = xcm_bridge
            .dest_multilocation_template
            .get_full_dest_multilocation(dest)
            .expect("Valid dest MultiLocation");

        let extrinsic_data =
            polkadot_xcm_limited_reserve_transfer_assets(dot_native_asset, full_dest)
                .expect("Valid extrinsic");
        // ink_env::debug_println!("Data: {:?}", slice_to_hex_string(&extrinsic_data));
        // https://polkadot.js.org/apps/?rpc=wss%3A%2F%2F1rpc.io%2Fdot#/extrinsics/decode/0x630801000100511f010001030005a81d8564a3ea298660e34e03e5eff9a29d7a2a01040000000002286bee000000000102286bee
        let expected_extrinsic_data = hex!("630801000100511f010001030005a81d8564a3ea298660e34e03e5eff9a29d7a2a01040000000002286bee00000000010700e40b5402").to_vec();
        assert_eq!(extrinsic_data, expected_extrinsic_data);
    }
}
