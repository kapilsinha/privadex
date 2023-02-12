import { Token, LiquidityPosition, Pair } from '../model'
import { BatchContext, SubstrateBlock } from '@subsquid/substrate-processor'
import { Store } from '@subsquid/typeorm-store'
import * as erc20 from '../types/abi/erc20'
import * as erc20NameBytes from '../types/abi/erc20NameBytes'
import * as erc20SymbolBytes from '../types/abi/erc20SymbolBytes'
import { ZERO_BD } from '../consts'
import {BigDecimal} from '@subsquid/big-decimal'

async function fetchTokenSymbol(
    contract: erc20.Contract,
    contractSympolBytes: erc20SymbolBytes.Contract
): Promise<string> {
    try {
        const symbolResult = await contract.symbol()

        return symbolResult
    } catch (err) {
        const symbolResultBytes = await contractSympolBytes.symbol()

        return Buffer.from(symbolResultBytes).toString('ascii')
    }
}

async function fetchTokenName(contract: erc20.Contract, contractNameBytes: erc20NameBytes.Contract): Promise<string> {
    try {
        const nameResult = await contract.name()

        return nameResult
    } catch (err) {
        const nameResultBytes = await contractNameBytes.name()

        return Buffer.from(nameResultBytes).toString('ascii')
    }
}

async function fetchTokenTotalSupply(contract: erc20.Contract): Promise<bigint> {
    const totalSupplyResult = (await contract.totalSupply())?.toBigInt()

    return totalSupplyResult
}

async function fetchTokenDecimals(contract: erc20.Contract): Promise<number> {
    const decimalsResult = await contract.decimals()

    return decimalsResult
}

interface LiquidityPositionData {
    pair: Pair
    user: string
}

export function createLiquidityPosition(data: LiquidityPositionData): LiquidityPosition {
    const { pair, user } = data

    pair.liquidityProviderCount += 1

    return new LiquidityPosition({
        id: `${pair.id}-${user}`,
        liquidityTokenBalance: ZERO_BD,
        pair,
        user,
    })
}

// interface LiquiditySnapshotData {
//     position: LiquidityPosition
//     block: SubstrateBlock
//     bundle: Bundle
//     pair: Pair
//     user: User
// }

// export function createLiquiditySnapshot(data: LiquiditySnapshotData): LiquidityPositionSnapshot {
//     const { position, block, bundle, pair, user } = data

//     const token0 = pair.token0
//     const token1 = pair.token1

//     // create new snapshot
//     const snapshot = new LiquidityPositionSnapshot({
//         id: `${position.id}-${block.timestamp}`,
//         liquidityPosition: position,
//         timestamp: new Date(block.timestamp),
//         block: BigInt(block.height),
//         user,
//         pair,
//         token0PriceUSD: token0.derivedETH * bundle.ethPrice,
//         token1PriceUSD: token1.derivedETH * bundle.ethPrice,
//         reserve0: pair.reserve0,
//         reserve1: pair.reserve1,
//         reserveUSD: pair.reserveUSD,
//         liquidityTokenTotalSupply: pair.totalSupply,
//         liquidityTokenBalance: position.liquidityTokenBalance,
//     })

//     return snapshot
// }

export async function createToken(
    ctx: BatchContext<Store, unknown>,
    block: SubstrateBlock,
    address: string
): Promise<Token> {
    const contract = new erc20.Contract(ctx, block, address)
    const contractNameBytes = new erc20NameBytes.Contract(ctx, block, address)
    const contractSympolBytes = new erc20SymbolBytes.Contract(ctx, block, address)

    const symbol = await fetchTokenSymbol(contract, contractSympolBytes)
    const name = await fetchTokenName(contract, contractNameBytes)
    const totalSupply = await fetchTokenTotalSupply(contract)
    const decimals = await fetchTokenDecimals(contract)

    // bail if we couldn't figure out the decimals
    if (!decimals) {
        throw new Error(`Decimals for token ${address} not found`)
    }

    return new Token({
        id: address,
        symbol,
        name,
        totalSupply: BigDecimal(totalSupply, decimals).toFixed(),
        decimals,
        derivedETH: ZERO_BD,
        tradeVolume: ZERO_BD,
        tradeVolumeUSD: ZERO_BD,
        untrackedVolumeUSD: ZERO_BD,
        totalLiquidity: ZERO_BD,
        txCount: 0,
    })
}
