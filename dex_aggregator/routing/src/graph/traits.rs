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

use privadex_chain_metadata::common::{Amount, UniversalTokenId};

// Implemented for GraphPath, Edge, SwapEdge, BridgeEdge, etc.
pub trait QuoteGetter {
    fn get_src_dest_token(
        &self,
    ) -> (
        &UniversalTokenId, /* src */
        &UniversalTokenId, /* dest */
    );

    // All quotes are in the dest token
    fn get_quote(&self, amount_in: Amount) -> Amount;

    fn get_quote_with_estimated_txn_fees(&self, amount_in: Amount) -> Amount {
        let quote = self.get_quote(amount_in);
        let fee = self.get_estimated_txn_fees_in_dest_token();
        if quote > fee {
            quote - fee
        } else {
            0
        }
    }

    fn get_estimated_txn_fees_in_dest_token(&self) -> Amount;

    // in $ x 10^USD_AMOUNT_EXPONENT
    fn get_estimated_txn_fees_usd(&self) -> Amount;

    // This is used downstream when converting to an ExecutionPlan to
    // estimate the postend_step's gas fee
    // It is identical to get_estimated_txn_fees_usd(...) for SwapEdges
    // but corresponds to the dest chain's gas fee for BridgeEdges. Note
    // that this is NOT paid (and thus is NOT included in get_estimated_txn_fees_usd)
    // because gas fee is only paid on the source chain
    fn get_dest_chain_estimated_gas_fee_usd(&self) -> Amount;
}
