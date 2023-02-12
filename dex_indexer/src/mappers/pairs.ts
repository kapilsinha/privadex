import { EvmLogEvent } from '@subsquid/substrate-processor'
import { ADDRESS_ZERO, ZERO_BD } from '../consts'
import { FACTORY_ADDRESS, WHITELIST } from '../dex_consts'
import { Transaction, TokenSwapEvent, Pair, LiquidityPosition, Bundle, UniswapFactory } from '../model'
import { getEthPriceInUSD, findEthPerToken, MINIMUM_USD_THRESHOLD_NEW_PAIRS } from '../utils/pricing'
import * as pairAbi from '../types/abi/pair'
import { createLiquidityPosition } from '../utils/helpers'
import { BaseMapper, EntityClass, EntityMap } from './baseMapper'
import assert from 'assert'
import { getOrCreateToken } from '../entities/token'
import { BigDecimal } from '@subsquid/big-decimal'
import { EvmLog, Transaction as EvmTransaction } from '@subsquid/frontier'

const transferEventAbi = pairAbi.events['Transfer(address,address,uint256)']

interface TransferData {
    txHash: string
    timestamp: Date
    blockNumber: number
    pairId: string
    fromId: string
    toId: string
    amount: bigint
}

export class TransferMapper extends BaseMapper<TransferData> {
    async parse(evmLog: EvmLog, transaction: EvmTransaction) {
        const contractAddress = evmLog.address

        const data = transferEventAbi.decode(evmLog)
        // ignore initial transfers for first adds

        if (data.to === ADDRESS_ZERO && data.value.toBigInt() === 1000n) {
            return this
        }

        this.data = {
            txHash: transaction.hash,
            timestamp: new Date(this.block.timestamp),
            blockNumber: this.block.height,
            pairId: contractAddress,
            // user stats
            fromId: data.from.toLowerCase(),
            toId: data.to.toLowerCase(),
            amount: data.value.toBigInt(),
        }

        return this
    }

    getRequest(): Map<EntityClass, string[]> {
        if (this.data == null) {
            return new Map()
        } else {
            const { txHash, pairId, fromId, toId } = this.data
            return new Map()
                .set(Transaction, [txHash])
                .set(Pair, [pairId])
                .set(LiquidityPosition, [`${pairId}-${fromId}`, `${pairId}-${toId}`])
        }
    }

    async process(entities: EntityMap) {
        if (this.data == null) return

        const { txHash, pairId, blockNumber, timestamp, fromId, toId } = this.data

        // get pair and load contract
        const pair = entities.get(Pair).get(pairId)
        assert(pair != null)

        // liquidity token amount being transfered
        const value = BigDecimal(this.data.amount, 18)

        // get or create transaction
        let transaction = entities.get(Transaction).get(txHash)
        if (transaction == null) {
            transaction = new Transaction({
                id: txHash,
                blockNumber,
                timestamp,
                swaps: [],
            })
            entities.get(Transaction).set(txHash, transaction)
        }

        // mints
        if (fromId === ADDRESS_ZERO) {
            pair.totalSupply = pair.totalSupply.plus(value)
        }

        // burn
        if (toId == ADDRESS_ZERO && fromId == pair.id) {
            pair.totalSupply = pair.totalSupply.minus(value)
        }

        if (fromId !== ADDRESS_ZERO && fromId !== pair.id) {
            if (!entities.get(LiquidityPosition).has(`${pairId}-${fromId}`)) {
                const position = createLiquidityPosition({
                    pair,
                    user: fromId,
                })
                entities.get(LiquidityPosition).set(position.id, position)
            }
        }

        if (toId !== ADDRESS_ZERO && toId !== pair.id) {
            if (!entities.get(LiquidityPosition).has(`${pairId}-${toId}`)) {
                const position = createLiquidityPosition({
                    pair,
                    user: fromId,
                })
                entities.get(LiquidityPosition).set(position.id, position)
            }
        }
    }
}

const syncEventAbi = pairAbi.events['Sync(uint112,uint112)']

interface SyncData {
    pairId: string
    reserve0: bigint
    reserve1: bigint
}

export class SyncMapper extends BaseMapper<SyncData> {
    async parse(evmLog: EvmLog) {
        const contractAddress = evmLog.address

        const data = syncEventAbi.decode(evmLog)
        // ignore initial transfers for first adds

        this.data = {
            pairId: contractAddress,
            reserve0: data.reserve0.toBigInt(),
            reserve1: data.reserve1.toBigInt(),
        }

        return this
    }

    getRequest(): Map<EntityClass, string[]> {
        if (this.data == null) {
            return new Map()
        } else {
            const { pairId } = this.data
            return new Map().set(Pair, [pairId]).set(UniswapFactory, [FACTORY_ADDRESS]).set(Bundle, ['ethP'])
        }
    }

    async process(entities: EntityMap) {
        if (this.data == null) return

        const { pairId, reserve0, reserve1 } = this.data

        // get pair and load contract
        const pair = entities.get(Pair).get(pairId)
        assert(pair != null)

        const bundle = entities.get(Bundle).get('ethP')
        assert(bundle != null)

        const uniswap = entities.get(UniswapFactory).get(FACTORY_ADDRESS)
        assert(uniswap != null)

        const token0 = await getOrCreateToken.call(this, entities, pair.token0Id)
        const token1 = await getOrCreateToken.call(this, entities, pair.token1Id)

        // reset factory liquidity by subtracting onluy tarcked liquidity
        uniswap.totalLiquidityETH = uniswap.totalLiquidityETH.minus(pair.trackedReserveETH)

        // reset token total liquidity amounts
        token0.totalLiquidity = token0.totalLiquidity.minus(pair.reserve0)
        token1.totalLiquidity = token1.totalLiquidity.minus(pair.reserve1)

        pair.reserve0 = BigDecimal(reserve0, token0.decimals)
        pair.reserve1 = BigDecimal(reserve1, token1.decimals)

        pair.token0Price = !pair.reserve1.eq(ZERO_BD) ? pair.reserve0.div(pair.reserve1) : ZERO_BD
        pair.token1Price = !pair.reserve0.eq(ZERO_BD) ? pair.reserve1.div(pair.reserve0) : ZERO_BD

        // update ETH price now that reserves could have changed
        bundle.ethPrice = await getEthPriceInUSD.call(this, entities)

        token0.derivedETH = await findEthPerToken.call(this, entities, token0.id)
        token1.derivedETH = await findEthPerToken.call(this, entities, token1.id)

        let trackedLiquidityETH = ZERO_BD
        if (!bundle.ethPrice.eq(ZERO_BD)) {
            const price0 = token0.derivedETH.times(bundle.ethPrice)
            const price1 = token1.derivedETH.times(bundle.ethPrice)

            // both are whitelist tokens, take average of both amounts
            if (WHITELIST.includes(token0.id) && WHITELIST.includes(token1.id)) {
                trackedLiquidityETH = pair.reserve0.times(price0).plus(pair.reserve1.times(price1))
            }

            // take double value of the whitelisted token amount
            if (WHITELIST.includes(token0.id) && !WHITELIST.includes(token1.id)) {
                trackedLiquidityETH = pair.reserve0.times(price0).times(2)
            }

            // take double value of the whitelisted token amount
            if (!WHITELIST.includes(token0.id) && WHITELIST.includes(token1.id)) {
                trackedLiquidityETH = pair.reserve1.times(price1).times(2)
            }

            trackedLiquidityETH = trackedLiquidityETH.div(bundle.ethPrice)
        }

        // use derived amounts within pair
        pair.trackedReserveETH = trackedLiquidityETH
        pair.reserveETH = pair.reserve0.times(token0.derivedETH).plus(pair.reserve1.times(token1.derivedETH))
        pair.reserveUSD = pair.reserveETH.times(bundle.ethPrice)

        // use tracked amounts globally
        uniswap.totalLiquidityETH = uniswap.totalLiquidityETH.plus(trackedLiquidityETH)
        uniswap.totalLiquidityUSD = uniswap.totalLiquidityETH.plus(bundle.ethPrice)

        // now correctly set liquidity amounts for each token
        token0.totalLiquidity = token0.totalLiquidity.plus(pair.reserve0)
        token1.totalLiquidity = token1.totalLiquidity.plus(pair.reserve1)
    }
}

const mintAbi = pairAbi.events['Mint(address,uint256,uint256)']

interface MintData {
    pairId: string
    fromId: string
}

export class MintMapper extends BaseMapper<MintData> {
    async parse(evmLog: EvmLog) {
        const contractAddress = evmLog.address

        const data = mintAbi.decode(evmLog)

        this.data = {
            pairId: contractAddress,
            fromId: data.sender.toLowerCase(),
        }

        return this
    }

    getRequest(): Map<EntityClass, string[]> {
        if (this.data == null) {
            return new Map()
        } else {
            const { pairId, fromId } = this.data
            return new Map()
                .set(Pair, [pairId])
                .set(LiquidityPosition, [`${pairId}-${fromId}`])
                .set(UniswapFactory, [FACTORY_ADDRESS])
                .set(Bundle, ['ethP'])
        }
    }

    async process(entities: EntityMap) {
        if (this.data == null) return

        const { pairId, fromId } = this.data

        // get pair and load contract
        const pair = entities.get(Pair).get(pairId)
        assert(pair != null)

        const bundle = entities.get(Bundle).get('ethP')
        assert(bundle != null)

        const uniswap = entities.get(UniswapFactory).get(FACTORY_ADDRESS)
        assert(uniswap != null)

        const token0 = await getOrCreateToken.call(this, entities, pair.token0Id)
        const token1 = await getOrCreateToken.call(this, entities, pair.token1Id)

        token0.txCount += 1

        token1.txCount += 1

        // update txn counts
        pair.txCount += 1

        // update txn counts
        uniswap.txCount += 1

        // update the LP position

        if (!entities.get(LiquidityPosition).has(`${pairId}-${fromId}`)) {
            const position = createLiquidityPosition({
                pair,
                user: fromId,
            })
            entities.get(LiquidityPosition).set(position.id, position)
        }
    }
}

const burnAbi = pairAbi.events['Burn(address,uint256,uint256,address)']

interface BurnData {
    pairId: string
    fromId: string
}

export class BurnMapper extends BaseMapper<BurnData> {
    async parse(evmLog: EvmLog) {
        const contractAddress = evmLog.address

        const data = burnAbi.decode(evmLog)

        this.data = {
            pairId: contractAddress,
            fromId: data.sender.toLowerCase(),
        }

        return this
    }

    getRequest(): Map<EntityClass, string[]> {
        if (this.data == null) {
            return new Map()
        } else {
            const { pairId, fromId } = this.data
            return new Map()
                .set(Pair, [pairId])
                .set(LiquidityPosition, [`${pairId}-${fromId}`])
                .set(UniswapFactory, [FACTORY_ADDRESS])
                .set(Bundle, ['ethP'])
        }
    }

    async process(entities: EntityMap) {
        if (this.data == null) return

        const { pairId, fromId } = this.data

        // get pair and load contract
        const pair = entities.get(Pair).get(pairId)
        assert(pair != null)

        const bundle = entities.get(Bundle).get('ethP')
        assert(bundle != null)

        const uniswap = entities.get(UniswapFactory).get(FACTORY_ADDRESS)
        assert(uniswap != null)

        const token0 = await getOrCreateToken.call(this, entities, pair.token0Id)
        const token1 = await getOrCreateToken.call(this, entities, pair.token1Id)

        token0.txCount += 1

        token1.txCount += 1

        // update txn counts
        pair.txCount += 1

        // update txn counts
        uniswap.txCount += 1

        // update the LP position

        if (!entities.get(LiquidityPosition).has(`${pairId}-${fromId}`)) {
            const position = createLiquidityPosition({
                pair,
                user: fromId,
            })
            entities.get(LiquidityPosition).set(position.id, position)
        }
    }
}

const swapAbi = pairAbi.events['Swap(address,uint256,uint256,uint256,uint256,address)']

interface SwapData {
    txHash: string
    timestamp: Date
    blockNumber: number
    pairId: string
    fromId: string
    amount0In: bigint
    amount1In: bigint
    amount0Out: bigint
    amount1Out: bigint
    toId: string
}

export class SwapMapper extends BaseMapper<SwapData> {
    async parse(evmLog: EvmLog, transaction: EvmTransaction) {
        const contractAddress = evmLog.address

        const data = swapAbi.decode(evmLog)

        this.data = {
            txHash: transaction.hash,
            timestamp: new Date(this.block.timestamp),
            blockNumber: this.block.height,
            pairId: contractAddress,
            fromId: data.sender.toLowerCase(),
            amount0In: data.amount0In.toBigInt(),
            amount1In: data.amount1In.toBigInt(),
            amount0Out: data.amount0Out.toBigInt(),
            amount1Out: data.amount1Out.toBigInt(),
            toId: data.to.toLowerCase(),
        }

        return this
    }

    getRequest(): Map<EntityClass, string[]> {
        if (this.data == null) {
            return new Map()
        } else {
            const { pairId, txHash } = this.data
            return new Map()
                .set(Transaction, [txHash])
                .set(Pair, [pairId])
                .set(UniswapFactory, [FACTORY_ADDRESS])
                .set(Bundle, ['ethP'])
        }
    }

    async process(entities: EntityMap) {
        if (this.data == null) return

        const { pairId, fromId, txHash, timestamp, blockNumber } = this.data

        const pair = entities.get(Pair).get(pairId)
        assert(pair != null)

        const bundle = entities.get(Bundle).get('ethP')
        assert(bundle != null)

        const uniswap = entities.get(UniswapFactory).get(FACTORY_ADDRESS)
        assert(uniswap != null)

        const token0 = await getOrCreateToken.call(this, entities, pair.token0Id)
        const amount0In = BigDecimal(this.data.amount0In, token0.decimals)
        const amount0Out = BigDecimal(this.data.amount0Out, token0.decimals)
        const amount0Total = amount0Out.plus(amount0In)

        const token1 = await getOrCreateToken.call(this, entities, pair.token1Id)
        const amount1In = BigDecimal(this.data.amount1In, token1.decimals)
        const amount1Out = BigDecimal(this.data.amount1Out, token1.decimals)
        const amount1Total = amount1Out.plus(amount1In)

        // get total amounts of derived USD and ETH for tracking
        const derivedAmountETH = token1.derivedETH
            .times(amount1Total)
            .plus(token0.derivedETH.times(amount0Total))
            .div(2)
        const derivedAmountUSD = derivedAmountETH.times(bundle.ethPrice)
        // only accounts for volume through white listed tokens

        let trackedAmountUSD = ZERO_BD

        const price0 = token0.derivedETH.times(bundle.ethPrice)
        const price1 = token1.derivedETH.times(bundle.ethPrice)

        const reserve0USD = pair.reserve0.times(price0)
        const reserve1USD = pair.reserve1.times(price1)

        // // if less than 5 LPs, require high minimum reserve amount amount or return 0
        // if (
        //     pair.liquidityProviderCount < 5 &&
        //     ((WHITELIST.includes(token0.id) &&
        //         WHITELIST.includes(token1.id) &&
        //         reserve0USD.plus(reserve1USD).lt(MINIMUM_USD_THRESHOLD_NEW_PAIRS)) ||
        //         (WHITELIST.includes(token0.id) &&
        //             !WHITELIST.includes(token1.id) &&
        //             reserve0USD.times(2).lt(MINIMUM_USD_THRESHOLD_NEW_PAIRS)) ||
        //         (!WHITELIST.includes(token0.id) &&
        //             WHITELIST.includes(token1.id) &&
        //             reserve1USD.times(2).lt(MINIMUM_USD_THRESHOLD_NEW_PAIRS)))
        // ) {
        //     // do nothing
        // } else {
        // both are whitelist tokens, take average of both amounts
        if (WHITELIST.includes(token0.id) && WHITELIST.includes(token1.id)) {
            trackedAmountUSD = amount0Total.times(price0).plus(amount1Total.times(price1)).div(2)
        }

        // take full value of the whitelisted token amount
        if (WHITELIST.includes(token0.id) && !WHITELIST.includes(token1.id)) {
            trackedAmountUSD = amount0Total.times(price0)
        }

        // take full value of the whitelisted token amount
        if (!WHITELIST.includes(token0.id) && WHITELIST.includes(token1.id)) {
            trackedAmountUSD = amount1Total.times(price1)
        }
        // }

        const trackedAmountETH = bundle.ethPrice.eq(ZERO_BD) ? ZERO_BD : trackedAmountUSD.div(bundle.ethPrice)
        // update token0 global volume and token liquidity stats
        token0.tradeVolume = token0.tradeVolume.plus(amount0Total)
        token0.tradeVolumeUSD = token0.tradeVolumeUSD.plus(trackedAmountUSD)
        token0.untrackedVolumeUSD = token0.untrackedVolumeUSD.plus(derivedAmountUSD)
        token0.txCount += 1
        // update token1 global volume and token liquidity stats
        token1.tradeVolume = token1.tradeVolume.plus(amount1Total)
        token1.tradeVolumeUSD = token1.tradeVolumeUSD.plus(trackedAmountUSD)
        token1.untrackedVolumeUSD = token1.untrackedVolumeUSD.plus(derivedAmountUSD)
        token1.txCount += 1

        // update pair volume data, use tracked amount if we have it as its probably more accurate
        pair.volumeUSD = pair.volumeUSD.plus(trackedAmountUSD)
        pair.volumeToken0 = pair.volumeToken0.plus(amount0Total)
        pair.volumeToken1 = pair.volumeToken1.plus(amount1Total)
        pair.untrackedVolumeUSD = pair.untrackedVolumeUSD.plus(derivedAmountUSD)
        pair.txCount += 1

        // update global values, only used tracked amounts for volume
        uniswap.totalVolumeUSD = uniswap.totalVolumeUSD.plus(trackedAmountUSD)
        uniswap.totalVolumeETH = uniswap.totalVolumeETH.plus(trackedAmountETH)
        uniswap.untrackedVolumeUSD = uniswap.untrackedVolumeUSD.plus(derivedAmountUSD)
        uniswap.txCount += 1

        let transaction = entities.get(Transaction).get(txHash)
        if (transaction == null) {
            transaction = new Transaction({
                id: txHash,
                blockNumber,
                timestamp,
                swaps: [],
            })
            entities.get(Transaction).set(txHash, transaction)
        }

        const swapId = `${transaction.id}-${transaction.swaps?.length}`

        transaction.swaps?.push(swapId)

        // if (amount0Total.eq(0) && amount1Total.eq(0)) return

        const swap = new TokenSwapEvent({
            id: swapId,
            transaction,
            pair,
            timestamp,
            tokenSold: amount0In.eq(0) ? token1 : token0,
            soldAmount: this.data.amount0In || this.data.amount1In,
            tokenBought: amount0Out.eq(0) ? token1 : token0,
            boughtAmount: this.data.amount0Out || this.data.amount1Out,
            buyer: fromId,
            // sender: data.sender.toLowerCase(),
            // to: data.to.toLowerCase(),
            // from:
            amountUSD: trackedAmountUSD.eq(ZERO_BD) ? derivedAmountUSD : trackedAmountUSD,
        })

        entities.get(TokenSwapEvent).set(swap.id, swap)
    }
}
