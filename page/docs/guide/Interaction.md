---
sidebar_position: 2
---

# Interaction

- Interact with Devnet as you would with the official Starknet [**Alpha testnet**](https://www.cairo-lang.org/docs/hello_starknet/amm.html?highlight=alpha#interaction-examples).
- The exact underlying API is not exposed for the same reason Alpha testnet does not expose it.
- To use Devnet with Starknet CLI, provide Devnet's URL to the `--gateway_url` and `--feeder_gateway_url` options of Starknet CLI commands.
- The following Starknet CLI commands are supported:
  - `call`
  - `declare`
  - `deploy`
  - `estimate_fee`
  - `get_block` (currently pending block is not supported)
  - `get_block_traces`
  - `get_class_by_hash`
  - `get_class_hash_at`
  - `get_code`
  - `get_full_contract`
  - `get_state_update`
  - `get_storage_at`
  - `get_transaction`
  - `get_transaction_receipt`
  - `get_transaction_trace`
  - `invoke`
  - `tx_status`
- The following Starknet CLI commands are **not** supported:
  - `get_contract_addresses`