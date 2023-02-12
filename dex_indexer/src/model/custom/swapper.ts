import { BigDecimal } from '@subsquid/big-decimal'
import { Entity as Entity_, Column as Column_, PrimaryColumn as PrimaryColumn_ } from 'typeorm'
import * as marshal from '../generated/marshal'

export enum SwapperType {
    PAIR = 'Pair',
    USER = 'User',
}

@Entity_()
export class Swapper {
    constructor(props?: Partial<Swapper>) {
        Object.assign(this, props)
    }

    @PrimaryColumn_()
    id!: string

    @Column_('varchar', { length: 5, nullable: false })
    type!: SwapperType

    @Column_('numeric', { transformer: marshal.bigdecimalTransformer, nullable: false })
    dayAmountUSD!: BigDecimal

    @Column_('numeric', { transformer: marshal.bigdecimalTransformer, nullable: false })
    weekAmountUSD!: BigDecimal

    @Column_('numeric', { transformer: marshal.bigdecimalTransformer, nullable: false })
    monthAmountUSD!: BigDecimal
}
