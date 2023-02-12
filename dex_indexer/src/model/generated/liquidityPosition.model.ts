import {BigDecimal} from "@subsquid/big-decimal"
import {Entity as Entity_, Column as Column_, PrimaryColumn as PrimaryColumn_, ManyToOne as ManyToOne_, Index as Index_} from "typeorm"
import * as marshal from "./marshal"
import {Pair} from "./pair.model"

@Entity_()
export class LiquidityPosition {
  constructor(props?: Partial<LiquidityPosition>) {
    Object.assign(this, props)
  }

  @PrimaryColumn_()
  id!: string

  @Column_("text", {nullable: false})
  user!: string

  @Index_()
  @ManyToOne_(() => Pair, {nullable: true})
  pair!: Pair

  @Column_("numeric", {transformer: marshal.bigdecimalTransformer, nullable: false})
  liquidityTokenBalance!: BigDecimal
}
