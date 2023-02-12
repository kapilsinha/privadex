import { BigDecimal } from '@subsquid/big-decimal'

export const ADDRESS_ZERO = '0x0000000000000000000000000000000000000000'

export const ZERO_BI = 0n
export const ONE_BI = 1n
export const ZERO_BD = BigDecimal(0)
export const ONE_BD = BigDecimal(1)
export const BI_18 = 1000000000000000000n

export const PRECISION = 32

export const HOUR_MS = 1000.0 * 60.0 * 60.0
export const DAY_MS = HOUR_MS * 24.0
export const WEEK_MS = DAY_MS * 7.0
export const MONTH_MS = DAY_MS * 30.0
