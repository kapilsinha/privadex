import { CommonHandlerContext } from '@subsquid/substrate-processor'
import { Store } from '@subsquid/typeorm-store'
import assert from 'assert'
import { FACTORY_ADDRESS } from '../dex_consts'
import { UniswapFactory, Bundle, LiquidityPosition, Transaction } from '../model'

let _uniswap: UniswapFactory | undefined

export async function getUniswap(ctx: CommonHandlerContext<Store>) {
    _uniswap = _uniswap || (await ctx.store.get(UniswapFactory, FACTORY_ADDRESS))
    assert(_uniswap != null)

    return _uniswap
}

let _bundle: Bundle | undefined

export async function getBundle(ctx: CommonHandlerContext<Store>) {
    _bundle = _bundle || (await ctx.store.get(Bundle, '1'))
    assert(_bundle != null)

    return _bundle
}

// const pairs: Map<string, Pair> = new Map()

// const tokens: Map<string, Token> = new Map()

// export async function getToken(store: Store, id: string) {
//     let item = tokens.get(id)

//     if (item == null) {
//         item = await store.get(Token, id)
//         assert(item != null)
//         if (item) tokens.set(item.id, item)
//     }

//     return item
// }

// const transactions: Map<string, Transaction> = new Map()

export async function getTransaction(ctx: CommonHandlerContext<Store>, id: string) {
    const item = await ctx.store.get(Transaction, id)

    return item
}

// export function addTransaction(item: Transaction) {
//     transactions.set(item.id, item)
// }

// const swaps: Map<string, TokenSwapEvent> = new Map()

// export function addSwap(item: TokenSwapEvent) {
//     swaps.set(item.id, item)
// }

// const positions: Map<string, LiquidityPosition> = new Map()

export async function getPosition(ctx: CommonHandlerContext<Store>, id: string) {
    const item = await ctx.store.get(LiquidityPosition, id)

    return item
}

// export function addPosition(item: LiquidityPosition) {
//     positions.set(item.id, item)
// }

// export async function saveAll(store: Store) {
//     if (uniswap != null) await store.save(uniswap)

//     if (bundle != null) await store.save(bundle)

//     await store.save([...tokens.values()])

//     await store.save([...pairs.values()])

//     await store.save([...transactions.values()])
//     transactions.clear()

//     await store.save([...swaps.values()])
//     swaps.clear()

//     await store.save([...positions.values()])
//     positions.clear()
// }
