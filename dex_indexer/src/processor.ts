import { BatchContext, EvmLogEvent, SubstrateBatchProcessor, SubstrateBlock } from '@subsquid/substrate-processor'
import * as factory from './types/abi/factory'
import * as pair from './types/abi/pair'
import * as swapFlashLoan from './types/abi/swapFlashLoan'
import { DAY_MS, HOUR_MS, MONTH_MS, WEEK_MS } from './consts'
import { 
    CHAIN_NODE, FACTORY_ADDRESS, FACTORY_CREATION_BLOCK, POOLS_JSON_FILE, STABLESWAP_POOLS, SUBSQUID_ARCHIVE 
} from './dex_consts'
import { Store, TypeormDatabase } from '@subsquid/typeorm-store'
import {
    Pair,
    TokenSwapEvent,
    Swapper,
    SwapperType,
    UniswapFactory,
    Bundle,
    Token,
    LiquidityPosition,
    Transaction,
    Pool,
} from './model'
import { SwapStatPeriod, SwapPeriod } from './model/custom/swapStat'
import { Between, Not, In } from 'typeorm'
import { BaseMapper, EntityClass, EntityMap } from './mappers/baseMapper'
import { NewPairMapper } from './mappers/factory'
import { BurnMapper, MintMapper, SwapMapper, SyncMapper, TransferMapper } from './mappers/pairs'
import { TokenSwapMapper } from './mappers/swapFlashLoan'
import { BigDecimal } from '@subsquid/big-decimal'
import { readFileSync } from 'fs'
import { getEvmLog, getTransaction } from '@subsquid/frontier'

const knownContracts: { lastBlock: number; pools: string[] } = JSON.parse(
    readFileSync(POOLS_JSON_FILE).toString()
)

const database = new TypeormDatabase()
const processor = new SubstrateBatchProcessor()
    // .setBatchSize(200) // deprecated
    .setBlockRange({ from: FACTORY_CREATION_BLOCK }) // from 0 is default behavior
    .setDataSource({
        chain: CHAIN_NODE,
        archive: SUBSQUID_ARCHIVE,
    })
    .setTypesBundle('moonbeam')
    .addEvmLog(FACTORY_ADDRESS, {
        filter: [factory.events['PairCreated(address,address,address,uint256)'].topic],
    })
    .addEvmLog('*', {
        filter: [
            [
                pair.events['Sync(uint112,uint112)'].topic,
                pair.events['Swap(address,uint256,uint256,uint256,uint256,address)'].topic,
            ],
        ],
        range: {
            from: knownContracts.lastBlock + 1,
        },
    })

for (const [address, start_block] of STABLESWAP_POOLS) {
    processor.addEvmLog(address, {
        filter: [[swapFlashLoan.events['TokenSwap(address,uint256,uint256,uint128,uint128)'].topic]],
        range: { from: start_block },
    })
}
    
for (const address of knownContracts.pools) {
    processor.addEvmLog(address, {
        filter: [
            [
                pair.events['Sync(uint112,uint112)'].topic,
                pair.events['Swap(address,uint256,uint256,uint256,uint256,address)'].topic,
            ],
        ],
        range: {
            from: 0,
            to: knownContracts.lastBlock,
        },
    })
}

processor.run(database, async (ctx) => {
    const mappers: BaseMapper<any>[] = []

    // console.log("Start run")
    for (const block of ctx.blocks) {
        for (const item of block.items) {
            if (item.kind === 'event') {
                if (item.name === 'EVM.Log') {
                    await handleEvmLog(ctx, block.header, item.event).then((mapper) => {
                        if (mapper != null) mappers.push(mapper)
                    })
                }
            }
        }
    }

    const requests = new Map<EntityClass, Set<string>>()
    for (const mapper of mappers) {
        for (const [entityClass, ids] of mapper.getRequest()) {
            const oldRequest = requests.get(entityClass) || new Set()
            requests.set(entityClass, new Set([...oldRequest, ...ids]))
        }
    }

    const entities = new EntityMap()
    for (const [entityClass, ids] of requests) {
        const e: Map<string, any> = await ctx.store
            .find(entityClass, { where: { id: In([...ids]) } })
            .then((es) => new Map(es.map((e: any) => [e.id, e])))
        entities.set(entityClass, e)
    }
    entities.set(Token, await ctx.store.find(Token, {}).then((es) => new Map(es.map((e: any) => [e.id, e]))))

    for (const mapper of mappers) {
        await mapper.process(entities)
    }

    await ctx.store.save([...entities.get(UniswapFactory).values()])
    await ctx.store.save([...entities.get(Bundle).values()])
    await ctx.store.save([...entities.get(Token).values()])
    await ctx.store.save([...entities.get(Pair).values()])
    await ctx.store.save([...entities.get(Pool).values()])
    await ctx.store.save([...entities.get(LiquidityPosition).values()])
    await ctx.store.save([...entities.get(Transaction).values()])
    await ctx.store.save([...entities.get(TokenSwapEvent).values()])

    for (const [entityClass, entity] of entities) {
        ctx.log.info(`saved ${entity.size} ${entityClass.name}`)
    }

    const lastBlock = ctx.blocks[ctx.blocks.length - 1].header
    await updateTop(ctx, lastBlock)
})

async function isKnownPairContracts(store: Store, address: string) {
    const normalizedAddress = address.toLowerCase()
    if (knownContracts.pools.includes(normalizedAddress)) {
        return true
    } else if ((await store.countBy(Pair, { id: normalizedAddress })) > 0) {
        knownContracts.pools.push(normalizedAddress)
        return true
    }
    return false
}

async function handleEvmLog(
    ctx: BatchContext<Store, unknown>,
    block: SubstrateBlock,
    event: EvmLogEvent
): Promise<BaseMapper<any> | undefined> {
    const evmLog = getEvmLog(ctx, event)
    // console.log("Call:", event.call.name, event.call.args.transaction)
    // console.log("Misc:", block.height, evmLog, ctx._chain.getCallHash('Ethereum.transact'))
    const transaction = getTransaction(ctx, event.call)
    const contractAddress = evmLog.address
    // console.log(block.height, contractAddress)
    if (contractAddress == FACTORY_ADDRESS) {
        return await new NewPairMapper(ctx, block).parse(evmLog)
    } else if (STABLESWAP_POOLS.map(x => x[0].toLowerCase()).includes(contractAddress)) {
        return await new TokenSwapMapper(ctx, block).parse(evmLog, transaction)
    } else if (await isKnownPairContracts(ctx.store, contractAddress)) {
        switch (evmLog.topics[0]) {
            case pair.events['Transfer(address,address,uint256)'].topic:
                return await new TransferMapper(ctx, block).parse(evmLog, transaction)
            case pair.events['Sync(uint112,uint112)'].topic:
                return await new SyncMapper(ctx, block).parse(evmLog)
            case pair.events['Swap(address,uint256,uint256,uint256,uint256,address)'].topic:
                return await new SwapMapper(ctx, block).parse(evmLog, transaction)
            case pair.events['Mint(address,uint256,uint256)'].topic:
                return await new MintMapper(ctx, block).parse(evmLog)
            case pair.events['Burn(address,uint256,uint256,address)'].topic:
                return await new BurnMapper(ctx, block).parse(evmLog)
        }
    }
}

const topUpdateInterval = 60 * 60 * 1000
let lastUpdateTopTimestamp: number | undefined

async function updateTop(ctx: BatchContext<Store, unknown>, block: SubstrateBlock) {
    if (lastUpdateTopTimestamp == null) {
        const swapStat = await ctx.store.findOneBy(SwapStatPeriod, { id: SwapPeriod.DAY })
        lastUpdateTopTimestamp = swapStat?.to.getTime() || -topUpdateInterval
    }

    if (block.timestamp < lastUpdateTopTimestamp + topUpdateInterval) return
    ctx.log.info('Updating top...')

    const swappers = new Map<string, Swapper>()

    const end = Math.floor(block.timestamp / HOUR_MS) * HOUR_MS

    const newSwapStat: Record<SwapPeriod, SwapStatPeriod> = {
        [SwapPeriod.DAY]: createSwapStat(SwapPeriod.DAY, end - DAY_MS, end),
        [SwapPeriod.WEEK]: createSwapStat(SwapPeriod.WEEK, Math.floor((end - WEEK_MS) / DAY_MS) * DAY_MS, end),
        [SwapPeriod.MONTH]: createSwapStat(SwapPeriod.MONTH, Math.floor((end - MONTH_MS) / DAY_MS) * DAY_MS, end),
    }

    const start = Math.min(...Object.values(newSwapStat).map((s) => s.from.getTime()))

    const swaps = await ctx.store.find(TokenSwapEvent, {
        where: { timestamp: Between(new Date(start), new Date(end)) },
    })

    for await (const TokenSwapEvent of swaps) {
        let user = swappers.get(TokenSwapEvent.buyer)
        if (user == null) {
            user = new Swapper({
                id: TokenSwapEvent.buyer,
                dayAmountUSD: BigDecimal('0'),
                weekAmountUSD: BigDecimal('0'),
                monthAmountUSD: BigDecimal('0'),
                type: SwapperType.USER,
            })
            swappers.set(user.id, user)
        }

        let pair = swappers.get(TokenSwapEvent.pairId!)
        if (pair == null) {
            pair = new Swapper({
                id: String(TokenSwapEvent.pairId),
                dayAmountUSD: BigDecimal('0'),
                weekAmountUSD: BigDecimal('0'),
                monthAmountUSD: BigDecimal('0'),
                type: SwapperType.PAIR,
            })
            swappers.set(pair.id, pair)
        }

        if (TokenSwapEvent.timestamp.getTime() >= end - DAY_MS) {
            user.dayAmountUSD = TokenSwapEvent.amountUSD.plus(user.dayAmountUSD)
            pair.dayAmountUSD = TokenSwapEvent.amountUSD.plus(pair.dayAmountUSD)
            updateSwapStat(newSwapStat[SwapPeriod.DAY], TokenSwapEvent.amountUSD)
        }

        if (TokenSwapEvent.timestamp.getTime() >= end - WEEK_MS) {
            user.weekAmountUSD = TokenSwapEvent.amountUSD.plus(user.weekAmountUSD)
            pair.weekAmountUSD = TokenSwapEvent.amountUSD.plus(pair.weekAmountUSD)
            updateSwapStat(newSwapStat[SwapPeriod.WEEK], TokenSwapEvent.amountUSD)
        }

        if (TokenSwapEvent.timestamp.getTime() >= end - MONTH_MS) {
            user.monthAmountUSD = TokenSwapEvent.amountUSD.plus(user.monthAmountUSD)
            pair.monthAmountUSD = TokenSwapEvent.amountUSD.plus(pair.monthAmountUSD)
            updateSwapStat(newSwapStat[SwapPeriod.MONTH], TokenSwapEvent.amountUSD)
        }
    }

    for (const swapper of swappers.values()) {
        if (swapper.type === SwapperType.PAIR) {
            if (swapper.dayAmountUSD.gt(0)) newSwapStat[SwapPeriod.DAY].pairsCount += 1
            if (swapper.weekAmountUSD.gt(0)) newSwapStat[SwapPeriod.WEEK].pairsCount += 1
            if (swapper.monthAmountUSD.gt(0)) newSwapStat[SwapPeriod.MONTH].pairsCount += 1
        } else {
            if (swapper.dayAmountUSD.gt(0)) newSwapStat[SwapPeriod.DAY].usersCount += 1
            if (swapper.weekAmountUSD.gt(0)) newSwapStat[SwapPeriod.WEEK].usersCount += 1
            if (swapper.monthAmountUSD.gt(0)) newSwapStat[SwapPeriod.MONTH].usersCount += 1
        }
    }

    await ctx.store.save(Object.values(newSwapStat))
    await ctx.store.remove(await ctx.store.findBy(Swapper, { id: Not(In([...swappers.keys()])) }))
    await ctx.store.save([...swappers.values()])

    lastUpdateTopTimestamp = block.timestamp

    ctx.log.info('Top updated.')
}

function updateSwapStat(swapStat: SwapStatPeriod, amountUSD: BigDecimal) {
    swapStat.swapsCount += 1
    swapStat.totalAmountUSD = amountUSD.plus(swapStat.totalAmountUSD)
}

function createSwapStat(id: SwapPeriod, from: number, to: number) {
    return new SwapStatPeriod({
        id,
        from: new Date(from),
        to: new Date(to),
        swapsCount: 0,
        usersCount: 0,
        pairsCount: 0,
        totalAmountUSD: BigDecimal('0'),
    })
}
