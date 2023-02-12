import { CommonHandlerContext } from '@subsquid/substrate-processor'
import { Store } from '@subsquid/typeorm-store'
import assert from 'assert'
import { Pair } from '../model'

export async function getPair(ctx: CommonHandlerContext<Store>, id: string) {
    const item = await ctx.store.get(Pair, {
        where: { id },
        relations: { token0: true, token1: true },
    })
    assert(item != null, id)

    return item
}