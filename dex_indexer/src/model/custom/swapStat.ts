import {BigDecimal} from '@subsquid/big-decimal'
import {
    Entity as Entity_,
    Column as Column_,
    PrimaryColumn as PrimaryColumn_,
    ManyToOne as ManyToOne_,
    Index as Index_,
} from 'typeorm'
import * as marshal from "../generated/marshal"

export enum SwapPeriod {
    DAY = 'Day',
    MONTH = 'Month',
    WEEK = 'Week',
}

@Entity_()
export class SwapStatPeriod {
    constructor(props?: Partial<SwapStatPeriod>) {
        Object.assign(this, props)
    }

    @PrimaryColumn_()
    id!: string

    @Column_('timestamp with time zone', { nullable: false })
    from!: Date

    @Column_('timestamp with time zone', { nullable: false })
    to!: Date

    @Column_('int4', { nullable: false })
    swapsCount!: number

    @Column_('int4', { nullable: false })
    pairsCount!: number

    @Column_('int4', { nullable: false })
    usersCount!: number

    @Column_("numeric", {transformer: marshal.bigdecimalTransformer, nullable: false})
    totalAmountUSD!: BigDecimal
}
