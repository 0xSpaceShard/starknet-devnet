---
sidebar_position: 3
---

# JSON-RPC API

Devnet also partially supports JSON-RPC API v0.1.0: [**specifications**](https://github.com/starkware-libs/starknet-specs/releases/tag/v0.1.0) . It can be reached under `/rpc`. For an example:

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

Methods currently not supported:

- `starknet_protocolVersion` - will be removed in a future version of the specification
- `starknet_getNonce`
