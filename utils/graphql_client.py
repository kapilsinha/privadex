import json
import requests

url = 'https://squid.subsquid.io/<endpoint>/graphql'
query = '''
{
  pairs(orderBy: reserveUSD_DESC, where: {reserveUSD_gt: "20000"}) {
    id
    reserve1
    reserve0
    token0Id
    token1Id
  }
# Replace id_in list with the unique tokens from above
  tokens(
    where: {id_in: [
      "0xffffffff1fcacbd218edc0eba20fc2308c778080"
      "0xacc15dc74880c9944775448304b263d191c6077f"
    ]}
  ) {
    symbol
    id
    derivedETH
  }
  bundleById(id: "1") {
    ethPrice
  }
}
'''

headers = {
   'content-type': 'application/json',
}
body = {'query': query}

x = requests.post(url, headers=headers, data=json.dumps(body))
print(x.json())

