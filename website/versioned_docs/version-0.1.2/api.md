# API

## JSON-RPC API

Both Starknet's and Devnet's JSON-RPC API are reachable at `/rpc` and `/`. E.g. if spawning Devnet with default settings, these URLs are functionally equivalent: `http://127.0.0.1:5050/rpc` and `http://127.0.0.1:5050/`. The difference between these two groups of methods is their prefix: `starknet_` (e.g. `starknet_getNonce`) and `devnet_` (e.g. `devnet_mint`).

### Starknet API

Unlike Pythonic Devnet, which also supported Starknet's gateway and feeder gateway API, Devnet in Rust supports [Starknet's JSON-RPC API](https://github.com/starkware-libs/starknet-specs/tree/master/api). Since JSON-RPC v0.6.0, to find out which JSON-RPC version is supported by which Devnet version, check out the [releases page](https://github.com/0xSpaceShard/starknet-devnet/releases).

### Devnet API

Devnet has many additional features which are available via their own endpoints and JSON-RPC, which are all mentioned throughout the documentation.

## Interacting with Devnet in JavaScript and TypeScript

To interact with Devnet using the [Devnet API](#devnet-api), you can use [`starknet-devnet-js`](https://github.com/0xSpaceShard/starknet-devnet-js/). This can be especially useful in achieving [L1-L2 communication](./postman.md).

To interact with Devnet usings the [Starknet API](#starknet-api), use [starknet.js](https://www.starknetjs.com/).

## Config API

To retrieve the current configuration of Devnet, send a `GET` request to `/config` or `JSON-RPC` request with method name `devnet_getConfig`. Example response is attached below. It can be interpreted as a JSON mapping of CLI input parameters, both specified and default ones, with some irrelevant parameters omitted. So use `starknet-devnet --help` to better understand the meaning of each value, though keep in mind that some of the parameters have slightly modified names.

```json
{
  "seed": 4063802897,
  "total_accounts": 10,
  "account_contract_class_hash": "0x61dac032f228abef9c6626f995015233097ae253a7f72d68552db02f2971b8f",
  "predeployed_accounts_initial_balance": "1000000000000000000000",
  "start_time": null,
  "gas_price_wei": 100000000000,
  "gas_price_strk": 100000000000,
  "data_gas_price_wei": 100000000000,
  "data_gas_price_strk": 100000000000,
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
    "request_body_size_limit": 2000000
  },
  "block_generation": null,
  "lite_mode": false
}
```
