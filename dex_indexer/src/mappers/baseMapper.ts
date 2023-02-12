import { BatchContext, SubstrateBlock } from '@subsquid/substrate-processor'
import { Store } from '@subsquid/typeorm-store'

export abstract class BaseMapper<T> {
    protected data: T | undefined

    constructor(protected ctx: BatchContext<Store, unknown>, protected block: SubstrateBlock) {}
    abstract parse(...args: unknown[]): Promise<this>
    abstract process(entities: EntityMap): Promise<void>
    abstract getRequest(): Map<EntityClass, string[]>
}

export interface EntityClass<T = any> {
    new (): T
}

export class EntityMap {
    private map = new Map<EntityClass, Map<string, any>>()

    get<T>(entityConstructor: EntityClass<T>): Map<string, T> {
        let entities = this.map.get(entityConstructor)
        if (entities == null) {
            entities = new Map()
            this.map.set(entityConstructor, entities)
        }
        return entities
    }

    set<T>(entityConstructor: EntityClass<T>, entities: Map<string, T>): this {
        this.map.set(entityConstructor, entities)
        return this
    }

    [Symbol.iterator]() {
        return this.map.entries()
    }
}
