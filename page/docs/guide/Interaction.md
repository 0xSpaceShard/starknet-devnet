---
sidebar_position: 2
---

# Interaction

- Interact with Devnet as you would with the official StarkNet [**Alpha testnet**](https://www.cairo-lang.org/docs/hello_starknet/amm.html?highlight=alpha#interaction-examples).
- The exact underlying API is not exposed for the same reason Alpha testnet does not expose it.
- To use Devnet with StarkNet CLI, provide Devnet's URL to the `--gateway_url` and `--feeder_gateway_url` options of StarkNet CLI commands.
- The following StarkNet gateway endpoints are supported (mostly corresponding to StarkNet CLI commands):
  - `call`
  - `declare`
  - `deploy`
  - `deploy_account`
  - `estimate_fee`
  - `estimate_fee_bulk`
  - `get_block` (currently pending block is not supported)
  - `get_block_traces`
  - `get_class_by_hash`
  - `get_class_hash_at`
  - `get_code`
  - `get_full_contract`
  - `get_nonce`
  - `get_state_update`
  - `get_storage_at`
  - `get_transaction`
  - `get_transaction_receipt`
  - `get_transaction_trace`
  - `invoke`
  - `tx_status`
- The following StarkNet CLI commands are **not** supported:
  - `get_contract_addresses`
