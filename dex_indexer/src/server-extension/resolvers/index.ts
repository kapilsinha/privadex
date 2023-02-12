import { Arg, Field, ObjectType, Query, Resolver, registerEnumType, Int, Info } from 'type-graphql'
import { GraphQLResolveInfo } from 'graphql'
import graphqlFields from 'graphql-fields'
import 'reflect-metadata'
import { EntityManager } from 'typeorm'
import { TokenSwapEvent, Swapper, SwapPeriod, SwapperType, SwapStatPeriod } from '../../model'
import { DAY_MS } from '../../consts'
import { In, Between } from 'typeorm'
import assert from 'assert'
import { BigDecimal } from '@subsquid/big-decimal'

@ObjectType()
class SwapInfoObject {
    constructor(props?: Partial<SwapInfoObject>) {
        Object.assign(this, props)
    }

    @Field(() => String, { nullable: false })
    id!: string

    @Field(() => [SwapDayVolumeObject], { nullable: false })
    volumesPerDay!: SwapDayVolumeObject[]
}

@ObjectType()
class SwapDayVolumeObject {
    constructor(props?: Partial<SwapDayVolumeObject>) {
        Object.assign(this, props)
    }

    @Field(() => Date, { nullable: false })
    day!: Date

    @Field(() => BigDecimal, { nullable: false })
    amountUSD!: BigDecimal
}

@ObjectType()
class SwapperObject {
    constructor(props?: Partial<SwapperObject>) {
        Object.assign(this, props)
    }

    @Field(() => String, { nullable: false })
    id!: string

    @Field(() => BigDecimal, { nullable: false })
    amountUSD!: BigDecimal
}

@ObjectType()
class TopObject {
    constructor(props?: Partial<TopObject>) {
        Object.assign(this, props)
    }

    @Field(() => Date, { nullable: false })
    from!: Date

    @Field(() => Date, { nullable: false })
    to!: Date

    @Field(() => Int, { nullable: false })
    count!: number

    @Field(() => Int, { nullable: false })
    swapsCount!: number

    @Field(() => BigDecimal, { nullable: false })
    totalAmountUSD!: BigDecimal

    @Field(() => [SwapperObject], { nullable: false })
    top!: SwapperObject[]
}

enum Order {
    ASC = 'ASC',
    DESC = 'DESC',
}

enum Range {
    DAY = '24 HOUR',
    WEEK = '7 DAY',
    MONTH = '30 DAY',
}

const rangeToSwapPeriod: Record<Range, SwapPeriod> = {
    [Range.DAY]: SwapPeriod.DAY,
    [Range.WEEK]: SwapPeriod.WEEK,
    [Range.MONTH]: SwapPeriod.MONTH,
}

registerEnumType(Order, { name: 'Order' })
registerEnumType(Range, { name: 'Range' })

interface TopOptions {
    type: SwapperType
    limit: number
    offset: number
    order: Order
    range: Range
    requireVolumesPerDay: boolean
    requireEntities: boolean
}

@Resolver()
export class TradersResolver {
    constructor(private tx: () => Promise<EntityManager>) {}

    @Query(() => TopObject || null)
    async getUsersTop(
        @Arg('limit', { nullable: true, defaultValue: null })
        limit: number,
        @Arg('offset', { nullable: true, defaultValue: 0 })
        offset: number,
        @Arg('order', () => Order, { nullable: true, defaultValue: Order.DESC })
        order: Order,
        @Arg('range', () => Range, { nullable: false })
        range: Range,
        @Info()
        info: GraphQLResolveInfo
    ): Promise<TopObject | null> {
        console.log(new Date(Date.now()), 'Query users top...')

        const fields = graphqlFields(info)

        const result = await this.getTop({
            type: SwapperType.USER,
            limit,
            offset,
            order,
            range,
            requireVolumesPerDay: fields.top?.volumesPerDay != null,
            requireEntities: fields.top != null,
        })

        return result
    }

    @Query(() => TopObject)
    async getPairsTop(
        @Arg('limit', { nullable: true, defaultValue: null })
        limit: number,
        @Arg('offset', { nullable: true, defaultValue: 0 })
        offset: number,
        @Arg('order', () => Order, { nullable: true, defaultValue: Order.DESC })
        order: Order,
        @Arg('range', () => Range, { nullable: false })
        range: Range,
        @Info()
        info: GraphQLResolveInfo
    ): Promise<TopObject> {
        console.log(new Date(Date.now()), 'Query pairs top...')

        const fields = graphqlFields(info)

        const result = await this.getTop({
            type: SwapperType.PAIR,
            limit,
            offset,
            order,
            range,
            requireVolumesPerDay: fields.top?.volumesPerDay != null,
            requireEntities: fields.top != null,
        })

        return result
    }

    private async getTop(options: TopOptions) {
        const { type, limit, offset, order, range, requireVolumesPerDay, requireEntities } = options
        const manager = await this.tx()
        const stat = await manager.getRepository(SwapStatPeriod).findOneBy({
            id: rangeToSwapPeriod[range],
        })
        assert(stat != null)

        const top: Swapper[] = requireEntities
            ? await manager
                  .query(
                      `
                    SELECT 
                        id, 
                        day_amount_usd as "dayAmountUSD",
                        week_amount_usd as "weekAmountUSD",
                        month_amount_usd as "monthAmountUSD"
                    FROM swapper
                    ORDER BY ${
                        range === Range.DAY
                            ? '"dayAmountUSD"'
                            : range === Range.WEEK
                            ? '"weekAmountUSD"'
                            : '"monthAmountUSD"'
                    } ${order}
                    ${offset != null ? `OFFSET ${offset}` : ''}
                    ${limit != null ? `LIMIT ${limit}` : ''}
                    `
                  )
                  .then((swappers) => swappers.map((s: any) => new Swapper(s)))
            : []

        return new TopObject({
            to: stat.to,
            from: stat.from,
            count: type === SwapperType.PAIR ? stat.pairsCount : stat.usersCount,
            totalAmountUSD: stat.totalAmountUSD,
            swapsCount: stat.swapsCount,
            top: top
                .map(
                    (s) =>
                        new SwapperObject({
                            id: s.id,
                            amountUSD:
                                range === Range.DAY
                                    ? s.dayAmountUSD
                                    : range === Range.WEEK
                                    ? s.weekAmountUSD
                                    : s.monthAmountUSD,
                        })
                )
                .sort((a, b) =>
                    BigDecimal(a.amountUSD)
                        .minus(b.amountUSD)
                        .mul(order === Order.DESC ? -1 : 1)
                        .toNumber()
                )
                .filter((u) => BigDecimal(u.amountUSD).gt(0)),
        })
    }
}

@Resolver()
export class DayVolumeResolver {
    constructor(private tx: () => Promise<EntityManager>) {}

    @Query(() => [SwapInfoObject] || null)
    async getDayVolume(
        @Arg('ids', () => [String], { nullable: true, defaultValue: null })
        ids: string[],
        @Arg('from', () => Date, { nullable: false })
        from: Date,
        @Arg('to', () => Date, { nullable: false })
        to: Date
    ): Promise<SwapInfoObject[]> {
        const manager = await this.tx()

        const users = new Map(ids.map((id) => [id, new SwapInfoObject({ id, volumesPerDay: [] })]))
        const swaps = await manager.getRepository(TokenSwapEvent).find({
            where: {
                buyer: In(ids),
                timestamp: Between(from, to),
            },
        })

        for (const TokenSwapEvent of swaps) {
            const user = users.get(TokenSwapEvent.buyer)
            assert(user != null)

            const timestamp = new Date(
                Math.max(Math.floor(TokenSwapEvent.timestamp.getTime() / DAY_MS) * DAY_MS, from.getTime())
            )

            let amountUSDperDay = user.volumesPerDay.find((d) => d.day.getTime() === timestamp.getTime())
            if (amountUSDperDay == null) {
                amountUSDperDay = new SwapDayVolumeObject({
                    day: timestamp,
                    amountUSD: BigDecimal('0'),
                })
                user.volumesPerDay.push(amountUSDperDay)
            }
            amountUSDperDay.amountUSD = amountUSDperDay.amountUSD.add(TokenSwapEvent.amountUSD)
        }

        return [...users.values()]
    }
}
