import {BigDecimal} from "@subsquid/big-decimal"
import {Entity as Entity_, Column as Column_, PrimaryColumn as PrimaryColumn_, ManyToOne as ManyToOne_, Index as Index_, OneToMany as OneToMany_} from "typeorm"
import * as marshal from "./marshal"
import {Token} from "./token.model"
import {LiquidityPosition} from "./liquidityPosition.model"
import {TokenSwapEvent} from "./tokenSwapEvent.model"

@Entity_()
export class Pair {
  constructor(props?: Partial<Pair>) {
    Object.assign(this, props)
  }

  @PrimaryColumn_()
  id!: string

  @Index_()
  @ManyToOne_(() => Token, {nullable: true})
  token0!: Token

  @Column_("text", {nullable: false})
  token0Id!: string

  @Index_()
  @ManyToOne_(() => Token, {nullable: true})
  token1!: Token

  @Column_("text", {nullable: false})
  token1Id!: string

  @Column_("numeric", {transformer: marshal.bigdecimalTransformer, nullable: false})
  reserve0!: BigDecimal

  @Column_("numeric", {transformer: marshal.bigdecimalTransformer, nullable: false})
  reserve1!: BigDecimal

  @Column_("numeric", {transformer: marshal.bigdecimalTransformer, nullable: false})
  totalSupply!: BigDecimal

  @Column_("numeric", {transformer: marshal.bigdecimalTransformer, nullable: false})
  reserveETH!: BigDecimal

  @Column_("numeric", {transformer: marshal.bigdecimalTransformer, nullable: false})
  reserveUSD!: BigDecimal

  @Column_("numeric", {transformer: marshal.bigdecimalTransformer, nullable: false})
  trackedReserveETH!: BigDecimal

  @Column_("numeric", {transformer: marshal.bigdecimalTransformer, nullable: false})
  token0Price!: BigDecimal

  @Column_("numeric", {transformer: marshal.bigdecimalTransformer, nullable: false})
  token1Price!: BigDecimal

  @Column_("numeric", {transformer: marshal.bigdecimalTransformer, nullable: false})
  volumeToken0!: BigDecimal

  @Column_("numeric", {transformer: marshal.bigdecimalTransformer, nullable: false})
  volumeToken1!: BigDecimal

  @Column_("numeric", {transformer: marshal.bigdecimalTransformer, nullable: false})
  volumeUSD!: BigDecimal

  @Column_("numeric", {transformer: marshal.bigdecimalTransformer, nullable: false})
  untrackedVolumeUSD!: BigDecimal

  @Column_("int4", {nullable: false})
  txCount!: number

  @Column_("timestamp with time zone", {nullable: false})
  createdAtTimestamp!: Date

  @Column_("int4", {nullable: false})
  createdAtBlockNumber!: number

  @Column_("int4", {nullable: false})
  liquidityProviderCount!: number

  @OneToMany_(() => LiquidityPosition, e => e.pair)
  liquidityPositions!: LiquidityPosition[]

  @OneToMany_(() => TokenSwapEvent, e => e.pair)
  swaps!: TokenSwapEvent[]
}
