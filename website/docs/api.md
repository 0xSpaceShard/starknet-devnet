# API

## Starknet API

Unlike Pythonic Devnet, which also supported Starknet's gateway and feeder gateway API, Devnet in Rust supports Starknet's JSON-RPC API. Since JSON-RPC v0.6.0, to find out which JSON-RPC version is supported by which Devnet version, check out the [releases page](https://github.com/0xspaceshard/starknet-devnet-rs/releases).

The JSON-RPC API is reachable via `/rpc` and `/` (e.g. if spawning Devnet with default settings, these URLs are functionally equivalent: `http://127.0.0.1:5050/rpc` and `http://127.0.0.1:5050/`)

## Devnet API

Devnet has many other functional features which are available via their own endpoints, which are all mentioned throughout the documentation.
