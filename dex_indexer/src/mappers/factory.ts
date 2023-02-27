import { EvmLogEvent } from '@subsquid/substrate-processor'
import { BaseMapper, EntityClass, EntityMap } from './baseMapper'
import { Bundle, Pair, UniswapFactory, Token } from '../model'
import * as factoryAbi from '../types/abi/factory'
import { ZERO_BD } from '../consts'
import { getOrCreateToken } from '../entities/token'
import {EvmLog} from '@subsquid/frontier'

interface NewPairData {
    blockNumber: number
    timestamp: Date
    factoryId: string
    pairId: string
    token0Id: string
    token1Id: string
}

export class NewPairMapper extends BaseMapper<NewPairData> {
    async parse(evmLog: EvmLog) {
        const contractAddress = evmLog.address

        const data = factoryAbi.events['PairCreated(address,address,address,uint256)'].decode(evmLog)

        this.data = {
            factoryId: contractAddress,
            timestamp: new Date(this.block.timestamp),
            blockNumber: this.block.height,
            pairId: data.pair.toLowerCase(),
            // user stats
            token0Id: data.token0.toLowerCase(),
            token1Id: data.token1.toLowerCase(),
        }
        // console.log("NewPairMapper", this.data)

        return this
    }

    getRequest(): Map<EntityClass, string[]> {
        if (this.data == null) {
            return new Map()
        } else {
            const { token0Id, token1Id, factoryId } = this.data
            return new Map().set(Token, [token0Id, token1Id]).set(UniswapFactory, [factoryId]).set(Bundle, ['ethP'])
        }
    }

    async process(entities: EntityMap) {
        if (this.data == null) return

        const { pairId, token0Id, token1Id, factoryId, blockNumber, timestamp } = this.data

        let factory = entities.get(UniswapFactory).get(factoryId)
        if (factory == null) {
            factory = new UniswapFactory({
                id: factoryId,
                pairCount: 0,
                totalVolumeETH: ZERO_BD,
                totalLiquidityETH: ZERO_BD,
                totalVolumeUSD: ZERO_BD,
                untrackedVolumeUSD: ZERO_BD,
                totalLiquidityUSD: ZERO_BD,
                txCount: 0,
            })

            entities.get(UniswapFactory).set(factory.id, factory)
        }

        let bundle = entities.get(Bundle).get('ethP')
        if (bundle == null) {
            bundle = new Bundle({
                id: 'ethP',
                ethPrice: ZERO_BD,
            })
            entities.get(Bundle).set(bundle.id, bundle)
        }

        // create the tokens
        const token0 = await getOrCreateToken.call(this, entities, token0Id)
        const token1 = await getOrCreateToken.call(this, entities, token1Id)

        const pair = new Pair({
            id: pairId,
            token0Id: token0.id,
            token1Id: token1.id,
            liquidityProviderCount: 0,
            createdAtTimestamp: timestamp,
            createdAtBlockNumber: blockNumber,
            txCount: 0,
            reserve0: ZERO_BD,
            reserve1: ZERO_BD,
            trackedReserveETH: ZERO_BD,
            reserveETH: ZERO_BD,
            reserveUSD: ZERO_BD,
            totalSupply: ZERO_BD,
            volumeToken0: ZERO_BD,
            volumeToken1: ZERO_BD,
            volumeUSD: ZERO_BD,
            untrackedVolumeUSD: ZERO_BD,
            token0Price: ZERO_BD,
            token1Price: ZERO_BD,
        })

        entities.get(Pair).set(pair.id, pair)
    }
}
