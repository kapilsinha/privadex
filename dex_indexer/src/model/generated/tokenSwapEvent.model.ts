import {BigDecimal} from "@subsquid/big-decimal"
import {Entity as Entity_, Column as Column_, PrimaryColumn as PrimaryColumn_, ManyToOne as ManyToOne_, Index as Index_} from "typeorm"
import * as marshal from "./marshal"
import {Transaction} from "./transaction.model"
import {Pair} from "./pair.model"
import {Pool} from "./pool.model"
import {Token} from "./token.model"

@Entity_()
export class TokenSwapEvent {
  constructor(props?: Partial<TokenSwapEvent>) {
    Object.assign(this, props)
  }

  @PrimaryColumn_()
  id!: string

  @Index_()
  @ManyToOne_(() => Transaction, {nullable: true})
  transaction!: Transaction | undefined | null

  @Index_()
  @Column_("timestamp with time zone", {nullable: false})
  timestamp!: Date

  @Index_()
  @ManyToOne_(() => Pair, {nullable: true})
  pair!: Pair | undefined | null

  @Column_("text", {nullable: true})
  pairId!: string | undefined | null

  @Index_()
  @ManyToOne_(() => Pool, {nullable: true})
  pool!: Pool | undefined | null

  @Column_("text", {nullable: true})
  poolId!: string | undefined | null

  @Index_()
  @Column_("text", {nullable: false})
  buyer!: string

  @Index_()
  @ManyToOne_(() => Token, {nullable: true})
  tokenSold!: Token

  @Column_("numeric", {transformer: marshal.bigintTransformer, nullable: false})
  soldAmount!: bigint

  @Index_()
  @ManyToOne_(() => Token, {nullable: true})
  tokenBought!: Token

  @Column_("numeric", {transformer: marshal.bigintTransformer, nullable: false})
  boughtAmount!: bigint

  @Column_("numeric", {transformer: marshal.bigdecimalTransformer, nullable: false})
  amountUSD!: BigDecimal
}
