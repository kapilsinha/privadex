import type {Result} from './support'

export interface EvmLog {
  address: H160
  topics: H256[]
  data: Uint8Array
}

export type H160 = Uint8Array

export type H256 = Uint8Array
