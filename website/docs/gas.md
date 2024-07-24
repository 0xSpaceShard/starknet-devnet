# Gas price modification

The `devnet_setGasPrice` RPC method allows users to modify the current gas prices on a running Devnet. This feature is particularly useful for testing purposes and for adjustments needed after forking to align with the forked network's gas prices. All parameters are optional, allowing you to choose which ones you want to set. A boolean flag `generate_block` indicates whether a new block should be generated immediately after set of the gas prices.

## Explanation
The modified gas prices take effect starting with the next block that is generated.

`generate_block`:
- When set to `true`, a new block will be generated immediately after the gas prices are set. This ensures that the changes take effect right away and are reflected in the devnet state without waiting for the next block generation.
- When set to `false` (or omitted), the gas prices will be set, but the changes will not be immediately committed to the devnet state until the next block is generated through the usual block generation process.

## JSON-RPC Request

The following JSON-RPC request can be used to set gas prices:

```
JSON-RPC
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "setGasPrice"
    "params": {
        "gas_price_wei": 1000000,
        "data_gas_price_wei": 10000,
        "gas_price_strk": 10000,
        "data_gas_price_strk": 10000,
        "generate_block": false,
    }
}
```

## Response

The expected response from the server will mirror the request parameters, confirming the modification of gas prices:

```
{
    "gas_price_wei": 1000000,
    "data_gas_price_wei": 10000,
    "gas_price_strk": 10000,
    "data_gas_price_strk": 10000,
}
```
