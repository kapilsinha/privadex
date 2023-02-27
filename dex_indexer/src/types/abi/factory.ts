import * as ethers from "ethers";
import assert from "assert";

export const abi = new ethers.utils.Interface(getJsonAbi());

export interface PairCreated0Event {
  token0: string;
  token1: string;
  pair: string;
  param: ethers.BigNumber;
}

export interface EvmEvent {
  data: string;
  topics: string[];
}

export const events = {
  "PairCreated(address,address,address,uint256)":  {
    topic: abi.getEventTopic("PairCreated(address,address,address,uint256)"),
    decode(data: EvmEvent): PairCreated0Event {
      const result = abi.decodeEventLog(
        abi.getEvent("PairCreated(address,address,address,uint256)"),
        data.data || "",
        data.topics
      );
      return  {
        token0: result[0],
        token1: result[1],
        pair: result[2],
        param: result[3],
      }
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

  private async call(name: string, args: any[]) : Promise<ReadonlyArray<any>> {
    const fragment = abi.getFunction(name)
    const data = abi.encodeFunctionData(fragment, args)
    const result = await this._chain.client.call('eth_call', [{to: this.address, data}, this.blockHeight])
    return abi.decodeFunctionResult(fragment, result)
  }

  async INIT_CODE_PAIR_HASH(): Promise<string> {
    const result = await this.call("INIT_CODE_PAIR_HASH", [])
    return result[0]
  }

  async allPairs(param: ethers.BigNumber): Promise<string> {
    const result = await this.call("allPairs", [param])
    return result[0]
  }

  async allPairsLength(): Promise<ethers.BigNumber> {
    const result = await this.call("allPairsLength", [])
    return result[0]
  }

  async feeTo(): Promise<string> {
    const result = await this.call("feeTo", [])
    return result[0]
  }

  async feeToSetter(): Promise<string> {
    const result = await this.call("feeToSetter", [])
    return result[0]
  }

  async getPair(addressA: string, addressB: string): Promise<string> {
    const result = await this.call("getPair", [addressA, addressB])
    return result[0]
  }
}

function getJsonAbi(): any {
  return [
    {
      "inputs": [
        {
          "internalType": "address",
          "name": "_feeToSetter",
          "type": "address"
        }
      ],
      "payable": false,
      "stateMutability": "nonpayable",
      "type": "constructor"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": true,
          "internalType": "address",
          "name": "token0",
          "type": "address"
        },
        {
          "indexed": true,
          "internalType": "address",
          "name": "token1",
          "type": "address"
        },
        {
          "indexed": false,
          "internalType": "address",
          "name": "pair",
          "type": "address"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "param",
          "type": "uint256"
        }
      ],
      "name": "PairCreated",
      "type": "event"
    },
    {
      "constant": true,
      "inputs": [],
      "name": "INIT_CODE_PAIR_HASH",
      "outputs": [
        {
          "internalType": "bytes32",
          "name": "",
          "type": "bytes32"
        }
      ],
      "payable": false,
      "stateMutability": "view",
      "type": "function"
    },
    {
      "constant": true,
      "inputs": [
        {
          "internalType": "uint256",
          "name": "param",
          "type": "uint256"
        }
      ],
      "name": "allPairs",
      "outputs": [
        {
          "internalType": "address",
          "name": "",
          "type": "address"
        }
      ],
      "payable": false,
      "stateMutability": "view",
      "type": "function"
    },
    {
      "constant": true,
      "inputs": [],
      "name": "allPairsLength",
      "outputs": [
        {
          "internalType": "uint256",
          "name": "",
          "type": "uint256"
        }
      ],
      "payable": false,
      "stateMutability": "view",
      "type": "function"
    },
    {
      "constant": false,
      "inputs": [
        {
          "internalType": "address",
          "name": "tokenA",
          "type": "address"
        },
        {
          "internalType": "address",
          "name": "tokenB",
          "type": "address"
        }
      ],
      "name": "createPair",
      "outputs": [
        {
          "internalType": "address",
          "name": "pair",
          "type": "address"
        }
      ],
      "payable": false,
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "constant": true,
      "inputs": [],
      "name": "feeTo",
      "outputs": [
        {
          "internalType": "address",
          "name": "",
          "type": "address"
        }
      ],
      "payable": false,
      "stateMutability": "view",
      "type": "function"
    },
    {
      "constant": true,
      "inputs": [],
      "name": "feeToSetter",
      "outputs": [
        {
          "internalType": "address",
          "name": "",
          "type": "address"
        }
      ],
      "payable": false,
      "stateMutability": "view",
      "type": "function"
    },
    {
      "constant": true,
      "inputs": [
        {
          "internalType": "address",
          "name": "addressA",
          "type": "address"
        },
        {
          "internalType": "address",
          "name": "addressB",
          "type": "address"
        }
      ],
      "name": "getPair",
      "outputs": [
        {
          "internalType": "address",
          "name": "",
          "type": "address"
        }
      ],
      "payable": false,
      "stateMutability": "view",
      "type": "function"
    },
    {
      "constant": false,
      "inputs": [
        {
          "internalType": "address",
          "name": "_feeTo",
          "type": "address"
        }
      ],
      "name": "setFeeTo",
      "outputs": [],
      "payable": false,
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "constant": false,
      "inputs": [
        {
          "internalType": "address",
          "name": "_feeToSetter",
          "type": "address"
        }
      ],
      "name": "setFeeToSetter",
      "outputs": [],
      "payable": false,
      "stateMutability": "nonpayable",
      "type": "function"
    }
  ]
}
