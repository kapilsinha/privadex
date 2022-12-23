# RoutingSolutionToExecutionPlanConverter (Connector)

***crate: privadex_route_execution_connector, mod: route_to_execution***

Input: Route solution of path(s) from src to dest

Output: An execution plan of sequential executable steps

Graph struct definitions are in the `graph` crate

Execution struct definitions are in the `executor` crate.

# Conversion logic

## GraphSolution -> ExecutionPlan

```rust
fn convert_graph_soln_to_exec_plan(graph_soln: GraphSolution) -> ExecutionPlan;

struct GraphSolution {
	paths: Vec<GraphPath>,
	src_token: UniversalTokenId,
	src_addr: Address, // wallet src
	dest_token: UniversalTokenId, 
	dest_addr: Address, // wallet dest
}

struct ExecutionPlan {
	paths: Vec<ExecutionPath>,
	prestart_user_to_escrow_transfer: ExecutionStep,
	postend_escrow_to_user_transfer: ExecutionStep,
}
```

1. src_token, src_addr, escrow_addr → prestart_user_to_escrow_transfer := EthSendStep or ERC20TransferStep
2. *GraphPath → ExecutionPath* (see below)
3. dest_token, escrow_addr, dest_addr → postend_escrow_to_user_transfer := EthSendStep or ERC20TransferStep

## GraphPath → ExecutionPath

```rust
fn convert graph_path_to_exec_path(graph_path: GraphPath) -> ExecutionPath;

type GraphPath = Vec<Edge>;
type ExecutionPath = Vec<ExecutionStep>;
```

Loop over Edges in GraphPath until we reach the end

1. Initialize cur_dex to null. Keep moving our loop index forward if we encounter SwapEdge.Wrap | SwapEdge.Unwrap. If we see SwapEdge.CPMM and dex == cur_dex or cur_dex == null, then set cur_dex to dex and continue. If we see SwapEdge.CPMM with dex != cur_dex and cur_dex != null, stop at the previous step. If we see BridgeEdge, stop here.
2. Take the start and cur_index and convert that slice *List[GraphEdge] → ExecutionStep* (see below)

For example we would consolidate the following list as such: 

[Wrap, CPMM(Stella), CPMM(Stella)],   [CPMM(Beam), CPMM(Beam), Unwrap],   [Bridge],   [Wrap, CPMM(Arth)]

## List[Edge] → ExecutionStep

The input list is either a swap on a single DEX ([Wrap | Unwrap | CPMM(dex = x)]) or a bridge ([Bridge])

### Swap on a single DEX

```rust
struct ConstantProductAMMSwapEdge {
	// Used for SOR
	src_token: UniversalTokenId,
	dest_token: UniversalTokenId,
	token0: ChainTokenId,
	token1: ChainTokenId,
	reserve0: U256,
    reserve1: U256,
	
	// Token pair metadata needed for executor
	dex: Dex,
	pair_address: Address,
}

struct EthDexSwapStep {
	dex_router_addr: Address,
	token_path: Vec<UniversalTokenId>, // token.chain are all the same of course
	amount_in: Option<U256>,
	// eventually will add amount_out_min
	common: CommonExecutionMeta,
	status: EthStepStatus,
}
```

- dex_router_addr - comes from CPMM.dex + chain_info lookup
- token_path - taken straight from src_token, dest_token for the items in input list
- amount_in - initialized to null, this needs to be propagated from step to step by the executor
- common.src_addr, common.dest_addr - both are the escrow_address on this chain
- status initialized to NotStarted

### Bridge

```rust
struct XCMBridgeEdge {
	src_token: UniversalTokenId,
	dest_token: UniversalTokenId,

	// XCM instruction and fee metadata needed for executor
	token_asset_multilocation: xcm::latest::MultiLocation,
	dest_multilocation_template: xcm::latest::MultiLocation,
	estimated_bridge_fee_in_dest_chain_native_token: Amount, // in dest_network native token
}

struct XCMTransferStep {
	src_token: UniversalTokenId,
	dest_token: UniversalTokenId,
	token_asset_multilocation: xcm::latest::MultiLocation,
	dest_multilocation: xcm::latest::MultiLocation,
	amount: Option<Amount>,
	bridge_fee: Amount,
	bridge_fee_usd: Amount,
	common: CommonExecutionMeta,
	status: SubstrateCrossChainStepStatus,
}
```

- src_token, dest_token, token_asset_multilocation - simple passthrough
- dest_multilocation - take dest_multilocation_template and replace dest_address with the escrow_address on dest_chain (the template has address set to Address::zero())
- amount - initialized to null, this needs to be propagated from step to step by the executor
- bridge_fee, bridge_fee_usd, common.gas_fee, common.gas_fee_usd - initialized to zero
- common.src_addr, common.dest_addr - escrow addresses on src and dest chains
- status - NotStarted

# Misc

***crate: privadex_route_execution_connector, mod: validator***

We will write a `validateSolution` function that checks certain invariants (e.g. consecutive swaps are on the same chain, destToken1 = srcToken2), bridge after a swap src_chain = swap_chain, etc.)

