# Gas fees update

The gas fees update endpoint allows users to modify the current gas prices on a running Devnet. This feature is particularly useful for testing purposes and for adjustments needed after forking to align with the forked network's gas prices. All parameters are optional, allowing you to choose which ones you want to update.

## JSON-RPC Request

The following JSON-RPC request can be used to update gas prices:

```
JSON-RPC
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "devnet_updateGas"
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

The expected response from the server will mirror the request parameters, confirming the updated gas prices:

```
{
    "gas_price_wei": 1000000,
    "data_gas_price_wei": 10000,
    "gas_price_strk": 10000,
    "data_gas_price_strk": 10000,
}
```
