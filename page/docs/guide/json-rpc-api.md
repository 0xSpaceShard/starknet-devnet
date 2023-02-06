---
sidebar_position: 3
---

# JSON-RPC API

Devnet also supports JSON-RPC API v0.2.1: [specifications](https://github.com/starkware-libs/starknet-specs/releases/tag/v0.2.1) . It can be reached under `/rpc`. For an example:

Requests to rpc and devnet responses are automatically validated against JSON schema in runtime.
In case of problems, this validations can be disabled by `--disable-rpc-request-validation` and
`--disable-rpc-response-validation` run flags. If you encounter issues with validation, please [report it on github](https://github.com/Shard-Labs/starknet-devnet/issues).

```
POST /rpc
{
  "jsonrpc": "2.0",
  "method": "starknet_getBlockTransactionCount",
  "params": {
    "block_id": "latest"
  },
  "id": 0
}
```

Response:

```
{
  "id": 0,
  "jsonrpc": "2.0",
  "result": 1
}
```

Methods that require a `block_id` only support ids of the `latest` or `pending` block.
Please note however, that the `pending` block will be the same block as the `latest`.

```js
// Use latest
{
  "block_id": "latest"
}

// or pending
{
  "block_id": "pending"
}

// or block number
{
  "block_id": {
    "block_number": 1234  // Must be the number of the latest block
  }
}

// or block hash
{
  "block_id": {
    "block_hash": "0x1234" // Must be hash of the latest block
  }
}
```