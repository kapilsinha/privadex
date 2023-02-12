import * as ethers from "ethers";
import assert from "assert";

export const abi = new ethers.utils.Interface(getJsonAbi());

export type AddLiquidity0Event = ([provider: string, tokenAmounts: Array<ethers.BigNumber>, fees: Array<ethers.BigNumber>, invariant: ethers.BigNumber, lpTokenSupply: ethers.BigNumber] & {provider: string, tokenAmounts: Array<ethers.BigNumber>, fees: Array<ethers.BigNumber>, invariant: ethers.BigNumber, lpTokenSupply: ethers.BigNumber})

export type FlashLoan0Event = ([receiver: string, tokenIndex: number, amount: ethers.BigNumber, amountFee: ethers.BigNumber, protocolFee: ethers.BigNumber] & {receiver: string, tokenIndex: number, amount: ethers.BigNumber, amountFee: ethers.BigNumber, protocolFee: ethers.BigNumber})

export type NewAdminFee0Event = ([newAdminFee: ethers.BigNumber] & {newAdminFee: ethers.BigNumber})

export type NewSwapFee0Event = ([newSwapFee: ethers.BigNumber] & {newSwapFee: ethers.BigNumber})

export type OwnershipTransferred0Event = ([previousOwner: string, newOwner: string] & {previousOwner: string, newOwner: string})

export type Paused0Event = ([account: string] & {account: string})

export type RampA0Event = ([oldA: ethers.BigNumber, newA: ethers.BigNumber, initialTime: ethers.BigNumber, futureTime: ethers.BigNumber] & {oldA: ethers.BigNumber, newA: ethers.BigNumber, initialTime: ethers.BigNumber, futureTime: ethers.BigNumber})

export type RemoveLiquidity0Event = ([provider: string, tokenAmounts: Array<ethers.BigNumber>, lpTokenSupply: ethers.BigNumber] & {provider: string, tokenAmounts: Array<ethers.BigNumber>, lpTokenSupply: ethers.BigNumber})

export type RemoveLiquidityImbalance0Event = ([provider: string, tokenAmounts: Array<ethers.BigNumber>, fees: Array<ethers.BigNumber>, invariant: ethers.BigNumber, lpTokenSupply: ethers.BigNumber] & {provider: string, tokenAmounts: Array<ethers.BigNumber>, fees: Array<ethers.BigNumber>, invariant: ethers.BigNumber, lpTokenSupply: ethers.BigNumber})

export type RemoveLiquidityOne0Event = ([provider: string, lpTokenAmount: ethers.BigNumber, lpTokenSupply: ethers.BigNumber, boughtId: ethers.BigNumber, tokensBought: ethers.BigNumber] & {provider: string, lpTokenAmount: ethers.BigNumber, lpTokenSupply: ethers.BigNumber, boughtId: ethers.BigNumber, tokensBought: ethers.BigNumber})

export type StopRampA0Event = ([currentA: ethers.BigNumber, time: ethers.BigNumber] & {currentA: ethers.BigNumber, time: ethers.BigNumber})

export type TokenSwap0Event = ([buyer: string, tokensSold: ethers.BigNumber, tokensBought: ethers.BigNumber, soldId: ethers.BigNumber, boughtId: ethers.BigNumber] & {buyer: string, tokensSold: ethers.BigNumber, tokensBought: ethers.BigNumber, soldId: ethers.BigNumber, boughtId: ethers.BigNumber})

export type Unpaused0Event = ([account: string] & {account: string})

export interface EvmEvent {
  data: string;
  topics: string[];
}

function decodeEvent(signature: string, data: EvmEvent): any {
  return abi.decodeEventLog(
    abi.getEvent(signature),
    data.data || "",
    data.topics
  );
}

export const events = {
  "AddLiquidity(address,uint256[],uint256[],uint256,uint256)": {
    topic: abi.getEventTopic("AddLiquidity(address,uint256[],uint256[],uint256,uint256)"),
    decode(data: EvmEvent): AddLiquidity0Event {
      return decodeEvent("AddLiquidity(address,uint256[],uint256[],uint256,uint256)", data)
    }
  }
  ,
  "FlashLoan(address,uint8,uint256,uint256,uint256)": {
    topic: abi.getEventTopic("FlashLoan(address,uint8,uint256,uint256,uint256)"),
    decode(data: EvmEvent): FlashLoan0Event {
      return decodeEvent("FlashLoan(address,uint8,uint256,uint256,uint256)", data)
    }
  }
  ,
  "NewAdminFee(uint256)": {
    topic: abi.getEventTopic("NewAdminFee(uint256)"),
    decode(data: EvmEvent): NewAdminFee0Event {
      return decodeEvent("NewAdminFee(uint256)", data)
    }
  }
  ,
  "NewSwapFee(uint256)": {
    topic: abi.getEventTopic("NewSwapFee(uint256)"),
    decode(data: EvmEvent): NewSwapFee0Event {
      return decodeEvent("NewSwapFee(uint256)", data)
    }
  }
  ,
  "OwnershipTransferred(address,address)": {
    topic: abi.getEventTopic("OwnershipTransferred(address,address)"),
    decode(data: EvmEvent): OwnershipTransferred0Event {
      return decodeEvent("OwnershipTransferred(address,address)", data)
    }
  }
  ,
  "Paused(address)": {
    topic: abi.getEventTopic("Paused(address)"),
    decode(data: EvmEvent): Paused0Event {
      return decodeEvent("Paused(address)", data)
    }
  }
  ,
  "RampA(uint256,uint256,uint256,uint256)": {
    topic: abi.getEventTopic("RampA(uint256,uint256,uint256,uint256)"),
    decode(data: EvmEvent): RampA0Event {
      return decodeEvent("RampA(uint256,uint256,uint256,uint256)", data)
    }
  }
  ,
  "RemoveLiquidity(address,uint256[],uint256)": {
    topic: abi.getEventTopic("RemoveLiquidity(address,uint256[],uint256)"),
    decode(data: EvmEvent): RemoveLiquidity0Event {
      return decodeEvent("RemoveLiquidity(address,uint256[],uint256)", data)
    }
  }
  ,
  "RemoveLiquidityImbalance(address,uint256[],uint256[],uint256,uint256)": {
    topic: abi.getEventTopic("RemoveLiquidityImbalance(address,uint256[],uint256[],uint256,uint256)"),
    decode(data: EvmEvent): RemoveLiquidityImbalance0Event {
      return decodeEvent("RemoveLiquidityImbalance(address,uint256[],uint256[],uint256,uint256)", data)
    }
  }
  ,
  "RemoveLiquidityOne(address,uint256,uint256,uint256,uint256)": {
    topic: abi.getEventTopic("RemoveLiquidityOne(address,uint256,uint256,uint256,uint256)"),
    decode(data: EvmEvent): RemoveLiquidityOne0Event {
      return decodeEvent("RemoveLiquidityOne(address,uint256,uint256,uint256,uint256)", data)
    }
  }
  ,
  "StopRampA(uint256,uint256)": {
    topic: abi.getEventTopic("StopRampA(uint256,uint256)"),
    decode(data: EvmEvent): StopRampA0Event {
      return decodeEvent("StopRampA(uint256,uint256)", data)
    }
  }
  ,
  "TokenSwap(address,uint256,uint256,uint128,uint128)": {
    topic: abi.getEventTopic("TokenSwap(address,uint256,uint256,uint128,uint128)"),
    decode(data: EvmEvent): TokenSwap0Event {
      return decodeEvent("TokenSwap(address,uint256,uint256,uint128,uint128)", data)
    }
  }
  ,
  "Unpaused(address)": {
    topic: abi.getEventTopic("Unpaused(address)"),
    decode(data: EvmEvent): Unpaused0Event {
      return decodeEvent("Unpaused(address)", data)
    }
  }
  ,
}

interface ChainContext  {
  _chain: Chain
}

interface BlockContext  {
  _chain: Chain
  block: Block
}

interface Block  {
  height: number
}

interface Chain  {
  client:  {
    call: <T=any>(method: string, params?: unknown[]) => Promise<T>
  }
}

export class Contract  {
  private readonly _chain: Chain
  private readonly blockHeight: number
  readonly address: string

  constructor(ctx: BlockContext, address: string)
  constructor(ctx: ChainContext, block: Block, address: string)
  constructor(ctx: BlockContext, blockOrAddress: Block | string, address?: string) {
    this._chain = ctx._chain
    if (typeof blockOrAddress === 'string')  {
      this.blockHeight = ctx.block.height
      this.address = ethers.utils.getAddress(blockOrAddress)
    }
    else  {
      assert(address != null)
      this.blockHeight = blockOrAddress.height
      this.address = ethers.utils.getAddress(address)
    }
  }

  async MAX_BPS(): Promise<ethers.BigNumber> {
    return this.call("MAX_BPS", [])
  }

  async calculateRemoveLiquidity(amount: ethers.BigNumber): Promise<Array<ethers.BigNumber>> {
    return this.call("calculateRemoveLiquidity", [amount])
  }

  async calculateRemoveLiquidityOneToken(tokenAmount: ethers.BigNumber, tokenIndex: number): Promise<ethers.BigNumber> {
    return this.call("calculateRemoveLiquidityOneToken", [tokenAmount, tokenIndex])
  }

  async calculateSwap(tokenIndexFrom: number, tokenIndexTo: number, dx: ethers.BigNumber): Promise<ethers.BigNumber> {
    return this.call("calculateSwap", [tokenIndexFrom, tokenIndexTo, dx])
  }

  async calculateTokenAmount(amounts: Array<ethers.BigNumber>, deposit: boolean): Promise<ethers.BigNumber> {
    return this.call("calculateTokenAmount", [amounts, deposit])
  }

  async flashLoanFeeBPS(): Promise<ethers.BigNumber> {
    return this.call("flashLoanFeeBPS", [])
  }

  async getA(): Promise<ethers.BigNumber> {
    return this.call("getA", [])
  }

  async getAPrecise(): Promise<ethers.BigNumber> {
    return this.call("getAPrecise", [])
  }

  async getAdminBalance(index: ethers.BigNumber): Promise<ethers.BigNumber> {
    return this.call("getAdminBalance", [index])
  }

  async getAdminBalances(): Promise<Array<ethers.BigNumber>> {
    return this.call("getAdminBalances", [])
  }

  async getLpToken(): Promise<string> {
    return this.call("getLpToken", [])
  }

  async getNumberOfTokens(): Promise<ethers.BigNumber> {
    return this.call("getNumberOfTokens", [])
  }

  async getToken(index: number): Promise<string> {
    return this.call("getToken", [index])
  }

  async getTokenBalance(index: number): Promise<ethers.BigNumber> {
    return this.call("getTokenBalance", [index])
  }

  async getTokenBalances(): Promise<Array<ethers.BigNumber>> {
    return this.call("getTokenBalances", [])
  }

  async getTokenIndex(tokenAddress: string): Promise<number> {
    return this.call("getTokenIndex", [tokenAddress])
  }

  async getTokenPrecisionMultipliers(): Promise<Array<ethers.BigNumber>> {
    return this.call("getTokenPrecisionMultipliers", [])
  }

  async getTokens(): Promise<Array<string>> {
    return this.call("getTokens", [])
  }

  async getVirtualPrice(): Promise<ethers.BigNumber> {
    return this.call("getVirtualPrice", [])
  }

  async owner(): Promise<string> {
    return this.call("owner", [])
  }

  async paused(): Promise<boolean> {
    return this.call("paused", [])
  }

  async protocolFeeShareBPS(): Promise<ethers.BigNumber> {
    return this.call("protocolFeeShareBPS", [])
  }

  async swapStorage(): Promise<([initialA: ethers.BigNumber, futureA: ethers.BigNumber, initialATime: ethers.BigNumber, futureATime: ethers.BigNumber, swapFee: ethers.BigNumber, adminFee: ethers.BigNumber, lpToken: string] & {initialA: ethers.BigNumber, futureA: ethers.BigNumber, initialATime: ethers.BigNumber, futureATime: ethers.BigNumber, swapFee: ethers.BigNumber, adminFee: ethers.BigNumber, lpToken: string})> {
    return this.call("swapStorage", [])
  }

  private async call(name: string, args: any[]) : Promise<any> {
    const fragment = abi.getFunction(name)
    const data = abi.encodeFunctionData(fragment, args)
    const result = await this._chain.client.call('eth_call', [{to: this.address, data}, this.blockHeight])
    const decoded = abi.decodeFunctionResult(fragment, result)
    return decoded.length > 1 ? decoded : decoded[0]
  }
}

function getJsonAbi(): any {
  return [
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": true,
          "internalType": "address",
          "name": "provider",
          "type": "address"
        },
        {
          "indexed": false,
          "internalType": "uint256[]",
          "name": "tokenAmounts",
          "type": "uint256[]"
        },
        {
          "indexed": false,
          "internalType": "uint256[]",
          "name": "fees",
          "type": "uint256[]"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "invariant",
          "type": "uint256"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "lpTokenSupply",
          "type": "uint256"
        }
      ],
      "name": "AddLiquidity",
      "type": "event"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": true,
          "internalType": "address",
          "name": "receiver",
          "type": "address"
        },
        {
          "indexed": false,
          "internalType": "uint8",
          "name": "tokenIndex",
          "type": "uint8"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "amount",
          "type": "uint256"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "amountFee",
          "type": "uint256"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "protocolFee",
          "type": "uint256"
        }
      ],
      "name": "FlashLoan",
      "type": "event"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "newAdminFee",
          "type": "uint256"
        }
      ],
      "name": "NewAdminFee",
      "type": "event"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "newSwapFee",
          "type": "uint256"
        }
      ],
      "name": "NewSwapFee",
      "type": "event"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": true,
          "internalType": "address",
          "name": "previousOwner",
          "type": "address"
        },
        {
          "indexed": true,
          "internalType": "address",
          "name": "newOwner",
          "type": "address"
        }
      ],
      "name": "OwnershipTransferred",
      "type": "event"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": false,
          "internalType": "address",
          "name": "account",
          "type": "address"
        }
      ],
      "name": "Paused",
      "type": "event"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "oldA",
          "type": "uint256"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "newA",
          "type": "uint256"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "initialTime",
          "type": "uint256"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "futureTime",
          "type": "uint256"
        }
      ],
      "name": "RampA",
      "type": "event"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": true,
          "internalType": "address",
          "name": "provider",
          "type": "address"
        },
        {
          "indexed": false,
          "internalType": "uint256[]",
          "name": "tokenAmounts",
          "type": "uint256[]"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "lpTokenSupply",
          "type": "uint256"
        }
      ],
      "name": "RemoveLiquidity",
      "type": "event"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": true,
          "internalType": "address",
          "name": "provider",
          "type": "address"
        },
        {
          "indexed": false,
          "internalType": "uint256[]",
          "name": "tokenAmounts",
          "type": "uint256[]"
        },
        {
          "indexed": false,
          "internalType": "uint256[]",
          "name": "fees",
          "type": "uint256[]"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "invariant",
          "type": "uint256"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "lpTokenSupply",
          "type": "uint256"
        }
      ],
      "name": "RemoveLiquidityImbalance",
      "type": "event"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": true,
          "internalType": "address",
          "name": "provider",
          "type": "address"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "lpTokenAmount",
          "type": "uint256"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "lpTokenSupply",
          "type": "uint256"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "boughtId",
          "type": "uint256"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "tokensBought",
          "type": "uint256"
        }
      ],
      "name": "RemoveLiquidityOne",
      "type": "event"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "currentA",
          "type": "uint256"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "time",
          "type": "uint256"
        }
      ],
      "name": "StopRampA",
      "type": "event"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": true,
          "internalType": "address",
          "name": "buyer",
          "type": "address"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "tokensSold",
          "type": "uint256"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "tokensBought",
          "type": "uint256"
        },
        {
          "indexed": false,
          "internalType": "uint128",
          "name": "soldId",
          "type": "uint128"
        },
        {
          "indexed": false,
          "internalType": "uint128",
          "name": "boughtId",
          "type": "uint128"
        }
      ],
      "name": "TokenSwap",
      "type": "event"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": false,
          "internalType": "address",
          "name": "account",
          "type": "address"
        }
      ],
      "name": "Unpaused",
      "type": "event"
    },
    {
      "inputs": [],
      "name": "MAX_BPS",
      "outputs": [
        {
          "internalType": "uint256",
          "name": "",
          "type": "uint256"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "uint256[]",
          "name": "amounts",
          "type": "uint256[]"
        },
        {
          "internalType": "uint256",
          "name": "minToMint",
          "type": "uint256"
        },
        {
          "internalType": "uint256",
          "name": "deadline",
          "type": "uint256"
        }
      ],
      "name": "addLiquidity",
      "outputs": [
        {
          "internalType": "uint256",
          "name": "",
          "type": "uint256"
        }
      ],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "uint256",
          "name": "amount",
          "type": "uint256"
        }
      ],
      "name": "calculateRemoveLiquidity",
      "outputs": [
        {
          "internalType": "uint256[]",
          "name": "",
          "type": "uint256[]"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "uint256",
          "name": "tokenAmount",
          "type": "uint256"
        },
        {
          "internalType": "uint8",
          "name": "tokenIndex",
          "type": "uint8"
        }
      ],
      "name": "calculateRemoveLiquidityOneToken",
      "outputs": [
        {
          "internalType": "uint256",
          "name": "availableTokenAmount",
          "type": "uint256"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "uint8",
          "name": "tokenIndexFrom",
          "type": "uint8"
        },
        {
          "internalType": "uint8",
          "name": "tokenIndexTo",
          "type": "uint8"
        },
        {
          "internalType": "uint256",
          "name": "dx",
          "type": "uint256"
        }
      ],
      "name": "calculateSwap",
      "outputs": [
        {
          "internalType": "uint256",
          "name": "",
          "type": "uint256"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "uint256[]",
          "name": "amounts",
          "type": "uint256[]"
        },
        {
          "internalType": "bool",
          "name": "deposit",
          "type": "bool"
        }
      ],
      "name": "calculateTokenAmount",
      "outputs": [
        {
          "internalType": "uint256",
          "name": "",
          "type": "uint256"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "address",
          "name": "receiver",
          "type": "address"
        },
        {
          "internalType": "contract IERC20",
          "name": "token",
          "type": "address"
        },
        {
          "internalType": "uint256",
          "name": "amount",
          "type": "uint256"
        },
        {
          "internalType": "bytes",
          "name": "params",
          "type": "bytes"
        }
      ],
      "name": "flashLoan",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "flashLoanFeeBPS",
      "outputs": [
        {
          "internalType": "uint256",
          "name": "",
          "type": "uint256"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "getA",
      "outputs": [
        {
          "internalType": "uint256",
          "name": "",
          "type": "uint256"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "getAPrecise",
      "outputs": [
        {
          "internalType": "uint256",
          "name": "",
          "type": "uint256"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "uint256",
          "name": "index",
          "type": "uint256"
        }
      ],
      "name": "getAdminBalance",
      "outputs": [
        {
          "internalType": "uint256",
          "name": "",
          "type": "uint256"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "getAdminBalances",
      "outputs": [
        {
          "internalType": "uint256[]",
          "name": "adminBalances",
          "type": "uint256[]"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "getLpToken",
      "outputs": [
        {
          "internalType": "contract LPToken",
          "name": "",
          "type": "address"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "getNumberOfTokens",
      "outputs": [
        {
          "internalType": "uint256",
          "name": "",
          "type": "uint256"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "uint8",
          "name": "index",
          "type": "uint8"
        }
      ],
      "name": "getToken",
      "outputs": [
        {
          "internalType": "contract IERC20",
          "name": "",
          "type": "address"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "uint8",
          "name": "index",
          "type": "uint8"
        }
      ],
      "name": "getTokenBalance",
      "outputs": [
        {
          "internalType": "uint256",
          "name": "",
          "type": "uint256"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "getTokenBalances",
      "outputs": [
        {
          "internalType": "uint256[]",
          "name": "",
          "type": "uint256[]"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "address",
          "name": "tokenAddress",
          "type": "address"
        }
      ],
      "name": "getTokenIndex",
      "outputs": [
        {
          "internalType": "uint8",
          "name": "",
          "type": "uint8"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "getTokenPrecisionMultipliers",
      "outputs": [
        {
          "internalType": "uint256[]",
          "name": "",
          "type": "uint256[]"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "getTokens",
      "outputs": [
        {
          "internalType": "contract IERC20[]",
          "name": "",
          "type": "address[]"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "getVirtualPrice",
      "outputs": [
        {
          "internalType": "uint256",
          "name": "",
          "type": "uint256"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "contract IERC20[]",
          "name": "_pooledTokens",
          "type": "address[]"
        },
        {
          "internalType": "uint8[]",
          "name": "decimals",
          "type": "uint8[]"
        },
        {
          "internalType": "string",
          "name": "lpTokenName",
          "type": "string"
        },
        {
          "internalType": "string",
          "name": "lpTokenSymbol",
          "type": "string"
        },
        {
          "internalType": "uint256",
          "name": "_a",
          "type": "uint256"
        },
        {
          "internalType": "uint256",
          "name": "_fee",
          "type": "uint256"
        },
        {
          "internalType": "uint256",
          "name": "_adminFee",
          "type": "uint256"
        },
        {
          "internalType": "address",
          "name": "lpTokenTargetAddress",
          "type": "address"
        }
      ],
      "name": "initialize",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "owner",
      "outputs": [
        {
          "internalType": "address",
          "name": "",
          "type": "address"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "pause",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "paused",
      "outputs": [
        {
          "internalType": "bool",
          "name": "",
          "type": "bool"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "protocolFeeShareBPS",
      "outputs": [
        {
          "internalType": "uint256",
          "name": "",
          "type": "uint256"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "uint256",
          "name": "futureA",
          "type": "uint256"
        },
        {
          "internalType": "uint256",
          "name": "futureTime",
          "type": "uint256"
        }
      ],
      "name": "rampA",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "uint256",
          "name": "amount",
          "type": "uint256"
        },
        {
          "internalType": "uint256[]",
          "name": "minAmounts",
          "type": "uint256[]"
        },
        {
          "internalType": "uint256",
          "name": "deadline",
          "type": "uint256"
        }
      ],
      "name": "removeLiquidity",
      "outputs": [
        {
          "internalType": "uint256[]",
          "name": "",
          "type": "uint256[]"
        }
      ],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "uint256[]",
          "name": "amounts",
          "type": "uint256[]"
        },
        {
          "internalType": "uint256",
          "name": "maxBurnAmount",
          "type": "uint256"
        },
        {
          "internalType": "uint256",
          "name": "deadline",
          "type": "uint256"
        }
      ],
      "name": "removeLiquidityImbalance",
      "outputs": [
        {
          "internalType": "uint256",
          "name": "",
          "type": "uint256"
        }
      ],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "uint256",
          "name": "tokenAmount",
          "type": "uint256"
        },
        {
          "internalType": "uint8",
          "name": "tokenIndex",
          "type": "uint8"
        },
        {
          "internalType": "uint256",
          "name": "minAmount",
          "type": "uint256"
        },
        {
          "internalType": "uint256",
          "name": "deadline",
          "type": "uint256"
        }
      ],
      "name": "removeLiquidityOneToken",
      "outputs": [
        {
          "internalType": "uint256",
          "name": "",
          "type": "uint256"
        }
      ],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "renounceOwnership",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "uint256",
          "name": "newAdminFee",
          "type": "uint256"
        }
      ],
      "name": "setAdminFee",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "uint256",
          "name": "newFlashLoanFeeBPS",
          "type": "uint256"
        },
        {
          "internalType": "uint256",
          "name": "newProtocolFeeShareBPS",
          "type": "uint256"
        }
      ],
      "name": "setFlashLoanFees",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "uint256",
          "name": "newSwapFee",
          "type": "uint256"
        }
      ],
      "name": "setSwapFee",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "stopRampA",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "uint8",
          "name": "tokenIndexFrom",
          "type": "uint8"
        },
        {
          "internalType": "uint8",
          "name": "tokenIndexTo",
          "type": "uint8"
        },
        {
          "internalType": "uint256",
          "name": "dx",
          "type": "uint256"
        },
        {
          "internalType": "uint256",
          "name": "minDy",
          "type": "uint256"
        },
        {
          "internalType": "uint256",
          "name": "deadline",
          "type": "uint256"
        }
      ],
      "name": "swap",
      "outputs": [
        {
          "internalType": "uint256",
          "name": "",
          "type": "uint256"
        }
      ],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "swapStorage",
      "outputs": [
        {
          "internalType": "uint256",
          "name": "initialA",
          "type": "uint256"
        },
        {
          "internalType": "uint256",
          "name": "futureA",
          "type": "uint256"
        },
        {
          "internalType": "uint256",
          "name": "initialATime",
          "type": "uint256"
        },
        {
          "internalType": "uint256",
          "name": "futureATime",
          "type": "uint256"
        },
        {
          "internalType": "uint256",
          "name": "swapFee",
          "type": "uint256"
        },
        {
          "internalType": "uint256",
          "name": "adminFee",
          "type": "uint256"
        },
        {
          "internalType": "contract LPToken",
          "name": "lpToken",
          "type": "address"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "address",
          "name": "newOwner",
          "type": "address"
        }
      ],
      "name": "transferOwnership",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "unpause",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "withdrawAdminFees",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    }
  ]
}
