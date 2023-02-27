import { SubstrateBatchProcessor } from '@subsquid/substrate-processor'
import { TypeormDatabase } from '@subsquid/typeorm-store'
import * as factory from './types/abi/factory'
import fs from 'fs'
import { FACTORY_ADDRESS, POOLS_JSON_FILE, SUBSQUID_ARCHIVE } from './dex_consts'

const processor = new SubstrateBatchProcessor().setBatchSize(500).addEvmLog(FACTORY_ADDRESS, {
    filter: [[factory.events['PairCreated(address,address,address,uint256)'].topic]],
    data: {
        event: {
            args: true,
        },
    } as const,
})

processor.setDataSource({
    archive: SUBSQUID_ARCHIVE,
})

const knownPools: string[] = []

processor.run(new TypeormDatabase({ stateSchema: 'pairs' }), async (ctx) => {
    for (let block of ctx.blocks) {
        for (let item of block.items) {
            if (item.kind !== 'event' || item.name !== 'EVM.Log') continue
            const log = item.event.args.log || item.event.args
            ctx.log.info(log);
            if (log.address === FACTORY_ADDRESS) {
                const data = factory.events['PairCreated(address,address,address,uint256)'].decode(log)
                knownPools.push(data.pair.toLowerCase())
                ctx.log.info(data);
                fs.writeFileSync(
                    POOLS_JSON_FILE,
                    JSON.stringify({ lastBlock: block.header.height, pools: knownPools })
                )
            }
            // if (item.kind === 'log' && item.address === '0xdac17f958d2ee523a2206206994597c13d831ec7')
        }
    }
})
