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

pub mod xcm_bridge_registry {
    use crate::bridge::{get_dest_multilocation_template, XCMBridge};
    use crate::registry::{
        chain::chain_info_registry, token::token_multilocation_spec_registry as token_spec_reg,
    };

    // NOTE: We cannot submit Substrate extrinsics from an Astar EVM account,
    // so we will use the XCM precompile (function assets_reserve_transfer)
    // (https://docs.astar.network/docs/EVM/precompiles/xcm). This means we won't use
    // the MultiLocations but the remaining information is sufficient to build the function call

    // This is a large array, so I don't want it in-lined. Hence I 'static' and not 'const'
    // DO NOT REORDER the bridges below because unit tests depend on the ordering
    pub static XCM_BRIDGES: [XCMBridge; 8] = [
        XCMBridge {
            src_token: token_spec_reg::ASTR_NATIVE.token,
            dest_token: token_spec_reg::ASTR_MOONBEAM.token,
            token_asset_multilocation: token_spec_reg::ASTR_NATIVE.token_asset_multilocation,
            dest_multilocation_template: get_dest_multilocation_template(
                &chain_info_registry::ASTAR_INFO,
                &chain_info_registry::MOONBEAM_INFO,
            ),
            estimated_bridge_fee_in_dest_chain_native_token: chain_info_registry::MOONBEAM_INFO
                .avg_bridge_fee_in_native_token,
        },
        XCMBridge {
            src_token: token_spec_reg::ASTR_MOONBEAM.token,
            dest_token: token_spec_reg::ASTR_NATIVE.token,
            token_asset_multilocation: token_spec_reg::ASTR_MOONBEAM.token_asset_multilocation,
            dest_multilocation_template: get_dest_multilocation_template(
                &chain_info_registry::MOONBEAM_INFO,
                &chain_info_registry::ASTAR_INFO,
            ),
            estimated_bridge_fee_in_dest_chain_native_token: chain_info_registry::ASTAR_INFO
                .avg_bridge_fee_in_native_token,
        },
        XCMBridge {
            src_token: token_spec_reg::GLMR_NATIVE.token,
            dest_token: token_spec_reg::GLMR_ASTAR.token,
            token_asset_multilocation: token_spec_reg::GLMR_NATIVE.token_asset_multilocation,
            dest_multilocation_template: get_dest_multilocation_template(
                &chain_info_registry::MOONBEAM_INFO,
                &chain_info_registry::ASTAR_INFO,
            ),
            estimated_bridge_fee_in_dest_chain_native_token: chain_info_registry::ASTAR_INFO
                .avg_bridge_fee_in_native_token,
        },
        XCMBridge {
            src_token: token_spec_reg::GLMR_ASTAR.token,
            dest_token: token_spec_reg::GLMR_NATIVE.token,
            token_asset_multilocation: token_spec_reg::GLMR_ASTAR.token_asset_multilocation,
            dest_multilocation_template: get_dest_multilocation_template(
                &chain_info_registry::ASTAR_INFO,
                &chain_info_registry::MOONBEAM_INFO,
            ),
            estimated_bridge_fee_in_dest_chain_native_token: chain_info_registry::MOONBEAM_INFO
                .avg_bridge_fee_in_native_token,
        },
        XCMBridge {
            src_token: token_spec_reg::DOT_NATIVE.token,
            dest_token: token_spec_reg::DOT_ASTAR.token,
            token_asset_multilocation: token_spec_reg::DOT_NATIVE.token_asset_multilocation,
            dest_multilocation_template: get_dest_multilocation_template(
                &chain_info_registry::POLKADOT_INFO,
                &chain_info_registry::ASTAR_INFO,
            ),
            estimated_bridge_fee_in_dest_chain_native_token: chain_info_registry::ASTAR_INFO
                .avg_bridge_fee_in_native_token,
        },
        XCMBridge {
            src_token: token_spec_reg::DOT_ASTAR.token,
            dest_token: token_spec_reg::DOT_NATIVE.token,
            token_asset_multilocation: token_spec_reg::DOT_ASTAR.token_asset_multilocation,
            dest_multilocation_template: get_dest_multilocation_template(
                &chain_info_registry::ASTAR_INFO,
                &chain_info_registry::POLKADOT_INFO,
            ),
            estimated_bridge_fee_in_dest_chain_native_token: chain_info_registry::POLKADOT_INFO
                .avg_bridge_fee_in_native_token,
        },
        XCMBridge {
            src_token: token_spec_reg::DOT_NATIVE.token,
            dest_token: token_spec_reg::DOT_MOONBEAM.token,
            token_asset_multilocation: token_spec_reg::DOT_NATIVE.token_asset_multilocation,
            dest_multilocation_template: get_dest_multilocation_template(
                &chain_info_registry::POLKADOT_INFO,
                &chain_info_registry::MOONBEAM_INFO,
            ),
            estimated_bridge_fee_in_dest_chain_native_token: chain_info_registry::MOONBEAM_INFO
                .avg_bridge_fee_in_native_token,
        },
        XCMBridge {
            src_token: token_spec_reg::DOT_MOONBEAM.token,
            dest_token: token_spec_reg::DOT_NATIVE.token,
            token_asset_multilocation: token_spec_reg::DOT_MOONBEAM.token_asset_multilocation,
            dest_multilocation_template: get_dest_multilocation_template(
                &chain_info_registry::MOONBEAM_INFO,
                &chain_info_registry::POLKADOT_INFO,
            ),
            estimated_bridge_fee_in_dest_chain_native_token: chain_info_registry::POLKADOT_INFO
                .avg_bridge_fee_in_native_token,
        },
    ];
}
