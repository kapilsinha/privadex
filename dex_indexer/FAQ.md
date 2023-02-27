# How-to cheat sheet

## How to run a processor against a different chain?

You will need to have WebSocket endpoint to connect to the chain node and a Squid Archive. For a registry of Squid Archives, check this community-owned [Archive Registry](https://github.com/subsquid/archive-registry)

If you don't find a suitable Squid Archive in the registry, set up your own Squid archive. There are multiple examples in this [repo](https://github.com/subsquid/squid-archive-setup)

Once set up, we encourage to contribute to the Squid community and make a PR to the [Archive Registry](https://github.com/subsquid/archive-registry).


## Where do I get a type bundle for my chain?

Most chains publish their type bundles as an npm package. One of the best places to check for the latest version is the [polkadot-js/app repo](https://github.com/polkadot-js/apps/tree/master/packages/apps-config). Note, however, that a types bundle is only needed for pre-Metadata v14 blocks, so for recently deployed chains it may be not needed. 

Note that the type bundle format for typegen is slightly different from `OverrideBundleDefinition` of `polkadot.js`. The structure is as follows, all the fields are optional.

```javascript
{
  types: {}, // top-level type definitions, as `.types` option of `ApiPromise`
  typesAlias: {}, // top-level type alieases, as `.typesAlias` option of `ApiPromise`
  versions: [ // spec version specific overrides, same as `OverrideBundleDefinition.types` of `polkadot.js`
    {
       minmax: [0, 1010] // spec range
       types: {}, // type overrides for the spec range
       typesAlias: {}, // type alias overrides for the spec range
    }
  ]
}
```


## How do I write the schema?

The schema file defines the shape of the final GraphQL API and has very few limitations. Designing the schema file is very similar to the design of the database schema. As a rule of thumb, the schema should represent high level domain specific entities and relations between them, to make data fetching and filtering easy for the API consumers.

Typically, the API is consumed by the frontend and mobile apps, so it's a good idea to design the schema even before you go on with implementing the processor mappings. In fact, the processor is completely indpendent from the GraphQL server serving the API, so you can experiment how the API looks like.

## How do I update my schema?

TL;DR: If you're ok dropping the database, simply update `schema.graphql` and run:

```sh
bash reset-schema.sh
```

OBS! The database will be wiped out, so if it's not an option, read below. 


Here's a step-by-step instruction. First, generated the model files:

```sh
npx sqd codegen
npm run build
```

Now you have to options: either create a migration for an incremental schema update or recreate the whole schema from scratch. 

During the development process, recreating the schema is often more convenient. However, if you already have a running API in production and don't want to resync it, having an incremental update is preferrable (but requires data backfilling).

### Option 1: Recreate schema from scratch

Run

```sh
bash reset-db.sh
```

### Option 2: Make an incremental update to the schema

Generate a migration for the incremental changes and apply it

```sh
npx sqd db create-migration AddMyAwesomeNewField
npx sqd db migrate
```

You can find the newly generated and applied migration in `db/migrations`.


## How do I run and test the GraphQL API?

Once the migrations are applied, simply run

```
npx squid-graphql-server
```

Observe the port (4350 by default) and navigate to `localhost:4350/graphql` to explore your API. However, you need to run the processor to start populating the database.


## How do I start the processor?

First, make sure you have compiled your project with
```
npm run build
```

Then simply run 
```
node -r dotenv/config lib/processor.js
```

Note that `-r dotenv/config` ensures that the database settings are picked up from `.env`. If you the environment variables them elsewhere, skip it. 

## How do I deploy my API to the Subsquid Hosted service?

Login to the [Subsquid Hosted Service](https://app.subsquid.io) with your github handle to obtain a deployment key. Then create a Squid (that is, your deployment) and follow the instructions.

## How do I know which events and extrinsics I need for the handlers? 

This part depends on the runtime business-logic of the chain. The primary and the most reliable source of information is thus the Rust sources for the pallets used by the chain. 
For a quick lookup of the documentation and the data format it is often useful to check `Runtime` section of Subscan, e.g. for [Statemine](https://statemine.subscan.io/runtime). One can see the deployed pallets and drill down to events and extrinsics from there. One can also choose the spec version on the drop down.

## How do I decode the event data? And how to deal with runtime upgrades?

Runtime upgrades may change the event data and even the event logic altogether, but Squid gets you covered with a first-class support for runtime upgrades. 

Subsquid SDK comes with a tool called metadata explorer which makes it easy to keep track of all runtime upgrades happen so far.

The basic usage of the explorer is as follows (check README for details):

```sh
npx squid-substrate-metadata-explorer \
  --chain <chain endpoint here> \
  --archive <archive endpoint here> \
  --out metadataVersions.json
```

Once the exploration is done, you should define all events and calls of interest in `typegen.json`, then adjust the bundle and metadata history references and run:

```sh
npx squid-substrate-typegen typegen.json
```

A type-safe definition for each and every version of the event will be generated. Most of the times, one should be able to infer a normalized interface together with some glue code to make it fit the runtime specific versions. For example, for Kusama `balances.Transfer` event, `squid-substrate-typegen` generated three slightly different versions that can be reconciled as follows:

```typescript
/**
 * Normalized `balances.Transfer` event data
 */
interface TransferEvent {
    from: Uint8Array
    to: Uint8Array
    amount: bigint
}

function getTransferEvent(ctx: EventHandlerContext): TransferEvent {
    // instanciate type-safe facade around event data
    let event = new BalancesTransferEvent(ctx)
    // initial version, with runtime spec 1020
    if (event.isV1020) { 
        let [from, to, amount, fee] = event.asV1020 
        return {from, to, amount}
    // first upgrade at runtime spec version 1050
    } else if (event.isV1050) { 
        let [from, to, amount] = event.asV1050
        return {from, to, amount}
    } else { // current version
        // This cast will assert,  
        // that the type of a given event matches
        // the type of generated facade.
        return event.asLatest
    }
}
```

## Resolved Errors
Error when running `make pairs`:
```
00:02:58 DEBUG sqd:processor:archive-request response
                                             archiveUrl: https://moonbeam.archive.subsquid.io/graphql
                                             archiveRequestId: 32
00:02:58 FATAL sqd:processor Error: Unexpected case: 6
                                 at throwUnexpectedCase (/Users/kapilsinha/Desktop/beamswap-squid/node_modules/@subsquid/scale-codec/lib/util.js:14:11)
                                 at getCodecType (/Users/kapilsinha/Desktop/beamswap-squid/node_modules/@subsquid/scale-codec/lib/types-codec.js:96:52)
                                 at toCodecTypes (/Users/kapilsinha/Desktop/beamswap-squid/node_modules/@subsquid/scale-codec/lib/types-codec.js:176:25)
                                 at new JsonCodec (/Users/kapilsinha/Desktop/beamswap-squid/node_modules/@subsquid/scale-codec/lib/codec-json.js:15:53)
                                 at new Chain (/Users/kapilsinha/Desktop/beamswap-squid/node_modules/@subsquid/substrate-processor/lib/chain.js:114:26)
                                 at ChainManager.createChain (/Users/kapilsinha/Desktop/beamswap-squid/node_modules/@subsquid/substrate-processor/lib/chain.js:69:16)
                                 at ChainManager.getChainForBlock (/Users/kapilsinha/Desktop/beamswap-squid/node_modules/@subsquid/substrate-processor/lib/chain.js:54:34)
                                 at process.processTicksAndRejections (node:internal/process/task_queues:95:5)
                                 at async Runner.splitBySpec (/Users/kapilsinha/Desktop/beamswap-squid/node_modules/@subsquid/substrate-processor/lib/processor/runner.js:194:29)
                                 at async Runner.process (/Users/kapilsinha/Desktop/beamswap-squid/node_modules/@subsquid/substrate-processor/lib/processor/runner.js:161:25)
```
Resolved by bumping "@subsquid/frontier" in package.json from 0.0.0 to 0.0.1

```
18:43:48 FATAL sqd:processor Error: Uknown "Ethereum.transact" version
                                 at getTransaction (node_modules/@subsquid/substrate-frontier-evm/lib/transaction.js:30:19)
                                 at handleEvmLog (lib/processor.js:145:69)
                                 at lib/processor.js:90:27
                                 at node_modules/@subsquid/substrate-processor/lib/processor/batchProcessor.js:300:28
                                 at TypeormDatabase.runTransaction (node_modules/@subsquid/typeorm-store/lib/database.js:110:19)
                                 at TypeormDatabase.transact (node_modules/@subsquid/typeorm-store/lib/database.js:64:35)
                                 at runner.processBatch (node_modules/@subsquid/substrate-processor/lib/processor/batchProcessor.js:299:27)
                                 at Runner.process (node_modules/@subsquid/substrate-processor/lib/processor/runner.js:164:28)
                                 at process.processTicksAndRejections (node:internal/process/task_queues:95:5)

```
This error happened on Moonbeam after a few 100k blocks at block 415948, soon after the specId is updated from moonbeam@1103 to moonbeam@1201. I needed to ask Subsquid devs to look into it.
They resolved some bugs in their types and hash computation code and released frontier 0.1.0.

```
04:10:40 FATAL sqd:processor AssertionError [ERR_ASSERTION]: The expression evaluated to a falsy value:

                               (0, assert_1.default)(pair != null)

                                 at SyncMapper.process (/Users/kapilsinha/Desktop/beamswap-squid/lib/mappers/pairs.js:147:30)
                                 at /Users/kapilsinha/Desktop/beamswap-squid/lib/processor.js:113:22
                                 at process.processTicksAndRejections (node:internal/process/task_queues:95:5)
                                 at async TypeormDatabase.runTransaction (/Users/kapilsinha/Desktop/beamswap-squid/node_modules/@subsquid/typeorm-store/lib/database.js:110:13)
                                 at async TypeormDatabase.transact (/Users/kapilsinha/Desktop/beamswap-squid/node_modules/@subsquid/typeorm-store/lib/database.js:64:24)
                                 at async Runner.process (/Users/kapilsinha/Desktop/beamswap-squid/node_modules/@subsquid/substrate-processor/lib/processor/runner.js:164:17)
                             err:
                               generatedMessage: true
                               code: ERR_ASSERTION
                               actual: false
                               expected: true
                               operator: ==
```
Resolved by (1) setting the from_block to the factory creation block and
by (2) not requiring the usdc-weth pair to exist (necessary for StellaSwap's USDC.wh)
