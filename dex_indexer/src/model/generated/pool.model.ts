import {Entity as Entity_, Column as Column_, PrimaryColumn as PrimaryColumn_, OneToMany as OneToMany_} from "typeorm"
import * as marshal from "./marshal"
import {TokenSwapEvent} from "./tokenSwapEvent.model"

@Entity_()
export class Pool {
  constructor(props?: Partial<Pool>) {
    Object.assign(this, props)
  }

  @PrimaryColumn_()
  id!: string

  @Column_("int4", {nullable: false})
  numTokens!: number

  @Column_("text", {array: true, nullable: false})
  tokens!: (string)[]

  @Column_("numeric", {array: true, nullable: false})
  balances!: (bigint)[]

  @Column_("text", {nullable: false})
  lpToken!: string

  @Column_("numeric", {transformer: marshal.bigintTransformer, nullable: false})
  a!: bigint

  @Column_("numeric", {transformer: marshal.bigintTransformer, nullable: false})
  swapFee!: bigint

  @Column_("numeric", {transformer: marshal.bigintTransformer, nullable: false})
  adminFee!: bigint

  @Column_("numeric", {transformer: marshal.bigintTransformer, nullable: false})
  virtualPrice!: bigint

  @Column_("text", {nullable: false})
  owner!: string

  @OneToMany_(() => TokenSwapEvent, e => e.pool)
  swaps!: TokenSwapEvent[]
}
