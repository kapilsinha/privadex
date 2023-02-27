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

use privadex_chain_metadata::common::{Amount, ChainTokenId, UniversalAddress, UniversalTokenId};

use super::super::common::{Result, SubstrateError};

// Note that the relay chain does not receive or send parachain tokens (yet),
// so we do not expect BalanceDmp or AssetUmp
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct XCMTransferLookup {
    pub src_token: UniversalTokenId,
    pub dest_token: UniversalTokenId,
    pub token_pallet: TokenPallet,
    pub msg_pass_direction: MessagePassingDirection,
    pub amount: Amount,
    pub dest_addr: UniversalAddress,
}

impl XCMTransferLookup {
    pub fn from_tokens_amount_addr(
        src_token: UniversalTokenId,
        dest_token: UniversalTokenId,
        amount: Amount,
        dest_addr: UniversalAddress,
    ) -> Result<Self> {
        if (src_token.chain.get_relay() != dest_token.chain.get_relay())
            || (src_token.chain == dest_token.chain)
        {
            return Err(SubstrateError::InvalidXcmLookup);
        }

        let token_pallet = match dest_token.id {
            ChainTokenId::Native => TokenPallet::Balance,
            _ => TokenPallet::Asset,
        };
        let msg_pass_direction = match (
            src_token.chain.get_parachain_id().is_some(),
            dest_token.chain.get_parachain_id().is_some(),
        ) {
            // (is_parachain, is_parachain)
            (true, true) => Ok(MessagePassingDirection::Xcmp),
            (true, false) => Ok(MessagePassingDirection::Ump),
            (false, true) => Ok(MessagePassingDirection::Dmp),
            (false, false) => Err(SubstrateError::InvalidXcmLookup),
        }?;

        Ok(Self {
            src_token,
            dest_token,
            token_pallet,
            msg_pass_direction,
            amount,
            dest_addr,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum TokenPallet {
    // Produces
    // 1. assets.Issued event: (assetId = dest_token, owner = dest_addr, totalSupply = unknown a priori)
    // 2. assets.Issued event: (assetId = dest_token, owner = Treasury, totalSupply = unknown a priori)
    // ^The Treasury address is arbitrarily chosen by each parachain, so we ignore it. The above two
    // totalSupply quantities add up to amount_in
    Asset, // typically used for fungible foreign assets

    // Produces
    // 1. balances.Withdraw event: (who = sovereign_account of src parachain, amount = amount_in)
    // 2. balances.Deposit event: (who = dest_addr, amount = unknown a priori)
    Balance, // typically used for the native token
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum MessagePassingDirection {
    Xcmp, // 'Cross-chain message passing' = parachain-to-parachain
    Ump,  // 'Upward message passing' = parachain-to-relay
    Dmp,  // 'Downward message passing' = relay-to-parachain
}

impl MessagePassingDirection {
    // The name of the event that is produced on the destination chain
    pub fn event_success_name(&self) -> String {
        match self {
            Self::Xcmp => "XcmpQueue.Success",
            Self::Ump => "Ump.ExecutedUpward",
            Self::Dmp => "DmpQueue.ExecutedDownward",
        }
        .to_string()
    }
}
