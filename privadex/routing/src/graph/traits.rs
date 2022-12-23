use privadex_chain_metadata::common::{
	Amount,
	UniversalTokenId,
};

// Implemented for GraphSolution, GraphRoute, Edge, SwapEdge, etc.
pub trait QuoteGetter {
	fn get_src_dest_token(&self) -> (&UniversalTokenId /* src */, &UniversalTokenId /* dest */);

	// All quotes are in the dest token
	fn get_quote(&self, amount_in: Amount) -> Amount;

	fn get_quote_with_estimated_txn_fees(&self, amount_in: Amount) -> Amount {
		let quote = self.get_quote(amount_in);
		let fee = self.get_estimated_txn_fees();
		if quote > fee { quote - fee } else { 0 }
	}

	fn get_estimated_txn_fees(&self) -> Amount;
}
