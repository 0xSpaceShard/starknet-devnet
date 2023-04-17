---
sidebar_position: 3
---

# JSON-RPC API

Devnet also supports JSON-RPC API v0.2.1: [specifications](https://github.com/starkware-libs/starknet-specs/releases/tag/v0.2.1) . It can be reached under `/rpc`. For an example:

Requests to rpc and devnet responses are automatically validated against JSON schema in runtime.
In case of problems, this validations can be disabled by `--disable-rpc-request-validation` and
`--disable-rpc-response-validation` run flags. If you encounter issues with validation, please [report it on github](https://github.com/0xSpaceShard/starknet-devnet/issues).

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

## starknet_getEvents

**Disclaimer!** JSON-RPC specifications are not completely in sync with those of gateway. While `starknet_getEvents` is supported for the pending block, the official schema does not allow the block hash and the block number in the response to be empty or anything other than a number. Since these values are undefined for the pending block and since they must be set to something, we decided to go with the compromise of setting them to zero-values.
