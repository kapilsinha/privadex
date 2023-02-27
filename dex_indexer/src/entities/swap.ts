import { getOrCreateToken } from './token'
import { EvmLogHandlerContext } from '@subsquid/substrate-processor'
import { Store } from '@subsquid/typeorm-store'
import { Pool } from '../model/generated/pool.model'
import { Token } from '../model'
import * as SwapFlash from '../types/abi/swapFlashLoan'
import { BaseMapper, EntityMap } from '../mappers/baseMapper'

const ZERO_ADDRESS = '0x0000000000000000000000000000000000000000'

interface SwapInfo {
    tokens: string[]
    balances: bigint[]
    a: bigint
    swapFee: bigint
    adminFee: bigint
    virtualPrice: bigint
    owner: string
    lpToken: string
}

export async function getOrCreatePool(this: BaseMapper<any>, entities: EntityMap, address: string): Promise<Pool> {
    let pool = entities.get(Pool).get(address)
    if (pool) return pool

    pool = await this.ctx.store.get(Pool, address)
    if (pool) {
        entities.get(Pool).set(address, pool)
        return pool
    }

    const info = await getSwapInfo.call(this, address)

    pool = new Pool({
        id: address.toLowerCase(),
        numTokens: info.tokens.length,
        tokens: (await registerTokens.call(this, entities, info.tokens)).map((t) => t.id),
        a: info.a,
        balances: info.balances,
        lpToken: info.lpToken,
        swapFee: info.swapFee,
        adminFee: info.adminFee,
        virtualPrice: info.virtualPrice,
        owner: info.owner,
    })
    entities.get(Pool).set(address, pool)

    return pool
}

export async function getSwapInfo(this: BaseMapper<any>, address: string): Promise<SwapInfo> {
    const swapContract = new SwapFlash.Contract(this.ctx, this.block, address)

    const tokens: string[] = []
    const balances: bigint[] = []

    let i = 0

    while (true) {
        try {
            const t = await swapContract.getToken(i)
            const b = await swapContract.getTokenBalance(i)

            if (t != ZERO_ADDRESS) {
                tokens.push(t.toLowerCase())
            }

            balances.push(b.toBigInt())

            i++
        } catch (e) {
            break
        }
    }

    const swapStorage = await swapContract.swapStorage()

    return {
        tokens,
        balances,
        a: (await swapContract.getA()).toBigInt(),
        swapFee: swapStorage.swapFee.toBigInt(),
        adminFee: swapStorage.adminFee.toBigInt(),
        virtualPrice: (await swapContract.getVirtualPrice()).toBigInt(),
        owner: await swapContract.owner(),
        lpToken: swapStorage.lpToken,
    }
}

export async function getBalancesSwap(
    ctx: EvmLogHandlerContext<Store>,
    swap: string,
    N_COINS: number
): Promise<bigint[]> {
    const swapContract = new SwapFlash.Contract(ctx, swap)
    const balances: bigint[] = new Array(N_COINS)

    for (let i = 0; i < N_COINS; ++i) {
        balances[i] = (await swapContract.getTokenBalance(i)).toBigInt()
    }

    return balances
}

async function registerTokens(this: BaseMapper<any>, entities: EntityMap, list: string[]): Promise<Token[]> {
    const result: Token[] = []

    for (let i = 0; i < list.length; ++i) {
        const current = list[i]

        if (current != ZERO_ADDRESS) {
            const token = await getOrCreateToken.call(this, entities, current)

            result.push(token)
        }
    }

    return result
}
