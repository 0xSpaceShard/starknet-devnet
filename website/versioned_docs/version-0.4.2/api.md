---
sidebar_position: 3
---

# API

## JSON-RPC API

Both Starknet's and Devnet's JSON-RPC API are reachable at `/rpc` and `/`. E.g. if spawning Devnet with default settings, these URLs are functionally equivalent: `http://127.0.0.1:5050/rpc` and `http://127.0.0.1:5050/`. The difference between these two groups of methods is their prefix: `starknet_` (e.g. `starknet_getNonce`) and `devnet_` (e.g. `devnet_mint`).

### Starknet API

Unlike Pythonic Devnet, which also supported Starknet's gateway and feeder gateway API, Devnet in Rust supports [Starknet's JSON-RPC API](https://github.com/starkware-libs/starknet-specs/tree/master/api), including [WebSocket support](#websocket).

Due to how Devnet internally works, the method `starknet_getStorageProof` is not applicable, and thus not supported.

Since JSON-RPC v0.6.0, to find out which JSON-RPC version is supported by which Devnet version, check out the [releases page](https://github.com/0xspaceshard/starknet-devnet/releases).

### Devnet API

Devnet has many additional features which are available via their own endpoints and JSON-RPC. The RPC methods are documented throughout the documentation in their corresponding pages, but are also aggregated [here](https://github.com/0xSpaceShard/starknet-devnet/blob/main/website/static/devnet_api.json).

:::warning Deprecation notice

New features are only supported as part of the JSON-RPC API. Older non-RPC requests are still supported, but considered deprecated - they will be removed in the future, except the [healthcheck endpoint](#healthcheck).

:::

#### Healthcheck

To check if a Devnet instance is alive, send an HTTP request `GET /is_alive`. If alive, Devnet will reply with a `200 OK` and an appropriate message.

### WebSocket

:::note Scope

Only Starknet's WebSocket methods are available via this protocol. General JSON-RPC support via WebSocket is not yet a feature of Devnet, meaning it does not support methods like `devnet_mint` or `starknet_getNonce`.

:::

JSON-RPC [WebSocket methods](https://github.com/starkware-libs/starknet-specs/blob/v0.8.0/api/starknet_ws_api.json) can be accessed via the WebSocket protocol, using text or binary messages. Devnet listens for new WebSocket connections at `ws://<HOST>:<PORT>/ws` (notice the protocol scheme). E.g. using [`wscat`](https://www.npmjs.com/package/wscat) on the same computer where Devnet is spawned at default host and port:

```
$ wscat -c ws://127.0.0.1:5050/ws
Connected (press CTRL+C to quit)
> { "jsonrpc": "2.0", "id": 0, "method": "starknet_subscribeNewHeads" }
< {"id":0,"result":2935616350010920547,"jsonrpc":"2.0"}
```

## Interacting with Devnet in JavaScript and TypeScript

To spawn Devnet and interact with it using the [Devnet API](#devnet-api), you can use [`starknet-devnet-js`](https://github.com/0xSpaceShard/starknet-devnet-js/). This can be especially useful in achieving [L1-L2 communication](./postman.md).

To interact with Devnet using the [Starknet API](#starknet-api), use [starknet.js](https://www.starknetjs.com/).

## Config API

To retrieve the current configuration of Devnet, as specified via [CLI](running/cli.md) and later requests, send a `GET` request to `/config` or `JSON-RPC` request with method name `devnet_getConfig`. Example response is attached below. It can be interpreted as a JSON mapping of CLI input parameters, both specified and default ones, with some irrelevant parameters omitted. So use `starknet-devnet --help` to better understand the meaning of each value, though keep in mind that some of the parameters have slightly modified names. The exact values may have changed and should not be referenced.

```json
{
  "seed": 4063802897,
  "total_accounts": 10,
  "account_contract_class_hash": "0x61dac032f228abef9c6626f995015233097ae253a7f72d68552db02f2971b8f",
  "predeployed_accounts_initial_balance": "1000000000000000000000",
  "start_time": null,
  "gas_price_wei": 100000000000,
  "gas_price_fri": 100000000000,
  "data_gas_price_wei": 100000000000,
  "data_gas_price_fri": 100000000000,
  "l2_gas_price_wei": 100000000000,
  "l2_gas_price_fri": 100000000000,
  "chain_id": "SN_SEPOLIA",
  "dump_on": "exit",
  "dump_path": "dump_path.json",
  "state_archive": "none",
  "fork_config": {
    "url": "http://rpc.pathfinder.equilibrium.co/integration-sepolia/rpc/v0_7",
    "block_number": 26429
  },
  "server_config": {
    "host": "127.0.0.1",
    "port": 5050,
    "timeout": 120,
    "restricted_methods": null
  },
  "block_generation": null,
  "lite_mode": false,
  "eth_erc20_class_hash": "0x046ded64ae2dead6448e247234bab192a9c483644395b66f2155f2614e5804b0",
  "strk_erc20_class_hash": "0x046ded64ae2dead6448e247234bab192a9c483644395b66f2155f2614e5804b0"
}
```
